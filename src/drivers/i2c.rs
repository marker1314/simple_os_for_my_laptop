//! I2C 버스 컨트롤러 드라이버
//!
//! AMD FCH (Fusion Controller Hub) I2C 컨트롤러를 지원합니다.

use spin::Mutex;
use x86_64::structures::paging::{PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

/// I2C 에러 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cError {
    /// 초기화되지 않음
    NotInitialized,
    /// ACPI 테이블에서 I2C 컨트롤러를 찾을 수 없음
    ControllerNotFound,
    /// 버스 타임아웃
    Timeout,
    /// NACK (Not Acknowledge) 수신
    Nack,
    /// 중재 손실
    ArbitrationLost,
    /// 잘못된 주소
    InvalidAddress,
    /// 버스 사용 중
    BusBusy,
}

/// I2C 레지스터 오프셋 (AMD FCH)
#[repr(u32)]
#[allow(dead_code)]
enum I2cReg {
    Control = 0x00,
    Status = 0x04,
    SlaveAddress = 0x08,
    Data = 0x0C,
    Control2 = 0x10,
    InterruptControl = 0x14,
}

/// I2C 컨트롤러 구조체
pub struct I2cController {
    /// MMIO 베이스 주소 (가상 주소)
    mmio_base: Option<VirtAddr>,
    /// 초기화 여부
    initialized: bool,
}

impl I2cController {
    /// 새 I2C 컨트롤러 생성
    pub const fn new() -> Self {
        Self {
            mmio_base: None,
            initialized: false,
        }
    }

    /// I2C 컨트롤러 초기화
    ///
    /// # Arguments
    /// * `physical_base` - I2C 컨트롤러의 물리 베이스 주소
    ///
    /// # Safety
    /// 메모리 매핑이 초기화된 후에 호출되어야 합니다.
    pub unsafe fn init(&mut self, physical_base: PhysAddr) -> Result<(), I2cError> {
        // MMIO 영역을 가상 메모리에 매핑
        let virt_addr = self.map_mmio(physical_base)?;
        self.mmio_base = Some(virt_addr);

        // 컨트롤러 초기화
        self.reset()?;
        self.configure()?;

        self.initialized = true;
        crate::log_info!("I2C controller initialized at physical address 0x{:X}", physical_base.as_u64());
        Ok(())
    }

    /// MMIO 영역을 가상 메모리에 매핑
    ///
    /// # Safety
    /// 유효한 물리 주소를 전달해야 합니다.
    unsafe fn map_mmio(&self, physical_base: PhysAddr) -> Result<VirtAddr, I2cError> {
        // I2C 컨트롤러의 MMIO 영역은 일반적으로 4KB
        let _phys_frame = PhysFrame::<Size4KiB>::containing_address(physical_base);
        
        // 가상 주소로 직접 매핑 (identity mapping 또는 offset mapping)
        // 실제 구현에서는 페이지 테이블을 수정해야 할 수 있음
        let virt_addr = VirtAddr::new(physical_base.as_u64());
        
        // TODO: 페이지 테이블에 매핑 추가 (현재는 간단한 변환만)
        // let page = Page::<Size4KiB>::containing_address(virt_addr);
        // map_page(page, phys_frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE)?;
        
        Ok(virt_addr)
    }

    /// 레지스터 읽기
    ///
    /// # Safety
    /// MMIO 영역에 대한 접근
    unsafe fn read_reg(&self, reg: I2cReg) -> u32 {
        if let Some(base) = self.mmio_base {
            let addr = (base.as_u64() + reg as u64) as *const u32;
            core::ptr::read_volatile(addr)
        } else {
            0
        }
    }

    /// 레지스터 쓰기
    ///
    /// # Safety
    /// MMIO 영역에 대한 접근
    unsafe fn write_reg(&self, reg: I2cReg, value: u32) {
        if let Some(base) = self.mmio_base {
            let addr = (base.as_u64() + reg as u64) as *mut u32;
            core::ptr::write_volatile(addr, value);
        }
    }

    /// 컨트롤러 리셋
    unsafe fn reset(&self) -> Result<(), I2cError> {
        // 소프트웨어 리셋
        self.write_reg(I2cReg::Control, 0);
        
        // 리셋 완료 대기
        for _ in 0..1000 {
            if self.read_reg(I2cReg::Status) & 0x01 == 0 {
                return Ok(());
            }
            x86_64::instructions::hlt();
        }
        
        Err(I2cError::Timeout)
    }

    /// 컨트롤러 설정
    unsafe fn configure(&self) -> Result<(), I2cError> {
        // I2C 표준 모드 (100 kHz) 설정
        // Control 레지스터 설정: 활성화, 마스터 모드
        self.write_reg(I2cReg::Control, 0x01); // Enable
        
        Ok(())
    }

    /// I2C 읽기
    ///
    /// # Arguments
    /// * `slave_addr` - 슬레이브 장치 주소 (7비트)
    /// * `buffer` - 읽을 데이터를 저장할 버퍼
    ///
    /// # Returns
    /// 실제로 읽은 바이트 수
    pub fn read(&self, slave_addr: u8, buffer: &mut [u8]) -> Result<usize, I2cError> {
        if !self.initialized {
            return Err(I2cError::NotInitialized);
        }

        unsafe {
            // START 조건 생성
            self.start()?;

            // 슬레이브 주소 전송 (읽기 모드: 주소 | 0x01)
            self.write_byte((slave_addr << 1) | 0x01)?;

            // 데이터 읽기
            let len = buffer.len();
            for (i, byte) in buffer.iter_mut().enumerate() {
                *byte = self.read_byte(i == len - 1)?;
            }

            // STOP 조건 생성
            self.stop()?;
        }

        Ok(buffer.len())
    }

