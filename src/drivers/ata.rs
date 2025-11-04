//! ATA/SATA 저장장치 드라이버
//!
//! 이 모듈은 ATA/SATA 하드 디스크 및 SSD를 위한 드라이버를 제공합니다.
//! 현재는 PIO (Programmed I/O) 모드만 지원하며, 향후 DMA 모드를 추가할 예정입니다.

use spin::Mutex;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

/// 블록 크기 (섹터 크기)
pub const SECTOR_SIZE: usize = 512;

// ATA 포트 정의 (Primary Bus)
const ATA_PRIMARY_DATA: u16 = 0x1F0;
const ATA_PRIMARY_ERROR: u16 = 0x1F1;
const ATA_PRIMARY_SECTOR_COUNT: u16 = 0x1F2;
const ATA_PRIMARY_LBA_LOW: u16 = 0x1F3;
const ATA_PRIMARY_LBA_MID: u16 = 0x1F4;
const ATA_PRIMARY_LBA_HIGH: u16 = 0x1F5;
const ATA_PRIMARY_DRIVE: u16 = 0x1F6;
const ATA_PRIMARY_STATUS: u16 = 0x1F7;
const ATA_PRIMARY_COMMAND: u16 = 0x1F7;

// ATA 포트 정의 (Secondary Bus)
const ATA_SECONDARY_DATA: u16 = 0x170;
const ATA_SECONDARY_ERROR: u16 = 0x171;
const ATA_SECONDARY_SECTOR_COUNT: u16 = 0x172;
const ATA_SECONDARY_LBA_LOW: u16 = 0x173;
const ATA_SECONDARY_LBA_MID: u16 = 0x174;
const ATA_SECONDARY_LBA_HIGH: u16 = 0x175;
const ATA_SECONDARY_DRIVE: u16 = 0x176;
const ATA_SECONDARY_STATUS: u16 = 0x177;
const ATA_SECONDARY_COMMAND: u16 = 0x177;

// ATA Control Registers
const ATA_PRIMARY_CONTROL: u16 = 0x3F6;
const ATA_SECONDARY_CONTROL: u16 = 0x376;

// ATA 명령어
const ATA_CMD_READ_PIO: u8 = 0x20;
const ATA_CMD_READ_PIO_EXT: u8 = 0x24;
const ATA_CMD_WRITE_PIO: u8 = 0x30;
const ATA_CMD_WRITE_PIO_EXT: u8 = 0x34;
const ATA_CMD_CACHE_FLUSH: u8 = 0xE7;
const ATA_CMD_STANDBY: u8 = 0xE2;
const ATA_CMD_SLEEP: u8 = 0xE6;
const ATA_CMD_IDENTIFY: u8 = 0xEC;

// ATA 상태 비트
const ATA_SR_BSY: u8 = 0x80;   // Busy
const ATA_SR_DRDY: u8 = 0x40;  // Drive ready
const ATA_SR_DF: u8 = 0x20;    // Drive write fault
const ATA_SR_DSC: u8 = 0x10;   // Drive seek complete
const ATA_SR_DRQ: u8 = 0x08;   // Data request ready
const ATA_SR_CORR: u8 = 0x04;  // Corrected data
const ATA_SR_IDX: u8 = 0x02;   // Index
const ATA_SR_ERR: u8 = 0x01;   // Error

// 타임아웃 설정 (루프 카운트)
const ATA_TIMEOUT: u32 = 100000;

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

/// ATA 버스 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaBus {
    Primary,
    Secondary,
}

/// ATA 드라이브 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaDrive {
    Master,
    Slave,
}

/// ATA 드라이버 상태
pub struct AtaDriver {
    bus: AtaBus,
    drive: AtaDrive,
    data_port: Port<u16>,
    error_port: PortReadOnly<u8>,
    sector_count_port: Port<u8>,
    lba_low_port: Port<u8>,
    lba_mid_port: Port<u8>,
    lba_high_port: Port<u8>,
    drive_port: Port<u8>,
    status_port: PortReadOnly<u8>,
    command_port: PortWriteOnly<u8>,
    control_port: Port<u8>,
    initialized: bool,
    num_sectors: u64,
    standby: bool,
}

