//! I2C-HID 프로토콜 구현
//!
//! I2C-HID 사양을 따르는 HID 장치를 지원합니다.

use crate::drivers::i2c::{I2cError, read as i2c_read, write as i2c_write};

/// I2C-HID 에러 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cHidError {
    /// I2C 통신 에러
    I2cError(I2cError),
    /// HID Descriptor를 읽을 수 없음
    DescriptorReadError,
    /// 잘못된 Descriptor
    InvalidDescriptor,
    /// 지원하지 않는 버전
    UnsupportedVersion,
    /// 장치가 응답하지 않음
    NoResponse,
    /// 버퍼 오버플로우
    BufferOverflow,
}

impl From<I2cError> for I2cHidError {
    fn from(err: I2cError) -> Self {
        I2cHidError::I2cError(err)
    }
}

/// I2C-HID Descriptor 구조체
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct I2cHidDescriptor {
    /// Descriptor 길이 (항상 30)
    pub length: u16,
    /// bcdVersion (1.00 = 0x0100)
    pub bcd_version: u16,
    /// Report Descriptor 길이
    pub report_desc_length: u16,
    /// Report Descriptor 레지스터
    pub report_desc_register: u16,
    /// Input Report 레지스터
    pub input_register: u16,
    /// Input Report 최대 길이
    pub max_input_length: u16,
    /// Output Report 레지스터
    pub output_register: u16,
    /// Output Report 최대 길이
    pub max_output_length: u16,
    /// Command 레지스터
    pub command_register: u16,
    /// Data 레지스터
    pub data_register: u16,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Version ID
    pub version_id: u16,
    /// 예약됨
    pub reserved: u32,
}

/// I2C-HID 명령어
#[repr(u8)]
#[allow(dead_code)]
enum I2cHidCommand {
    Reset = 0x01,
    GetReport = 0x02,
    SetReport = 0x03,
    GetIdle = 0x04,
    SetIdle = 0x05,
    GetProtocol = 0x06,
    SetProtocol = 0x07,
    SetPower = 0x08,
}

/// I2C-HID Power State
#[repr(u8)]
#[allow(dead_code)]
enum PowerState {
    On = 0x00,
    Sleep = 0x01,
}

/// I2C-HID 장치
pub struct I2cHidDevice {
    /// I2C 슬레이브 주소
    slave_addr: u8,
    /// HID Descriptor
    descriptor: Option<I2cHidDescriptor>,
    /// 초기화 여부
    initialized: bool,
}

impl I2cHidDevice {
    /// 새 I2C-HID 장치 생성
    ///
    /// # Arguments
    /// * `slave_addr` - I2C 슬레이브 주소 (7비트)
    pub const fn new(slave_addr: u8) -> Self {
        Self {
            slave_addr,
            descriptor: None,
            initialized: false,
        }
    }

    /// I2C-HID 장치 초기화
    ///
    /// # Safety
    /// I2C 컨트롤러가 초기화된 후에 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), I2cHidError> {
        // HID Descriptor 읽기 (레지스터 0x0001에서 시작)
        self.descriptor = Some(self.read_hid_descriptor()?);

        // Descriptor 검증
        let desc = match self.descriptor {
            Some(d) => d,
            None => return Err(I2cHidError::InvalidDescriptor),
        };
        if desc.length != 30 {
            return Err(I2cHidError::InvalidDescriptor);
        }
        if desc.bcd_version != 0x0100 {
            return Err(I2cHidError::UnsupportedVersion);
        }

        let vendor_id = desc.vendor_id;
        let product_id = desc.product_id;
        let version_id = desc.version_id;
        crate::log_info!("I2C-HID Device: Vendor=0x{:04X}, Product=0x{:04X}, Version=0x{:04X}",
                        vendor_id, product_id, version_id);

        // 장치 리셋
        self.reset()?;

        // Power On
        self.set_power(PowerState::On)?;

