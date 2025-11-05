//! 파일시스템 인터페이스 모듈
//!
//! 이 모듈은 가상 파일시스템(VFS) 및 실제 파일시스템 구현을 담당합니다.

pub mod vfs;
pub mod fat32;
pub mod path;
pub mod cache;
pub mod journal;
pub mod fsck;

use crate::fs::vfs::{FileSystem, FsResult};
use crate::fs::fat32::Fat32FileSystem;
use crate::drivers::ata::BlockDevice;
use alloc::boxed::Box;
use spin::Mutex;

/// 파일시스템 매니저
/// 
/// 전역 파일시스템 인스턴스를 관리합니다.
pub struct FileSystemManager {
    root_fs: Option<Box<dyn FileSystem>>,
}

impl FileSystemManager {
    /// 새 파일시스템 매니저 생성
    pub fn new() -> Self {
        Self {
            root_fs: None,
        }
    }
    
    /// 루트 파일시스템 마운트
    /// 
    /// # Arguments
    /// * `device` - 블록 디바이스
    /// 
    /// # Returns
    /// 성공 시 Ok(()), 실패 시 오류
    pub fn mount_root(&mut self, device: Box<dyn BlockDevice>) -> FsResult<()> {
        // FAT32 파일시스템 생성
        let mut fs = Fat32FileSystem::new(device)?;
        
        // 마운트
        fs.mount()?;
        
        self.root_fs = Some(Box::new(fs));
        Ok(())
    }
    
    /// 루트 파일시스템 가져오기
    pub fn get_root(&mut self) -> Option<&mut dyn FileSystem> {
        self.root_fs.as_mut().map(|fs| fs.as_mut())
    }
    
    /// 루트 파일시스템이 마운트되어 있는지 확인
    pub fn is_mounted(&self) -> bool {
        self.root_fs.as_ref()
            .map(|fs| fs.is_mounted())
            .unwrap_or(false)
    }
}

// 전역 파일시스템 매니저
// TODO: ATA 드라이버 구현 후 활성화
// pub static FS_MANAGER: Mutex<FileSystemManager> = Mutex::new(FileSystemManager::new());

