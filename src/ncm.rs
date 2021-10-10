use std::{collections::BTreeMap, fmt::{Debug, Display, Formatter, Result as FmtResult}, fs::{File as StdFile, read_dir}, path::PathBuf};
use cntx::{nca::{ContentType as CntxContentType, NCA}, util::new_shared};
use crate::{emu::cfg::{get_config, get_keyset}, fs::{DirectoryOpenMode, File, FileOpenMode, FileSystem, PartitionFileSystem, ReadOption, file_read_val}, result::*, util::{Shared, convert_io_result}};
pub mod result;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ProgramId(pub u64);

impl Display for ProgramId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:#018X}", self.0)
    }
}

impl Debug for ProgramId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:#018X}", self.0)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum StorageId {
    None,
    Host,
    GameCard,
    BuiltinSystem,
    BuiltinUser,
    SdCard,
    Any
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum ContentType {
    Meta = 0,
    Program = 1,
    Data = 2,
    Control = 3,
    HtmlDocument = 4,
    LegalInformation = 5,
    DeltaFragment = 6
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum ContentMetaType {
    Any = 0x0,
    SystemProgram = 0x1,
    SystemData = 0x2,
    SystemUpdate = 0x3,
    BootImagePackage = 0x4,
    BootImagePackageSafe = 0x5,
    Application = 0x80,
    Patch = 0x81,
    AddOnContent = 0x82,
    Delta = 0x83
}

bit_enum! {
    ContentMetaAttribute(u8) {
        None = 0,
        IncludesExFatDriver = bit!(0),
        Rebootless = bit!(1)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct Version {
    pub value: u32
}

impl Version {
    pub fn get_major(&self) -> u8 {
        read_bits!(26, 31, self.value) as u8
    }

    pub fn get_minor(&self) -> u8 {
        read_bits!(20, 25, self.value) as u8
    }

    pub fn get_micro(&self) -> u8 {
        read_bits!(16, 19, self.value) as u8
    }

    pub fn get_bugfix(&self) -> u16 {
        read_bits!(0, 15, self.value) as u16
    }
}

impl Debug for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}.{}.{}.{}", self.get_major(), self.get_minor(), self.get_micro(), self.get_bugfix())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct PackagedContentMetaHeader {
    pub program_id: ProgramId,
    pub version: Version,
    pub cnt_meta_type: ContentMetaType,
    pub reserved: u8,
    pub extended_header_size: u16,
    pub content_count: u16,
    pub content_meta_count: u16,
    pub cnt_meta_attr: ContentMetaAttribute,
    pub reserved_2: [u8; 0x3],
    pub required_download_system_version: u32,
    pub reserved_3: [u8; 0x4]
}

pub type ContentId = [u8; 0x10];

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct ContentInfo {
    pub id: ContentId,
    pub size: [u8; 0x6],
    pub cnt_type: ContentType,
    pub id_offset: u8
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct PackagedContentInfo {
    pub sha256_hash: [u8; 0x20],
    pub info: ContentInfo
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct ContentMetaInfo {
    pub program_id: ProgramId,
    pub version: Version,
    pub cnt_meta_type: ContentMetaType,
    pub cnt_meta_attr: ContentMetaAttribute,
    pub reserved: [u8; 0x2]
}

pub struct ContentEntry {
    path: String,
    program_id: ProgramId,
    cnt_type: CntxContentType
}

#[inline]
fn make_registered_path(nand_path: PathBuf) -> PathBuf {
    nand_path.join("Contents").join("registered")
}

static mut G_CONTENT_TABLE: BTreeMap<StorageId, Vec<ContentEntry>> = BTreeMap::new();

fn scan_registered_storage_contents(storage_id: StorageId, registered_path: PathBuf) -> Result<()> {
    let mut cnts: Vec<ContentEntry> = Vec::new();

    for entry in convert_io_result(read_dir(registered_path))? {
        if let Ok(dir_entry) = entry {

            let nca_reader = new_shared(convert_io_result(StdFile::open(dir_entry.path()))?);
            let nca = convert_io_result(NCA::new(nca_reader, get_keyset(), None))?;

            let cnt_entry = ContentEntry {
                path: dir_entry.path().as_path().display().to_string(),
                program_id: ProgramId(nca.header.program_id),
                cnt_type: nca.header.cnt_type
            };

            log_line!("[{:?}] Scanned content archive (NCA) {} of type {:?}", storage_id, cnt_entry.program_id, cnt_entry.cnt_type);

            cnts.push(cnt_entry);
        }
    }

    unsafe {
        G_CONTENT_TABLE.insert(storage_id, cnts);
    }

    Ok(())
}

pub fn lookup_content(storage_id: StorageId, program_id: ProgramId, cnt_type: CntxContentType) -> Result<NCA> {
    unsafe {
        if let Some(storage_cnts) = G_CONTENT_TABLE.get(&storage_id) {
            if let Some(cnt) = storage_cnts.iter().find(|f_cnt| (f_cnt.program_id == program_id) && (f_cnt.cnt_type == cnt_type)) {
                let nca_reader = new_shared(convert_io_result(StdFile::open(cnt.path.clone()))?);
                let nca = convert_io_result(NCA::new(nca_reader, get_keyset(), None))?;

                return Ok(nca);
            }
        }
    }

    result::ResultContentNotFound::make_err()
}

#[inline]
pub fn nca_pfs0_find_open_cnmt(pfs0: &Shared<PartitionFileSystem>) -> Result<Shared<dyn File>> {
    let root_dir = pfs0.get().open_directory(PathBuf::from(""), DirectoryOpenMode::ReadFiles())?;

    // The first file we find will be the CNMT, should be the only one
    let entry = root_dir.get().read(1)?[0];

    let cnmt_file_path = PathBuf::from("").join(entry.path.to_string());
    result_return_unless!(cnmt_file_path.extension().unwrap() == "cnmt", result::ResultInvalidPackageFormat);

    pfs0.get().open_file(cnmt_file_path, FileOpenMode::Read())
}

pub fn verify_system_contents() -> Result<()> {
    const SYSTEM_UPDATE_ID: ProgramId = ProgramId(0x0100000000000816);
    let mut system_update_nca = lookup_content(StorageId::BuiltinSystem, SYSTEM_UPDATE_ID, CntxContentType::Meta)?;
    let system_update_nca_pfs0 = PartitionFileSystem::from_nca(&mut system_update_nca, 0)?;
    let system_update_cnmt = nca_pfs0_find_open_cnmt(&system_update_nca_pfs0)?;

    let system_update_cnmt_header: PackagedContentMetaHeader = file_read_val(&system_update_cnmt, 0, ReadOption::None)?;
    result_return_unless!(system_update_cnmt_header.cnt_meta_type == ContentMetaType::SystemUpdate, result::ResultSystemUpdateNotFoundInPackage);
    
    for i in 0..system_update_cnmt_header.content_meta_count as usize {
        let cnt_meta_info_offset = (std::mem::size_of::<PackagedContentMetaHeader>()
                                + system_update_cnmt_header.extended_header_size as usize
                                + system_update_cnmt_header.content_count as usize * std::mem::size_of::<PackagedContentInfo>()
                                + i * std::mem::size_of::<ContentMetaInfo>()) as u64;

        let cnt_meta_info: ContentMetaInfo = file_read_val(&system_update_cnmt, cnt_meta_info_offset, ReadOption::None)?;

        // Verify the content -> find it (ensure it's present), open it's CNMT and check that the program ID and content type match
        let mut cnt_cnmt_nca = lookup_content(StorageId::BuiltinSystem, cnt_meta_info.program_id, CntxContentType::Meta)?;
        let cnt_cnmt_nca_pfs0 = PartitionFileSystem::from_nca(&mut cnt_cnmt_nca, 0)?;
        let cnt_cnmt = nca_pfs0_find_open_cnmt(&cnt_cnmt_nca_pfs0)?;

        let cnt_cnmt_header: PackagedContentMetaHeader = file_read_val(&cnt_cnmt, 0, ReadOption::None)?;
        result_return_unless!(cnt_cnmt_header.program_id == cnt_meta_info.program_id, result::ResultInvalidPackageFormat);
        result_return_unless!(cnt_cnmt_header.cnt_meta_type == cnt_meta_info.cnt_meta_type, result::ResultInvalidPackageFormat);

        log_line!("Content verified: {:?}", cnt_meta_info);
    }

    Ok(())
}

pub fn initialize() -> Result<()> {
    let nand_system_path = PathBuf::from(get_config().nand_system_path.clone());
    let nand_system_registered_path = make_registered_path(nand_system_path);
    scan_registered_storage_contents(StorageId::BuiltinSystem, nand_system_registered_path)?;
    verify_system_contents()?;

    Ok(())
}