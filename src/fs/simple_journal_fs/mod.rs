//! Simple journaling filesystem (skeleton)

use crate::fs::vfs::{FileSystem, File, Directory, FsResult, FsError, FileMetadata, FileType, Offset};
use alloc::boxed::Box;

pub struct Sjfs {
    mounted: bool,
}

impl Sjfs { pub fn new() -> Self { Self { mounted: false } } }

impl FileSystem for Sjfs {
    fn mount(&mut self) -> FsResult<()> { self.mounted = true; Ok(()) }
    fn unmount(&mut self) -> FsResult<()> { self.mounted = false; Ok(()) }
    fn open_file(&mut self, _path: &str) -> FsResult<Box<dyn File>> { Err(FsError::NotFound) }
    fn create_file(&mut self, _path: &str) -> FsResult<Box<dyn File>> { Err(FsError::IOError) }
    fn open_dir(&mut self, _path: &str) -> FsResult<Box<dyn Directory>> { Err(FsError::NotFound) }
    fn create_dir(&mut self, _path: &str) -> FsResult<()> { Err(FsError::IOError) }
    fn remove(&mut self, _path: &str) -> FsResult<()> { Err(FsError::IOError) }
    fn rename(&mut self, _old: &str, _new: &str) -> FsResult<()> { Err(FsError::IOError) }
    fn metadata(&mut self, _path: &str) -> FsResult<FileMetadata> { Err(FsError::NotFound) }
    fn is_mounted(&self) -> bool { self.mounted }
}