impl AtaDriver {
    /// 새 ATA 드라이버 생성
    pub fn new(bus: AtaBus, drive: AtaDrive) -> Self {
        let (data, error, sec_cnt, lba_lo, lba_mid, lba_hi, drv, status, cmd, ctrl) = match bus {
            AtaBus::Primary => (
                ATA_PRIMARY_DATA,
                ATA_PRIMARY_ERROR,
                ATA_PRIMARY_SECTOR_COUNT,
                ATA_PRIMARY_LBA_LOW,
                ATA_PRIMARY_LBA_MID,
                ATA_PRIMARY_LBA_HIGH,
                ATA_PRIMARY_DRIVE,
                ATA_PRIMARY_STATUS,
                ATA_PRIMARY_COMMAND,
                ATA_PRIMARY_CONTROL,
            ),
            AtaBus::Secondary => (
                ATA_SECONDARY_DATA,
                ATA_SECONDARY_ERROR,
                ATA_SECONDARY_SECTOR_COUNT,
                ATA_SECONDARY_LBA_LOW,
                ATA_SECONDARY_LBA_MID,
                ATA_SECONDARY_LBA_HIGH,
                ATA_SECONDARY_DRIVE,
                ATA_SECONDARY_STATUS,
                ATA_SECONDARY_COMMAND,
                ATA_SECONDARY_CONTROL,
            ),
        };

        Self {
            bus,
            drive,
            data_port: Port::new(data),
            error_port: PortReadOnly::new(error),
            sector_count_port: Port::new(sec_cnt),
            lba_low_port: Port::new(lba_lo),
            lba_mid_port: Port::new(lba_mid),
            lba_high_port: Port::new(lba_hi),
            drive_port: Port::new(drv),
            status_port: PortReadOnly::new(status),
            command_port: PortWriteOnly::new(cmd),
            control_port: Port::new(ctrl),
            initialized: false,
            num_sectors: 0,
            standby: false,
        }
    }
    
    /// ATA 드라이버 초기화
    /// 
    /// # Safety
    /// 이 함수는 부팅 과정에서 한 번만 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), BlockDeviceError> {
        // 드라이브 선택
        let drive_select = match self.drive {
            AtaDrive::Master => 0xA0,
            AtaDrive::Slave => 0xB0,
        };
        
        self.drive_port.write(drive_select);
        self.wait_400ns();
        
        // IDENTIFY 명령 전송
        self.sector_count_port.write(0);
        self.lba_low_port.write(0);
        self.lba_mid_port.write(0);
        self.lba_high_port.write(0);
        self.command_port.write(ATA_CMD_IDENTIFY);
        
        // 상태 확인
        let status = self.status_port.read();
        if status == 0 {
            // 드라이브가 없음
            return Err(BlockDeviceError::NotReady);
        }
        
        // BSY가 클리어될 때까지 대기
        if let Err(e) = self.wait_not_busy() {
            return Err(e);
        }
        
        // LBA mid와 high 확인 (ATA 디바이스인지 확인)
        let lba_mid = self.lba_mid_port.read();
        let lba_high = self.lba_high_port.read();
        
        if lba_mid != 0 || lba_high != 0 {
            // ATA가 아닌 디바이스 (ATAPI 등)
            return Err(BlockDeviceError::NotReady);
        }
        
        // DRQ 대기
        if let Err(e) = self.wait_drq() {
            return Err(e);
        }
        
        // IDENTIFY 데이터 읽기 (256 words = 512 bytes)
        let mut identify_data = [0u16; 256];
        for i in 0..256 {
            identify_data[i] = self.data_port.read();
        }
        
        // 섹터 수 읽기 (word 60-61: LBA28 섹터 수)
        let sectors_28 = (identify_data[61] as u64) << 16 | identify_data[60] as u64;
        
        // LBA48 지원 확인 (word 83, bit 10)
        let supports_lba48 = (identify_data[83] & (1 << 10)) != 0;
        
        if supports_lba48 {
            // word 100-103: LBA48 섹터 수
            self.num_sectors = (identify_data[103] as u64) << 48
                | (identify_data[102] as u64) << 32
                | (identify_data[101] as u64) << 16
                | identify_data[100] as u64;
        } else {
            self.num_sectors = sectors_28;
        }
        
