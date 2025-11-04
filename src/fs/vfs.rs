//! 가상 파일시스템 (VFS) 인터페이스
//!
//! 이 모듈은 파일시스템의 추상화 인터페이스를 제공합니다.
//! 다양한 파일시스템 구현(FAT32, ext2 등)이 이 인터페이스를 구현합니다.

use alloc::vec::Vec;
use alloc::string::String;

/// 파일 모드 및 권한
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMode(u32);

impl FileMode {
    /// 읽기 권한
    pub const READ: u32 = 0o400;
    /// 쓰기 권한
    pub const WRITE: u32 = 0o200;
    /// 실행 권한
    pub const EXECUTE: u32 = 0o100;
    
    pub fn new(mode: u32) -> Self {
        Self(mode)
    }
    
    pub fn can_read(&self) -> bool {
        (self.0 & Self::READ) != 0
    }
    
    pub fn can_write(&self) -> bool {
        (self.0 & Self::WRITE) != 0
    }
    
    pub fn can_execute(&self) -> bool {
        (self.0 & Self::EXECUTE) != 0
    }
}

/// 파일 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,    // 일반 파일
    Directory,  // 디렉토리
    Symlink,    // 심볼릭 링크
    Character,  // 문자 디바이스
    Block,      // 블록 디바이스
    Fifo,       // FIFO/파이프
    Socket,     // 소켓
}

/// 파일 속성 정보
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub file_type: FileType,
    pub size: u64,
    pub mode: FileMode,
    pub created: u64,   // 생성 시간 (Unix timestamp)
    pub modified: u64,   // 수정 시간 (Unix timestamp)
    pub accessed: u64,   // 접근 시간 (Unix timestamp)
}

/// 파일 오프셋 타입
pub type Offset = i64;

/// 파일시스템 오류 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    NotFound,           // 파일/디렉토리를 찾을 수 없음
    AlreadyExists,      // 파일/디렉토리가 이미 존재함
    InvalidPath,        // 잘못된 경로
    PermissionDenied,   // 권한 없음
    NotADirectory,      // 디렉토리가 아님
    NotAFile,           // 파일이 아님
    IsDirectory,        // 디렉토리임
    IsFile,             // 파일임
    IOError,            // I/O 오류
    InvalidFilesystem,  // 잘못된 파일시스템
    OutOfSpace,         // 공간 부족
    Busy,               // 파일시스템이 사용 중
}

/// 파일시스템 결과 타입
pub type FsResult<T> = Result<T, FsError>;

/// 파일 핸들
pub trait File: Send + Sync {
    /// 파일에서 데이터 읽기
    /// 
    /// # Arguments
    /// * `buf` - 읽은 데이터를 저장할 버퍼
    /// * `offset` - 읽기 시작 오프셋 (None이면 현재 위치)
    /// 
    /// # Returns
    /// 읽은 바이트 수
    fn read(&mut self, buf: &mut [u8], offset: Option<Offset>) -> FsResult<usize>;
    
    /// 파일에 데이터 쓰기
    /// 
    /// # Arguments
    /// * `buf` - 쓸 데이터 버퍼
    /// * `offset` - 쓰기 시작 오프셋 (None이면 현재 위치)
    /// 
    /// # Returns
    /// 쓴 바이트 수
    fn write(&mut self, buf: &[u8], offset: Option<Offset>) -> FsResult<usize>;
    
    /// 파일 메타데이터 가져오기
    fn metadata(&self) -> FsResult<FileMetadata>;
    
    /// 파일 크기 가져오기
    fn size(&self) -> FsResult<u64>;
    
    /// 현재 읽기/쓰기 위치 설정
    fn seek(&mut self, offset: Offset) -> FsResult<Offset>;
    
    /// 현재 읽기/쓰기 위치 가져오기
    fn tell(&self) -> FsResult<Offset>;
}

/// 디렉토리 핸들
pub trait Directory: Send + Sync {
    /// 디렉토리 항목 읽기
    /// 
    /// # Returns
    /// (이름, 파일 타입) 쌍의 벡터
    fn read_dir(&self) -> FsResult<Vec<(String, FileType)>>;
    
    /// 디렉토리에 새 파일 생성
    fn create_file(&mut self, name: &str) -> FsResult<()>;
    
    /// 디렉토리에 새 디렉토리 생성
    fn create_dir(&mut self, name: &str) -> FsResult<()>;
    
    /// 파일/디렉토리 삭제
    fn remove(&mut self, name: &str) -> FsResult<()>;
    
    /// 파일/디렉토리 이름 변경
    fn rename(&mut self, old_name: &str, new_name: &str) -> FsResult<()>;
}

/// 파일시스템 트레이트
pub trait FileSystem: Send + Sync {
    /// 파일시스템 마운트
    fn mount(&mut self) -> FsResult<()>;
    
    /// 파일시스템 언마운트
    fn unmount(&mut self) -> FsResult<()>;
    
    /// 파일 열기
    fn open_file(&mut self, path: &str) -> FsResult<Box<dyn File>>;
    
    /// 파일 생성 및 열기
    fn create_file(&mut self, path: &str) -> FsResult<Box<dyn File>>;
    
    /// 디렉토리 열기
    fn open_dir(&mut self, path: &str) -> FsResult<Box<dyn Directory>>;
    
    /// 디렉토리 생성
    fn create_dir(&mut self, path: &str) -> FsResult<()>;
    
    /// 파일/디렉토리 삭제
    fn remove(&mut self, path: &str) -> FsResult<()>;
    
    /// 파일/디렉토리 이름 변경
    fn rename(&mut self, old_path: &str, new_path: &str) -> FsResult<()>;
    
    /// 파일/디렉토리 메타데이터 가져오기
    fn metadata(&mut self, path: &str) -> FsResult<FileMetadata>;
    
    /// 파일시스템이 마운트되어 있는지 확인
    fn is_mounted(&self) -> bool;
}

