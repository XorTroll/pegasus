use unicorn::{RegisterARM64, Engine, Handle};
use unicorn::unicorn_const::{Arch, Mode, Permission};
use std::boxed::Box;
use std::ffi::c_void;
use std::path::PathBuf;
use crate::fs::{FileSystem, FileOpenMode, ReadOption};
use crate::fs::result as fs_result;
use crate::kern::proc::get_current_process;
use crate::ldr::npdm::NpdmData;
use crate::os::ThreadLocalRegion;
use crate::util::{self, Shared};
use crate::result::*;
use crate::emu::kern as emu_kern;
use crate::kern::thread::{self, get_current_thread, get_scheduler};
use crate::kern::svc;
use crate::ldr;
use crate::ldr::result as ldr_result;

pub mod result;

pub struct MemoryRegion {
    pub address: u64,
    pub data: Vec<u8>,
    pub perm: Permission
}

impl MemoryRegion {
    pub const fn empty() -> Self {
        Self {
            address: 0,
            data: Vec::new(),
            perm: Permission::NONE
        }
    }

    pub fn from(address: u64, data: Vec<u8>, perm: Permission) -> Self {
        Self {
            address: address,
            data: data,
            perm: perm
        }
    }

    pub fn start(&self) -> u64 {
        self.address
    }

    pub fn end(&self) -> u64 {
        self.address + self.len() as u64
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn contains(&self, addr: u64) -> bool {
        (self.start() <= addr) && (self.end() > addr)
    }
}

pub type UnicornHook = *mut c_void;
pub type Register = RegisterARM64;

pub struct ContextHandle(pub Handle);

impl ContextHandle {
    pub fn read_register<T>(&self, reg: Register) -> Result<T> {
        result::convert_unicorn_error(self.0.reg_read::<T>(reg as i32))
    }

    pub fn write_register<T>(&mut self, reg: Register, t: T) -> Result<()> {
        result::convert_unicorn_error(self.0.reg_write::<T>(reg as i32, t))
    }

    pub fn read_memory(&self, address: u64, data: &mut [u8]) -> Result<()> {
        result::convert_unicorn_error(self.0.mem_read(address, data))
    }

    pub fn write_memory(&mut self, address: u64, data: &[u8]) -> Result<()> {
        result::convert_unicorn_error(self.0.mem_write(address, data))
    }

    pub fn read_memory_val<T>(&self, address: u64) -> Result<T> {
        result::convert_unicorn_error(self.0.mem_read_val(address))
    }

    pub fn write_memory_val<T>(&mut self, address: u64, t: T) -> Result<()> {
        result::convert_unicorn_error(self.0.mem_write_val(address, t))
    }