        self.initialized = true;
        Ok(())
    }

    /// 유휴 대기(standby) 전환 (디스크 스핀다운 가능)
    pub unsafe fn enter_standby(&mut self) -> Result<(), BlockDeviceError> {
        if !self.initialized { return Err(BlockDeviceError::NotReady); }
        self.command_port.write(ATA_CMD_STANDBY);
        if let Err(e) = self.wait_not_busy() { return Err(e); }
        self.standby = true;
        Ok(())
    }

    /// 슬립 진입 (깊은 절전, 웨이크에 더 오래 걸림)
    pub unsafe fn enter_sleep(&mut self) -> Result<(), BlockDeviceError> {
        if !self.initialized { return Err(BlockDeviceError::NotReady); }
        self.command_port.write(ATA_CMD_SLEEP);
        // 일부 컨트롤러는 즉시 응답하지 않음. 상태 확인 생략.
        self.standby = true;
        Ok(())
    }

    /// 필요한 경우 웨이크업
    unsafe fn resume_if_needed(&mut self) -> Result<(), BlockDeviceError> {
        if self.standby {
            // 드라이브 선택 후 짧은 대기로 깨움
            let drive_select = match self.drive { AtaDrive::Master => 0xA0, AtaDrive::Slave => 0xB0 };
            self.drive_port.write(drive_select);
            self.wait_400ns();
            // 상태 안정화까지 대기
            self.wait_not_busy()?;
            self.standby = false;
        }
        Ok(())
    }
    
    /// 드라이버가 초기화되었는지 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// 400ns 대기 (ATA 스펙에 따른 지연)
    unsafe fn wait_400ns(&mut self) {
        // 상태 레지스터를 4번 읽으면 약 400ns 지연
        for _ in 0..4 {
            self.status_port.read();
        }
    }
    
    /// BSY 플래그가 클리어될 때까지 대기
    unsafe fn wait_not_busy(&mut self) -> Result<(), BlockDeviceError> {
        for _ in 0..ATA_TIMEOUT {
            let status = self.status_port.read();
            if (status & ATA_SR_BSY) == 0 {
                return Ok(());
            }
        }
        Err(BlockDeviceError::Timeout)
    }
    
    /// DRQ 플래그가 설정될 때까지 대기
    unsafe fn wait_drq(&mut self) -> Result<(), BlockDeviceError> {
        for _ in 0..ATA_TIMEOUT {
            let status = self.status_port.read();
            
            // 에러 체크
            if (status & ATA_SR_ERR) != 0 {
                return Err(BlockDeviceError::ReadError);
            }
            
            // DRQ 설정 확인
            if (status & ATA_SR_DRQ) != 0 {
                return Ok(());
            }
        }
        Err(BlockDeviceError::Timeout)
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
        
        if block >= self.num_sectors {
            return Err(BlockDeviceError::InvalidBlock);
        }
        
        unsafe {
            self.resume_if_needed()?;
            // 드라이브 선택 및 LBA 모드 설정
            let drive_select = match self.drive {
                AtaDrive::Master => 0xE0,  // LBA mode, master
                AtaDrive::Slave => 0xF0,   // LBA mode, slave
            } | ((block >> 24) & 0x0F) as u8;  // LBA bits 24-27
            
            self.drive_port.write(drive_select);
            self.wait_400ns();
            
            // 섹터 수 설정 (1 섹터)
            self.sector_count_port.write(1);
            
            // LBA 주소 설정
            self.lba_low_port.write((block & 0xFF) as u8);
            self.lba_mid_port.write(((block >> 8) & 0xFF) as u8);
            self.lba_high_port.write(((block >> 16) & 0xFF) as u8);
            
            // READ 명령 전송
            self.command_port.write(ATA_CMD_READ_PIO);
            
            // BSY 클리어 대기
            self.wait_not_busy()?;
            
            // DRQ 대기
            self.wait_drq()?;
            
            // 데이터 읽기 (256 words = 512 bytes)
            let buf_words = buf.as_mut_ptr() as *mut u16;
            for i in 0..256 {
                let word = self.data_port.read();
                *buf_words.add(i) = word;
            }
            
            Ok(SECTOR_SIZE)
        }
    }
    
    fn write_block(&mut self, block: u64, buf: &[u8]) -> Result<usize, BlockDeviceError> {
        if !self.initialized {
            return Err(BlockDeviceError::NotReady);
        }
        
        if buf.len() < SECTOR_SIZE {
            return Err(BlockDeviceError::InvalidBuffer);
        }
        
        if block >= self.num_sectors {
            return Err(BlockDeviceError::InvalidBlock);
        }
        
        unsafe {
            self.resume_if_needed()?;
            // 드라이브 선택 및 LBA 모드 설정
            let drive_select = match self.drive {
                AtaDrive::Master => 0xE0,  // LBA mode, master
                AtaDrive::Slave => 0xF0,   // LBA mode, slave
            } | ((block >> 24) & 0x0F) as u8;  // LBA bits 24-27
            
            self.drive_port.write(drive_select);
            self.wait_400ns();
            
            // 섹터 수 설정 (1 섹터)
            self.sector_count_port.write(1);
            
            // LBA 주소 설정
            self.lba_low_port.write((block & 0xFF) as u8);
            self.lba_mid_port.write(((block >> 8) & 0xFF) as u8);
            self.lba_high_port.write(((block >> 16) & 0xFF) as u8);
            
            // WRITE 명령 전송
            self.command_port.write(ATA_CMD_WRITE_PIO);
            
            // BSY 클리어 대기
            self.wait_not_busy()?;
            
            // DRQ 대기
            self.wait_drq()?;
            
            // 데이터 쓰기 (256 words = 512 bytes)
            let buf_words = buf.as_ptr() as *const u16;
            for i in 0..256 {
                let word = *buf_words.add(i);
                self.data_port.write(word);
            }
            
            // 캐시 플러시
            self.command_port.write(ATA_CMD_CACHE_FLUSH);
            self.wait_not_busy()?;
            
            Ok(SECTOR_SIZE)
        }
    }
    
    fn num_blocks(&self) -> u64 {
        self.num_sectors
    }
}