        self.initialized = true;
        Ok(())
    }

    /// HID Descriptor 읽기
    ///
    /// I2C-HID 사양에 따라 레지스터 0x0001에서 30바이트를 읽습니다.
    unsafe fn read_hid_descriptor(&self) -> Result<I2cHidDescriptor, I2cHidError> {
        let mut buffer = [0u8; 30];
        
        // 레지스터 주소 0x0001 전송 후 읽기
        let reg_addr = [0x01, 0x00]; // Little-endian
        i2c_write(self.slave_addr, &reg_addr)?;
        
        // Descriptor 읽기
        i2c_read(self.slave_addr, &mut buffer)?;

        // 구조체로 변환
        let descriptor: I2cHidDescriptor = core::ptr::read(buffer.as_ptr() as *const _);
        
        Ok(descriptor)
    }

    /// 장치 리셋
    fn reset(&self) -> Result<(), I2cHidError> {
        if let Some(_desc) = self.descriptor {
            let command = self.create_command(I2cHidCommand::Reset, 0, 0);
            i2c_write(self.slave_addr, &command)?;
            
            // 리셋 완료 대기 (최소 5ms)
            crate::drivers::timer::sleep_ms(10);
        }
        Ok(())
    }

    /// Power State 설정
    fn set_power(&self, state: PowerState) -> Result<(), I2cHidError> {
        if let Some(_desc) = self.descriptor {
            let command = self.create_command(I2cHidCommand::SetPower, state as u8, 0);
            i2c_write(self.slave_addr, &command)?;
            
            // Power 상태 변경 대기
            crate::drivers::timer::sleep_ms(5);
        }
        Ok(())
    }

    /// Input Report 읽기
    ///
    /// # Arguments
    /// * `buffer` - 읽은 데이터를 저장할 버퍼
    ///
    /// # Returns
    /// 실제로 읽은 바이트 수
    pub fn read_input_report(&self, buffer: &mut [u8]) -> Result<usize, I2cHidError> {
        if !self.initialized {
            return Err(I2cHidError::I2cError(I2cError::NotInitialized));
        }

        if let Some(desc) = self.descriptor {
            // Input Register에서 읽기
            let reg_addr = [
                (desc.input_register & 0xFF) as u8,
                ((desc.input_register >> 8) & 0xFF) as u8,
            ];
            i2c_write(self.slave_addr, &reg_addr)?;
            
            // 데이터 읽기 (첫 2바이트는 길이)
            let max_len = (desc.max_input_length as usize).min(buffer.len());
            let mut temp_buffer = [0u8; 64]; // 임시 버퍼 (최대 64바이트)
            
            i2c_read(self.slave_addr, &mut temp_buffer[..max_len])?;
            
            // 첫 2바이트는 길이 정보
            let data_len = u16::from_le_bytes([temp_buffer[0], temp_buffer[1]]) as usize;
            
            if data_len > buffer.len() {
                return Err(I2cHidError::BufferOverflow);
            }
            
            // 실제 데이터 복사 (길이 정보 제외)
            buffer[..data_len].copy_from_slice(&temp_buffer[2..2 + data_len]);
            
            Ok(data_len)
        } else {
            Err(I2cHidError::DescriptorReadError)
        }
    }

    /// Output Report 쓰기
    ///
    /// # Arguments
    /// * `report_id` - Report ID
    /// * `data` - 전송할 데이터
    pub fn write_output_report(&self, report_id: u8, data: &[u8]) -> Result<(), I2cHidError> {
        if !self.initialized {
            return Err(I2cHidError::I2cError(I2cError::NotInitialized));
        }

        if let Some(desc) = self.descriptor {
            // Output Register 주소 + 데이터 구성
            let mut buffer = [0u8; 66]; // 레지스터(2) + 길이(2) + Report ID(1) + 데이터(최대 61)
            
            // Output Register 주소
            buffer[0] = (desc.output_register & 0xFF) as u8;
            buffer[1] = ((desc.output_register >> 8) & 0xFF) as u8;
            
            // 길이 (Report ID + 데이터)
            let total_len = (1 + data.len()) as u16;
            buffer[2] = (total_len & 0xFF) as u8;
            buffer[3] = ((total_len >> 8) & 0xFF) as u8;
            
            // Report ID
            buffer[4] = report_id;
            
            // 데이터
            buffer[5..5 + data.len()].copy_from_slice(data);
            
            // I2C 전송
            i2c_write(self.slave_addr, &buffer[..5 + data.len()])?;
            
            Ok(())
        } else {
            Err(I2cHidError::DescriptorReadError)
        }
    }

    /// I2C-HID 명령어 생성
    fn create_command(&self, cmd: I2cHidCommand, arg1: u8, arg2: u8) -> [u8; 4] {
        if let Some(desc) = self.descriptor {
            [
                (desc.command_register & 0xFF) as u8,
                ((desc.command_register >> 8) & 0xFF) as u8,
                (cmd as u8) | (arg1 << 4),
                arg2,
            ]
        } else {
            [0; 4]
        }
    }

    /// Descriptor 가져오기
    pub fn get_descriptor(&self) -> Option<I2cHidDescriptor> {
        self.descriptor
    }

    /// 초기화 여부 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// I2C 주소 가져오기
    pub fn get_slave_addr(&self) -> u8 {
        self.slave_addr
    }
}