    pub fn start<T, U>(&mut self, arg_x0: T, arg_x1: U, exec_start_addr: u64, exec_end_addr: u64) -> Result<()> {
        self.write_register(Register::X0, arg_x0)?;
        self.write_register(Register::X1, arg_x1)?;

        // This avoids endless loops of interrupts (intr_no 1) for some reason
        let fpv: u64 = 3 << 20;
        self.write_register(Register::CPACR_EL1, fpv)?;

        result::convert_unicorn_error(self.0.emu_start(exec_start_addr, exec_end_addr, 0, 0))
    }
}

pub type HookedInstructionHandlerFn = Box<dyn Fn(ContextHandle) -> Result<()>>;

const SVC_INSN_BASE: u32 = 0xD4000001;

pub fn on_interrupt() {
    let is_schedulable = get_current_thread().get().is_schedulable;
    if is_schedulable {
        let cur_core = get_current_thread().get().cur_core;
        // log_line!("Scheduling in core {}...", cur_core);
        get_scheduler(cur_core).schedule();
        // log_line!("Scheduled in core {}!", cur_core);
    }
}

fn unicorn_code_hook(uc_h: Handle, address: u64, _size: usize) {
    let ctx_h = ContextHandle(uc_h);
    let cur_insn: u32 = ctx_h.read_memory_val(address).unwrap();

    // Check first if the instruction is an actual SVC instruction
    // This quick calc allows us to avoid iterating the SVC handler table for every single instruction, even though it's still a quite ugly implementation (see below)
    let maybe_svc_id = ((cur_insn & !SVC_INSN_BASE) >> 5) as u8;
    let svc_insn = SVC_INSN_BASE | ((maybe_svc_id as u32) << 5);
    if svc_insn == cur_insn {
        if let Some(svc_id) = svc::SvcId::from(maybe_svc_id) {
            if let Some(svc_handler) = emu_kern::try_find_svc_handler(&svc_id) {
                let svc_enabled = get_current_process().get().npdm.aci0_kernel_capabilities.enabled_svcs.contains(&svc_id);
                if !svc_enabled {
                    // TODO: how is this handled in a real console?
                    panic!("SVC not enabled for this process: {:?}", svc_id);
                }
                
                (svc_handler)(ctx_h).unwrap();
            }
            else {
                panic!("Unimplemented SVC: {:?}", svc_id);
            }
        }
        else {
            panic!("Invalid SVC Id: {}", maybe_svc_id);
        }
    }
    
}

fn unicorn_intr_hook(_uc_h: Handle, _intr_no: u32) {
    // This hook is present since unicorn would fail if an interrupt happens and no hook is added.
    // In other CPU emulators, we would be able to get the SVC ID from here, but unicorn itself doesn't provide it.
    // Therefore, the SVCs are handled above (thanks unicorn for this awful implementation)

    // log_line!("Interrupt {}!", intr_no);

    on_interrupt();
}

fn create_memory_region(segment_file_data: Vec<u8>, address: u64, is_compressed: bool, section_size: usize, perm: Permission) -> Result<MemoryRegion> {
    let mut segment_data = match is_compressed {
        true => lz4::block::decompress(segment_file_data.as_slice(), Some(section_size as i32)).unwrap(),
        false => segment_file_data
    };

    // TODO: check hashes if flag enabled?
    
    assert_eq!(segment_data.len(), section_size);
    segment_data.resize_with(util::align_up(section_size, 0x1000), || 0);
    log_line!("Creating memory region (size {:#X}, aligned {:#X}) at address {:#X}...", section_size, segment_data.len(), address);

    Ok(MemoryRegion::from(address, segment_data, perm))
}

fn map_memory_region(uc_h: &mut Handle, region: &MemoryRegion) -> Result<()> {
    result::convert_unicorn_error(uc_h.mem_map_ptr(region.address, region.len(), region.perm, region.data.as_ptr() as *mut c_void))
}

pub struct ExecutionContext {
    uc: Engine,
    uc_code_hook: UnicornHook,
    uc_intr_hook: UnicornHook,
    pub exec_start_addr: u64,
    pub exec_end_addr: u64,
    pub stack: MemoryRegion,
    pub tlr: MemoryRegion
}

impl ExecutionContext {
    pub fn new(entry_addr: u64, base_regions: &Vec<MemoryRegion>, stack: MemoryRegion, tlr: MemoryRegion) -> Result<Self> {
        let mut uc = result::convert_unicorn_error(Engine::new(Arch::ARM64, Mode::ARM))?; 

        let uc_code_hook = result::convert_unicorn_error(uc.add_code_hook(unicorn_code_hook, 1, 0))?;
        let uc_intr_hook = result::convert_unicorn_error(uc.add_intr_hook(unicorn_intr_hook, 1, 0))?;
        // NOTE: great unicorn Rust bindings, can't even add an invalid-mem-read/write/fetch hook ;)

        let mut exec_end_addr = u64::MAX;
        for region in base_regions {
            map_memory_region(&mut uc.handle, region)?;
            if region.contains(entry_addr) {
                exec_end_addr = region.end();
            }
        }
        result_return_if!(exec_end_addr == u64::MAX, 0xBA);

        map_memory_region(&mut uc.handle, &stack)?;
        map_memory_region(&mut uc.handle, &tlr)?;

        let stack_top = stack.end();
        let tlr_addr = tlr.start();

        let mut exec_ctx = Self {
            uc: uc,
            uc_code_hook: uc_code_hook,
            uc_intr_hook: uc_intr_hook,
            exec_start_addr: entry_addr,
            exec_end_addr: exec_end_addr,
            stack: stack,
            tlr: tlr
        };

        exec_ctx.write_register(Register::SP, stack_top)?;
        exec_ctx.write_register(Register::TPIDRRO_EL0, tlr_addr)?;

        Ok(exec_ctx)
    }

    pub fn get_handle(&self) -> ContextHandle {
        ContextHandle(self.uc.handle)
    }

    pub fn read_register<T>(&mut self, reg: Register) -> Result<T> {
        let ctx_h = self.get_handle();
        ctx_h.read_register(reg)
    }

    pub fn write_register<T>(&mut self, reg: Register, t: T) -> Result<()> {
        let mut ctx_h = self.get_handle();
        ctx_h.write_register(reg, t)
    }
}

pub struct Context {
    pub regions: Vec<MemoryRegion>
}

impl Context {
    pub const fn new() -> Self {
        Self {
            regions: Vec::new()
        }
    }

