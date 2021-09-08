use std::path::PathBuf;
use std::fs::{self, DirEntry, File as StdFile, OpenOptions};
use std::io::{ErrorKind, Read, Result as IoResult, Seek, SeekFrom, Write};
use crate::util;
use crate::util::{Shared, SharedObject, make_shared};
use crate::result::*;

pub mod result;

bit_enum! {
    CreateOption (u32) {
        ConcatenationFile = bit!(0)
    }
}

bit_enum! {
    FileOpenMode (u32) {
        Read = bit!(0),
        Write = bit!(1),
        Append = bit!(2)
    }
}

bit_enum! {
    DirectoryOpenMode (u32) {
        ReadDirectories = bit!(0),
        ReadFiles = bit!(1),
        NoFileSize = bit!(31)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum DirectoryEntryType {
    Directory = 0,
    File = 1
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct TimeStampRaw {
    pub created: u64,
    pub modified: u64,
    pub accessed: u64,
    pub is_valid: bool,
    pub pad: [u8; 0x7]
}

bit_enum! {
    FileAttribute (u8) {
        None = 0,
        IsDirectory = bit!(0),
        ArchiveBit = bit!(1)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct DirectoryEntry {
    pub path: util::CString<0x301>,
    pub file_attr: FileAttribute,
    pub pad_1: [u8; 0x2],
    pub entry_type: DirectoryEntryType,
    pub pad_2: [u8; 0x3],
    pub file_size: usize
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u32)]
pub enum ReadOption {
    None = 0
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u32)]
pub enum WriteOption {
    None = 0,
    Flush = 1
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u32)]
pub enum OperationId {
    Clear = 0,
    ClearSignature = 1,
    InvalidateCache = 2,
    QueryRange = 3
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct RangeInfo {
    pub aes_ctr_key_type: u32,
    pub speed_emulation_type: u32,
    pub reserved: [u8; 0x38]
}

pub trait File {
    fn read(&mut self, offset: u64, data: &mut [u8], option: ReadOption) -> Result<usize>;
    fn write(&mut self, offset: u64, data: &[u8], option: WriteOption) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;
    fn set_size(&mut self, size: usize) -> Result<()>;
    fn get_size(&mut self) -> Result<usize>;
    fn operate_range(&mut self, op_id: OperationId, offset: u64, size: usize) -> Result<RangeInfo>;
}

pub trait Directory {
    fn read(&mut self, count: usize) -> Result<Vec<DirectoryEntry>>;
    fn get_entry_count(&mut self) -> Result<u64>;
}

pub trait FileSystem {
    fn create_file(&mut self, path: PathBuf, size: usize, create_option: CreateOption) -> Result<()>;
    fn delete_file(&mut self, path: PathBuf) -> Result<()>;
    fn create_directory(&mut self, path: PathBuf) -> Result<()>;
    fn delete_directory(&mut self, path: PathBuf) -> Result<()>;
    fn delete_directory_recursively(&mut self, path: PathBuf) -> Result<()>;
    fn rename_file(&mut self, old_path: PathBuf, new_path: PathBuf) -> Result<()>;
    fn rename_directory(&mut self, old_path: PathBuf, new_path: PathBuf) -> Result<()>;
    fn get_entry_type(&mut self, path: PathBuf) -> Result<DirectoryEntryType>;
    fn open_file(&mut self, path: PathBuf, open_mode: FileOpenMode) -> Result<Shared<dyn File>>;
    fn open_directory(&mut self, path: PathBuf, open_mode: DirectoryOpenMode) -> Result<Shared<dyn Directory>>;
    fn commit(&mut self) -> Result<()>;
    fn get_free_space_size(&mut self, path: PathBuf) -> Result<usize>;
    fn get_total_space_size(&mut self, path: PathBuf) -> Result<usize>;
    fn clean_directory_recursively(&mut self, path: PathBuf) -> Result<()>;
    fn get_file_time_stamp_raw(&mut self, path: PathBuf) -> Result<TimeStampRaw>;
}

/// Host

fn convert_io_result<T>(r: IoResult<T>) -> Result<T> {
    r.map_err(|err| match err.kind() {
        // TODO: finish
        ErrorKind::NotFound => result::ResultPathNotFound::make(),
        ErrorKind::PermissionDenied => result::ResultTargetLocked::make(),
        ErrorKind::WouldBlock => result::ResultTargetLocked::make(),
        ErrorKind::UnexpectedEof => result::ResultOutOfRange::make(),
        _ => result::ResultNotImplemented::make()
    })
}

pub struct HostFile {
    inner_file: StdFile
}

impl HostFile {
    pub fn new(inner_file: StdFile) -> Self {
        Self {
            inner_file: inner_file
        }
    }
}

impl File for HostFile {
    fn read(&mut self, offset: u64, data: &mut [u8], _option: ReadOption) -> Result<usize> {
        convert_io_result(self.inner_file.seek(SeekFrom::Start(offset)))?;
        convert_io_result(self.inner_file.read(data))
    }

    fn write(&mut self, offset: u64, data: &[u8], option: WriteOption) -> Result<usize> {
        convert_io_result(self.inner_file.seek(SeekFrom::Start(offset)))?;
        let written = convert_io_result(self.inner_file.write(data))?;

        if option == WriteOption::Flush {
            convert_io_result(self.inner_file.flush())?;
        }

        Ok(written)
    }

    fn flush(&mut self) -> Result<()> {
        convert_io_result(self.inner_file.flush())
    }

    fn set_size(&mut self, size: usize) -> Result<()> {
        convert_io_result(self.inner_file.set_len(size as u64))
    }

    fn get_size(&mut self) -> Result<usize> {
        convert_io_result(self.inner_file.stream_len()).map(|len| len as usize)
    }

    fn operate_range(&mut self, _op_id: OperationId, _offset: u64, _size: usize) -> Result<RangeInfo> {
        todo!("OperateRange for host filesystem file");
    }
}

pub struct HostDirectory {
    entries: Vec<DirEntry>,
    open_mode: DirectoryOpenMode
}

impl HostDirectory {
    pub fn new(entries: Vec<DirEntry>, open_mode: DirectoryOpenMode) -> Self {
        Self {
            entries: entries,
            open_mode: open_mode
        }
    }
}

impl Directory for HostDirectory {
    fn read(&mut self, count: usize) -> Result<Vec<DirectoryEntry>> {
        result_return_unless!(count <= self.entries.len(), 0x9);

        let mut dir_entries: Vec<DirectoryEntry> = Vec::new();
        for i in 0..count {
            let entry = &self.entries[i];

            let entry_path = entry.path().into_os_string().into_string().unwrap();
            let entry_metadata = convert_io_result(entry.metadata())?;
            let is_dir = entry_metadata.is_dir();

            if is_dir && !self.open_mode.contains(DirectoryOpenMode::ReadDirectories()) {
                continue;
            }
            if !is_dir && !self.open_mode.contains(DirectoryOpenMode::ReadFiles()) {
                continue;
            }

            let dir_entry = DirectoryEntry {
                path: util::CString::from_string(entry_path)?,
                file_attr: match is_dir {
                    true => FileAttribute::IsDirectory(),
                    false => FileAttribute::None()
                },
                pad_1: [0; 0x2],
                entry_type: match is_dir {
                    true => DirectoryEntryType::Directory,
                    false => DirectoryEntryType::File
                },
                pad_2: [0; 0x3],
                file_size: match self.open_mode.contains(DirectoryOpenMode::NoFileSize()) {
                    true => 0,
                    false => match is_dir {
                        true => 0,
                        false => entry_metadata.len() as usize
                    }
                }
            };
            dir_entries.push(dir_entry);
        }

        Ok(dir_entries)
    }

    fn get_entry_count(&mut self) -> Result<u64> {
        Ok(self.entries.len() as u64)
    }
}

pub struct HostFileSystem {
    pub base_dir: String
}

impl HostFileSystem {
    pub fn new(base_dir: String) -> Self {
        Self {
            base_dir: base_dir
        }
    }

    fn make_path(&self, path: PathBuf) -> PathBuf {
        PathBuf::from(self.base_dir.clone()).join(path)
    }
}

impl FileSystem for HostFileSystem {
    fn create_file(&mut self, path: PathBuf, size: usize, _create_option: CreateOption) -> Result<()> {
        // Note: no need for concatenation file support

        let abs_path = self.make_path(path);
        result_return_if!(abs_path.exists(), 0x5);

        let file = convert_io_result(StdFile::open(abs_path))?;
        convert_io_result(file.set_len(size as u64))?;
        Ok(())
    }

    fn delete_file(&mut self, path: PathBuf) -> Result<()> {
        let abs_path = self.make_path(path);
        convert_io_result(fs::remove_file(abs_path))
    }

    fn create_directory(&mut self, path: PathBuf) -> Result<()> {
        let abs_path = self.make_path(path);
        convert_io_result(fs::create_dir(abs_path))
    }

    fn delete_directory(&mut self, path: PathBuf) -> Result<()> {
        let abs_path = self.make_path(path);
        convert_io_result(fs::remove_dir(abs_path))
    }

    fn delete_directory_recursively(&mut self, path: PathBuf) -> Result<()> {
        let abs_path = self.make_path(path);
        convert_io_result(fs::remove_dir_all(abs_path))
    }

    fn rename_file(&mut self, old_path: PathBuf, new_path: PathBuf) -> Result<()> {
        let abs_old_path = self.make_path(old_path);
        let abs_new_path = self.make_path(new_path);
        convert_io_result(fs::rename(abs_old_path, abs_new_path))
    }

    fn rename_directory(&mut self, old_path: PathBuf, new_path: PathBuf) -> Result<()> {
        let abs_old_path = self.make_path(old_path);
        let abs_new_path = self.make_path(new_path);
        convert_io_result(fs::rename(abs_old_path, abs_new_path))
    }

    fn get_entry_type(&mut self, path: PathBuf) -> Result<DirectoryEntryType> {
        let abs_path = self.make_path(path);
        let metadata = convert_io_result(fs::metadata(abs_path))?;

        let entry_type = match metadata.is_dir() {
            true => DirectoryEntryType::Directory,
            false => DirectoryEntryType::File
        };

        Ok(entry_type)
    }

    fn open_file(&mut self, path: PathBuf, open_mode: FileOpenMode) -> Result<Shared<dyn File>> {
        let abs_path = self.make_path(path);

        let std_file = convert_io_result(OpenOptions::new().read(open_mode.contains(FileOpenMode::Read())).write(open_mode.contains(FileOpenMode::Write())).append(open_mode.contains(FileOpenMode::Append())).open(abs_path))?;

        let file = make_shared(HostFile::new(std_file));
        Ok(file)
    }

    fn open_directory(&mut self, path: PathBuf, open_mode: DirectoryOpenMode) -> Result<Shared<dyn Directory>> {
        let abs_path = self.make_path(path);

        let entries = convert_io_result(convert_io_result(fs::read_dir(abs_path))?.collect::<IoResult<Vec<_>>>())?;

        let dir = make_shared(HostDirectory::new(entries, open_mode));
        Ok(dir)
    }

    fn commit(&mut self) -> Result<()> {
        Ok(())
    }


    fn get_free_space_size(&mut self, _path: PathBuf) -> Result<usize> {
        todo!("GetFreeSpaceSize for host filesystem");
    }

    fn get_total_space_size(&mut self, _path: PathBuf) -> Result<usize> {
        todo!("GetTotalSpaceSize for host filesystem");
    }

    fn clean_directory_recursively(&mut self, path: PathBuf) -> Result<()> {
        self.delete_directory_recursively(path.clone())?;
        self.create_directory(path)?;

        Ok(())
    }

    fn get_file_time_stamp_raw(&mut self, _path: PathBuf) -> Result<TimeStampRaw> {
        todo!("GetFileTimeStampRaw for host filesystem");
    }
}

// ---