    /// I2C 쓰기
    ///
    /// # Arguments
    /// * `slave_addr` - 슬레이브 장치 주소 (7비트)
    /// * `buffer` - 전송할 데이터
    ///
    /// # Returns
    /// 실제로 전송한 바이트 수
    pub fn write(&self, slave_addr: u8, buffer: &[u8]) -> Result<usize, I2cError> {
        if !self.initialized {
            return Err(I2cError::NotInitialized);
        }

        unsafe {
            // START 조건 생성
            self.start()?;

            // 슬레이브 주소 전송 (쓰기 모드: 주소 | 0x00)
            self.write_byte(slave_addr << 1)?;

            // 데이터 전송
            for &byte in buffer {
                self.write_byte(byte)?;
            }

            // STOP 조건 생성
            self.stop()?;
        }

        Ok(buffer.len())
    }

    /// START 조건 생성
    unsafe fn start(&self) -> Result<(), I2cError> {
        // START 비트 설정
        let control = self.read_reg(I2cReg::Control);
        self.write_reg(I2cReg::Control, control | 0x02); // START bit
        
        // START 완료 대기
        for _ in 0..1000 {
            let status = self.read_reg(I2cReg::Status);
            if status & 0x08 != 0 { // START 완료
                return Ok(());
            }
        }
        
        Err(I2cError::Timeout)
    }

    /// STOP 조건 생성
    unsafe fn stop(&self) -> Result<(), I2cError> {
        // STOP 비트 설정
        let control = self.read_reg(I2cReg::Control);
        self.write_reg(I2cReg::Control, control | 0x04); // STOP bit
        
        // STOP 완료 대기
        for _ in 0..1000 {
            let status = self.read_reg(I2cReg::Status);
            if status & 0x10 != 0 { // STOP 완료
                return Ok(());
            }
        }
        
        Ok(()) // STOP은 타임아웃 무시
    }

    /// 1바이트 쓰기
    unsafe fn write_byte(&self, byte: u8) -> Result<(), I2cError> {
        // 데이터 레지스터에 쓰기
        self.write_reg(I2cReg::Data, byte as u32);
        
        // 전송 완료 대기
        for _ in 0..1000 {
            let status = self.read_reg(I2cReg::Status);
            
            // NACK 확인
            if status & 0x20 != 0 {
                return Err(I2cError::Nack);
            }
            
            // 전송 완료
            if status & 0x40 != 0 {
                return Ok(());
            }
        }
        
        Err(I2cError::Timeout)
    }

    /// 1바이트 읽기
    ///
    /// # Arguments
    /// * `last` - 마지막 바이트인 경우 true (NACK 전송)
    unsafe fn read_byte(&self, last: bool) -> Result<u8, I2cError> {
        // ACK/NACK 설정
        let control = self.read_reg(I2cReg::Control);
        if last {
            self.write_reg(I2cReg::Control, control | 0x08); // NACK
        } else {
            self.write_reg(I2cReg::Control, control & !0x08); // ACK
        }
        
        // 수신 완료 대기
        for _ in 0..1000 {
            let status = self.read_reg(I2cReg::Status);
            
            // 수신 완료
            if status & 0x80 != 0 {
                return Ok(self.read_reg(I2cReg::Data) as u8);
            }
        }
        
        Err(I2cError::Timeout)
    }

    /// 초기화 여부 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// 전역 I2C 컨트롤러 인스턴스 (최대 4개 지원)
static I2C_CONTROLLERS: Mutex<[I2cController; 4]> = Mutex::new([
    I2cController::new(),
    I2cController::new(),
    I2cController::new(),
    I2cController::new(),
]);

/// I2C 컨트롤러 초기화
///
/// # Arguments
/// * `index` - 컨트롤러 인덱스 (0-3)
/// * `physical_base` - I2C 컨트롤러의 물리 베이스 주소
///
/// # Safety
/// 메모리 관리가 초기화된 후에 호출되어야 합니다.
pub unsafe fn init_controller(index: usize, physical_base: PhysAddr) -> Result<(), I2cError> {
    if index >= 4 {
        return Err(I2cError::InvalidAddress);
    }

    let mut controllers = I2C_CONTROLLERS.lock();
    controllers[index].init(physical_base)
}

/// I2C 컨트롤러 가져오기
pub fn get_controller(index: usize) -> Option<&'static Mutex<[I2cController; 4]>> {
    if index >= 4 {
        return None;
    }
    Some(&I2C_CONTROLLERS)
}

/// I2C 읽기 (컨트롤러 0 사용)
pub fn read(slave_addr: u8, buffer: &mut [u8]) -> Result<usize, I2cError> {
    let controllers = I2C_CONTROLLERS.lock();
    controllers[0].read(slave_addr, buffer)
}

/// I2C 쓰기 (컨트롤러 0 사용)
pub fn write(slave_addr: u8, buffer: &[u8]) -> Result<usize, I2cError> {
    let controllers = I2C_CONTROLLERS.lock();
    controllers[0].write(slave_addr, buffer)
}