    pub fn load_nso(&mut self, base_address: u64, nso_data: Vec<u8>) -> Result<u64> {
        let nso_header: ldr::NsoHeader = util::slice_read_val(&nso_data, None)?;
        result_return_unless!(nso_header.magic == ldr::NsoHeader::MAGIC, ldr_result::ResultInvalidNso);

        let text_address = base_address + nso_header.text_segment.memory_offset as u64;
        let text_file_offset = nso_header.text_segment.file_offset as usize;
        let text_file_size = nso_header.text_file_size as usize;
        let text_data = nso_data[text_file_offset..text_file_offset + text_file_size].to_vec();
        let text = create_memory_region(text_data, text_address,
            nso_header.flags.contains(ldr::NsoFlags::TextCompressed()),
            nso_header.text_segment.section_size as usize,
            Permission::READ | Permission::EXEC)?;

        let rodata_address = base_address + nso_header.rodata_segment.memory_offset as u64;
        let rodata_file_offset = nso_header.rodata_segment.file_offset as usize;
        let rodata_file_size = nso_header.rodata_file_size as usize;
        let rodata_data = nso_data[rodata_file_offset..rodata_file_offset + rodata_file_size].to_vec();
        let rodata = create_memory_region(rodata_data, rodata_address,
            nso_header.flags.contains(ldr::NsoFlags::RodataCompressed()),
            nso_header.rodata_segment.section_size as usize,
            Permission::READ)?;

        let data_address = base_address + nso_header.data_segment.memory_offset as u64;
        let data_file_offset = nso_header.data_segment.file_offset as usize;
        let data_file_size = nso_header.data_file_size as usize;
        let data_data = nso_data[data_file_offset..data_file_offset + data_file_size].to_vec();
        let data = create_memory_region(data_data, data_address,
            nso_header.flags.contains(ldr::NsoFlags::DataCompressed()),
            nso_header.data_segment.section_size as usize,
            Permission::READ | Permission::WRITE)?;

        let bss_address = data.end();
        let bss_data = vec![0; nso_header.bss_size as usize];
        let bss = create_memory_region(bss_data, bss_address,
            false,
            nso_header.bss_size as usize,
            Permission::READ | Permission::WRITE)?;
        
        let text_start_addr = text.start();
        self.regions.push(text);
        self.regions.push(rodata);
        self.regions.push(data);
        self.regions.push(bss);
        Ok(text_start_addr)
    }

    fn load_program_nso(&mut self, exefs: &Shared<dyn FileSystem>, nso_name: String, base_address: &mut u64) -> Result<u64> {
        let nso_file = exefs.get().open_file(PathBuf::from(nso_name.clone()), FileOpenMode::Read())?;

        let mut nso_data: Vec<u8> = vec![0; nso_file.get().get_size()?];
        nso_file.get().read(0, &mut nso_data, ReadOption::None)?;

        let addr = self.load_nso(*base_address, nso_data)?;
        log_line!("Loaded '{}' at {:#X}!", nso_name, *base_address);
        // TODO: this is quite a bad idea, memory regions might be bigger than this... I need to eventually implement memory support in kern
        *base_address += 0x1000000;
        Ok(addr)
    }

    pub fn load_program(&mut self, exefs: Shared<dyn FileSystem>, base_address: u64) -> Result<(u64, NpdmData)> {
        let mut cur_base_addr = base_address;
        let mut cur_start_addr: Option<u64> = None;

        if let Ok(addr) = self.load_program_nso(&exefs, String::from("rtld"), &mut cur_base_addr) {
            cur_start_addr = Some(addr);
        }

        if let Ok(addr) = self.load_program_nso(&exefs, String::from("main"), &mut cur_base_addr) {
            if cur_start_addr.is_none() {
                cur_start_addr = Some(addr);
            }
        }

        self.load_program_nso(&exefs, String::from("sdk"), &mut cur_base_addr).ok_if_r::<fs_result::ResultPathNotFound>(0)?;

        // TODO: actual max value?
        const MAX_SUBSDK_INDEX: u32 = 20;
        for i in 0..MAX_SUBSDK_INDEX {
            self.load_program_nso(&exefs, format!("subsdk{}", i), &mut cur_base_addr).ok_if_r::<fs_result::ResultPathNotFound>(0)?;
        }

        let npdm = {
            let npdm_file = exefs.get().open_file(PathBuf::from("main.npdm"), FileOpenMode::Read())?;
            let mut npdm_data: Vec<u8> = vec![0; npdm_file.get().get_size()?];
            npdm_file.get().read(0, &mut npdm_data, ReadOption::None)?;

            NpdmData::new(&npdm_data)?
        };

        match cur_start_addr {
            Some(addr) => Ok((addr, npdm)),
            None => fs_result::ResultInvalidNcaFileSystemType::make_err()
        }
    }

    pub fn create_execution_context(&self, stack_size: usize, entry_addr: u64) -> Result<ExecutionContext> {
        // TODO: set proper address
        let stack_address = self.regions.last().unwrap().end();
        let stack_data = vec![0; stack_size];
        let stack = create_memory_region(stack_data, stack_address,
            false,
            stack_size,
            Permission::READ | Permission::WRITE)?;

        // TODO: set proper address
        let tlr_address = stack.end();
        let tlr_size = std::mem::size_of::<ThreadLocalRegion>();
        let tlr_data = vec![0; tlr_size];
        let tlr = create_memory_region(tlr_data, tlr_address,
            false,
            tlr_size,
            Permission::READ | Permission::WRITE)?;

        ExecutionContext::new(entry_addr, &self.regions, stack, tlr)
    }
}

unsafe impl Send for ExecutionContext {}
unsafe impl Sync for ExecutionContext {}