// 전역 ATA 드라이버 인스턴스 (Primary Master)
pub static PRIMARY_MASTER: Mutex<Option<AtaDriver>> = Mutex::new(None);

// 간단한 유휴 관리 상태
struct AtaPowerConfig {
    idle_timeout_ms: u64,
    last_io_ms: u64,
}

static ATA_POWER: Mutex<AtaPowerConfig> = Mutex::new(AtaPowerConfig { idle_timeout_ms: 0, last_io_ms: 0 });

/// ATA 전원관리: 유휴 타임아웃 설정 (0 = 비활성)
pub fn set_idle_timeout_ms(ms: u64) {
    let mut cfg = ATA_POWER.lock();
    cfg.idle_timeout_ms = ms;
}

/// ATA 전원관리: I/O 발생 시 호출
pub fn note_io_activity(now_ms: u64) {
    let mut cfg = ATA_POWER.lock();
    cfg.last_io_ms = now_ms;
}

/// ATA 전원관리: 현재 시간 기준으로 유휴라면 standby 진입 시도
pub fn maybe_enter_idle(now_ms: u64) {
    let timeout = { ATA_POWER.lock().idle_timeout_ms };
    if timeout == 0 { return; }
    let last = { ATA_POWER.lock().last_io_ms };
    if now_ms.saturating_sub(last) < timeout { return; }
    if let Some(mutex) = get_primary_master() {
        if let Some(driver) = mutex.lock().as_mut() {
            unsafe {
                let _ = driver.enter_standby();
            }
        }
    }
}

/// ATA 드라이버 초기화 함수
/// 
/// Primary Master 드라이브를 감지하고 초기화합니다.
/// 
/// # Safety
/// 이 함수는 부팅 시 한 번만 호출되어야 합니다.
pub unsafe fn init() {
    let mut driver = AtaDriver::new(AtaBus::Primary, AtaDrive::Master);
    
    match driver.init() {
        Ok(()) => {
            crate::log_info!("[ATA] Primary Master initialized: {} sectors", driver.num_blocks());
            *PRIMARY_MASTER.lock() = Some(driver);
        }
        Err(e) => {
            crate::log_warn!("[ATA] Primary Master not found or error: {:?}", e);
        }
    }
}

/// Primary Master 드라이버에 대한 접근 함수
pub fn get_primary_master() -> Option<&'static Mutex<Option<AtaDriver>>> {
    if PRIMARY_MASTER.lock().is_some() {
        Some(&PRIMARY_MASTER)
    } else {
        None
    }
}
