//! ATA/SATA 저장장치 드라이버
//!
//! 이 모듈은 ATA/SATA 하드 디스크 및 SSD를 위한 드라이버를 제공합니다.
//! 현재는 PIO (Programmed I/O) 모드만 지원하며, 향후 DMA 모드를 추가할 예정입니다.

use spin::Mutex;
use core::marker::PhantomData;

/// 블록 크기 (섹터 크기)
pub const SECTOR_SIZE: usize = 512;

/// 블록 디바이스 트레이트
/// 
/// 저장장치 드라이버는 이 트레이트를 구현해야 합니다.
pub trait BlockDevice: Send + Sync {
    /// 블록 크기를 바이트 단위로 반환
    fn block_size(&self) -> usize;
    
    /// 디바이스에서 블록 읽기
    /// 
    /// # Arguments
    /// * `block` - 읽을 블록 번호
    /// * `buf` - 데이터를 저장할 버퍼 (최소 block_size() 크기)
    /// 
    /// # Returns
    /// 성공 시 읽은 바이트 수, 실패 시 오류
    fn read_block(&mut self, block: u64, buf: &mut [u8]) -> Result<usize, BlockDeviceError>;
    
    /// 디바이스에 블록 쓰기
    /// 
    /// # Arguments
    /// * `block` - 쓸 블록 번호
    /// * `buf` - 쓸 데이터 버퍼 (최소 block_size() 크기)
    /// 
    /// # Returns
    /// 성공 시 쓴 바이트 수, 실패 시 오류
    fn write_block(&mut self, block: u64, buf: &[u8]) -> Result<usize, BlockDeviceError>;
    
    /// 디바이스의 총 블록 수
    fn num_blocks(&self) -> u64;
}

/// 블록 디바이스 오류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDeviceError {
    InvalidBlock,      // 잘못된 블록 번호
    ReadError,         // 읽기 오류
    WriteError,        // 쓰기 오류
    Timeout,           // 타임아웃
    NotReady,          // 디바이스가 준비되지 않음
    InvalidBuffer,     // 잘못된 버퍼 크기
}

/// ATA 드라이버 상태
pub struct AtaDriver {
    // TODO: ATA 컨트롤러 레지스터 및 상태
    initialized: bool,
    _phantom: PhantomData<()>,
}

impl AtaDriver {
    /// 새 ATA 드라이버 생성
    pub fn new() -> Self {
        Self {
            initialized: false,
            _phantom: PhantomData,
        }
    }
    
    /// ATA 드라이버 초기화
    /// 
    /// # Safety
    /// 이 함수는 부팅 과정에서 한 번만 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), BlockDeviceError> {
        // TODO: ATA 컨트롤러 초기화
        // 1. ATA 컨트롤러 존재 확인
        // 2. 디바이스 식별
        // 3. PIO 모드 설정
        
        self.initialized = true;
        Ok(())
    }
    
    /// 드라이버가 초기화되었는지 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl BlockDevice for AtaDriver {
    fn block_size(&self) -> usize {
        SECTOR_SIZE
    }
    
    fn read_block(&mut self, block: u64, buf: &mut [u8]) -> Result<usize, BlockDeviceError> {
        if !self.initialized {
            return Err(BlockDeviceError::NotReady);
        }
        
        if buf.len() < SECTOR_SIZE {
            return Err(BlockDeviceError::InvalidBuffer);
        }
        
        // TODO: 실제 ATA PIO 읽기 구현
        // 1. ATA 레지스터 설정
        // 2. LBA 주소 설정
        // 3. PIO 읽기 명령 실행
        // 4. 데이터 포트에서 읽기
        
        // 현재는 더미 구현
        Err(BlockDeviceError::NotReady)
    }
    
    fn write_block(&mut self, block: u64, buf: &[u8]) -> Result<usize, BlockDeviceError> {
        if !self.initialized {
            return Err(BlockDeviceError::NotReady);
        }
        
        if buf.len() < SECTOR_SIZE {
            return Err(BlockDeviceError::InvalidBuffer);
        }
        
        // TODO: 실제 ATA PIO 쓰기 구현
        // 1. ATA 레지스터 설정
        // 2. LBA 주소 설정
        // 3. 데이터 포트에 쓰기
        // 4. PIO 쓰기 명령 실행
        
        // 현재는 더미 구현
        Err(BlockDeviceError::NotReady)
    }
    
    fn num_blocks(&self) -> u64 {
        // TODO: 실제 디바이스 크기 읽기
        0
    }
}

// 전역 ATA 드라이버 인스턴스
// TODO: 실제 구현 후 활성화
// pub static ATA_DRIVER: Mutex<AtaDriver> = Mutex::new(AtaDriver::new());

