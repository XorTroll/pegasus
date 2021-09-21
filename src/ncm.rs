use std::{collections::BTreeMap, fs::{File, read_dir}, path::PathBuf};
use cntx::{nca::{ContentType, NCA}, util::new_shared};
use crate::{emu::cfg::{get_config, get_keyset}, result::*, util::convert_io_result};
pub mod result;

pub type ProgramId = u64;

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

pub struct ContentEntry {
    path: String,
    program_id: ProgramId,
    cnt_type: ContentType
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

            let nca_reader = new_shared(convert_io_result(File::open(dir_entry.path()))?);
            let nca = convert_io_result(NCA::new(nca_reader, get_keyset()))?;

            let cnt_entry = ContentEntry {
                path: dir_entry.path().as_path().display().to_string(),
                program_id: nca.header.program_id,
                cnt_type: nca.header.cnt_type
            };

            log_line!("{:?} Scanned content {:#018X} of type {:?}", storage_id, cnt_entry.program_id, cnt_entry.cnt_type);

            cnts.push(cnt_entry);
        }
    }

    unsafe {
        G_CONTENT_TABLE.insert(storage_id, cnts);
    }

    Ok(())
}

pub fn lookup_content(storage_id: StorageId, program_id: ProgramId, cnt_type: ContentType) -> Result<NCA> {
    unsafe {
        if let Some(storage_cnts) = G_CONTENT_TABLE.get(&storage_id) {
            if let Some(cnt) = storage_cnts.iter().find(|f_cnt| (f_cnt.program_id == program_id) && (f_cnt.cnt_type == cnt_type)) {
                let nca_reader = new_shared(convert_io_result(File::open(cnt.path.clone()))?);
                let nca = convert_io_result(NCA::new(nca_reader, get_keyset()))?;

                return Ok(nca);
            }
        }
    }

    result::ResultContentNotFound::make_err()
}

pub fn initialize() -> Result<()> {
    let nand_system_path = PathBuf::from(get_config().nand_system_path.clone());
    let nand_system_registered_path = make_registered_path(nand_system_path);
    scan_registered_storage_contents(StorageId::BuiltinSystem, nand_system_registered_path)?;

    Ok(())
}