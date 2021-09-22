use std::path::PathBuf;
use std::fs::{self, DirEntry, File as StdFile, OpenOptions};
use std::io::{Read, Result as IoResult, Seek, SeekFrom, Write};
use cntx::nca::NCA;
use cntx::pfs0::PFS0;
use crate::util;
use crate::util::{Shared, convert_io_result};
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

// Host

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
        let actual_count = std::cmp::min(count, self.entries.len());
        let mut dir_entries: Vec<DirectoryEntry> = Vec::with_capacity(actual_count);

        for i in 0..actual_count {
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
    pub fn new(base_dir: String) -> Shared<Self> {
        Shared::new(Self {
            base_dir: base_dir
        })
    }

    fn make_path(&self, path: PathBuf) -> PathBuf {
        PathBuf::from(self.base_dir.clone()).join(path)
    }
}

impl FileSystem for HostFileSystem {
    fn create_file(&mut self, path: PathBuf, size: usize, _create_option: CreateOption) -> Result<()> {
        // Note: no need for concatenation file support
        let abs_path = self.make_path(path);
        result_return_if!(abs_path.exists(), result::ResultPathAlreadyExists);

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

        let file = Shared::new(HostFile::new(std_file));
        Ok(file)
    }

    fn open_directory(&mut self, path: PathBuf, open_mode: DirectoryOpenMode) -> Result<Shared<dyn Directory>> {
        let abs_path = self.make_path(path);

        let entries = convert_io_result(convert_io_result(fs::read_dir(abs_path))?.collect::<IoResult<Vec<_>>>())?;

        let dir = Shared::new(HostDirectory::new(entries, open_mode));
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

// PFS0

pub struct PartitionFile {
    base_fs: Shared<PFS0>,
    file_idx: usize
}

impl PartitionFile {
    pub fn new(base_fs: Shared<PFS0>, file_idx: usize) -> Self {
        Self {
            base_fs: base_fs,
            file_idx: file_idx
        }
    }
}

impl File for PartitionFile {
    fn read(&mut self, offset: u64, data: &mut [u8], _option: ReadOption) -> Result<usize> {
        convert_io_result(self.base_fs.get().read_file(self.file_idx, offset as usize, data))
    }

    fn write(&mut self, _offset: u64, _data: &[u8], _option: WriteOption) -> Result<usize> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn set_size(&mut self, _size: usize) -> Result<()> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn get_size(&mut self) -> Result<usize> {
        convert_io_result(self.base_fs.get().get_file_size(self.file_idx))
    }

    fn operate_range(&mut self, _op_id: OperationId, _offset: u64, _size: usize) -> Result<RangeInfo> {
        todo!("OperateRange for PFS0 filesystem file");
    }
}

pub struct PartitionRootDirectory {
    file_info: Vec<(String, usize)>,
    mode: DirectoryOpenMode
}

impl PartitionRootDirectory {
    pub fn new(file_info: Vec<(String, usize)>, mode: DirectoryOpenMode) -> Self {
        Self {
            file_info: file_info,
            mode: mode
        }
    }
}

impl Directory for PartitionRootDirectory {
    fn read(&mut self, count: usize) -> Result<Vec<DirectoryEntry>> {
        let actual_count = std::cmp::min(count, self.file_info.len());
        let mut dir_entries: Vec<DirectoryEntry> = Vec::with_capacity(actual_count);

        if self.mode.contains(DirectoryOpenMode::ReadFiles()) {
            for i in 0..actual_count {
                let (file_name, file_size) = &self.file_info[i];
    
                let dir_entry = DirectoryEntry {
                    path: util::CString::from_string(file_name.clone())?,
                    file_attr: FileAttribute::None(),
                    pad_1: [0; 0x2],
                    entry_type: DirectoryEntryType::File,
                    pad_2: [0; 0x3],
                    file_size: match self.mode.contains(DirectoryOpenMode::NoFileSize()) {
                        true => 0,
                        false => *file_size
                    }
                };
    
                dir_entries.push(dir_entry);
            }
        }

        Ok(dir_entries)
    }

    fn get_entry_count(&mut self) -> Result<u64> {
        Ok(self.file_info.len() as u64)
    }
}

pub struct PartitionFileSystem {
    base_fs: Shared<PFS0>,
    files: Vec<String>
}

impl PartitionFileSystem {
    pub fn new(base_fs: PFS0) -> Result<Shared<Self>> {
        let files = convert_io_result(base_fs.list_files())?;

        Ok(Shared::new(Self {
            base_fs: Shared::new(base_fs),
            files: files
        }))
    }

    #[inline]
    pub fn from_nca(nca: &mut NCA, fs_idx: usize) -> Result<Shared<Self>> {
        let pfs0 = convert_io_result(nca.open_pfs0_filesystem(fs_idx))?;
        Self::new(pfs0)
    }
}

impl FileSystem for PartitionFileSystem {
    fn create_file(&mut self, _path: PathBuf, _size: usize, _create_option: CreateOption) -> Result<()> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn delete_file(&mut self, _path: PathBuf) -> Result<()> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn create_directory(&mut self, _path: PathBuf) -> Result<()> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn delete_directory(&mut self, _path: PathBuf) -> Result<()> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn delete_directory_recursively(&mut self, _path: PathBuf) -> Result<()> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn rename_file(&mut self, _old_path: PathBuf, _new_path: PathBuf) -> Result<()> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn rename_directory(&mut self, _old_path: PathBuf, _new_path: PathBuf) -> Result<()> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn get_entry_type(&mut self, path: PathBuf) -> Result<DirectoryEntryType> {
        let path_str = path.as_path().display().to_string();

        if path_str.is_empty() {
            Ok(DirectoryEntryType::Directory)
        }
        else {
            match self.files.contains(&path_str) {
                true => Ok(DirectoryEntryType::File),
                false => result::ResultPathNotFound::make_err()
            }
        }
    }

    fn open_file(&mut self, path: PathBuf, open_mode: FileOpenMode) -> Result<Shared<dyn File>> {
        result_return_if!(open_mode != FileOpenMode::Read(), result::ResultWriteNotPermitted);

        let path_str = path.as_path().display().to_string();

        if let Some(file_idx) = self.files.iter().position(|file_name| file_name.eq(&path_str)) {
            let file = Shared::new(PartitionFile::new(self.base_fs.clone(), file_idx));
            Ok(file)
        }
        else {
            result::ResultPathNotFound::make_err()
        }
    }

    fn open_directory(&mut self, path: PathBuf, open_mode: DirectoryOpenMode) -> Result<Shared<dyn Directory>> {
        // The only directory in a PFS0 is the root directory
        let path_str = path.as_path().display().to_string();
        result_return_unless!(path_str.is_empty(), result::ResultPathNotFound);

        let mut file_info: Vec<(String, usize)> = Vec::new();
        for i in 0..self.files.len() {
            let file_name = self.files[i].clone();
            let file_size = convert_io_result(self.base_fs.get().get_file_size(i))?;

            file_info.push((file_name, file_size));
        }

        let root_dir = Shared::new(PartitionRootDirectory::new(file_info, open_mode));
        Ok(root_dir)
    }

    fn commit(&mut self) -> Result<()> {
        Ok(())
    }


    fn get_free_space_size(&mut self, _path: PathBuf) -> Result<usize> {
        Ok(0)
    }

    fn get_total_space_size(&mut self, _path: PathBuf) -> Result<usize> {
        todo!("GetTotalSpaceSize for PFS0 filesystem");
    }

    fn clean_directory_recursively(&mut self, _path: PathBuf) -> Result<()> {
        result::ResultWriteNotPermitted::make_err()
    }

    fn get_file_time_stamp_raw(&mut self, _path: PathBuf) -> Result<TimeStampRaw> {
        // PFS0 files don't contain timestamp info
        result::ResultNotImplemented::make_err()
    }
}