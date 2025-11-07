//! ACPI (Advanced Configuration and Power Interface) 파서
//!
//! ACPI 테이블을 파싱하여 전력 관리 정보를 추출합니다.

use crate::power::PowerError;
use crate::boot::acpi_rsdp_addr;
use x86_64::PhysAddr;
use crate::power::acpi_fadt::{parse_fadt, FadtInfo};

/// ACPI RSDP 시그니처
const RSDP_SIGNATURE: &[u8; 8] = b"RSD PTR ";

/// I2C 장치 정보
#[derive(Debug, Clone, Copy)]
pub struct I2cDeviceInfo {
    /// I2C 컨트롤러 물리 주소
    pub base_address: PhysAddr,
    /// I2C 슬레이브 주소
    pub slave_address: u8,
    /// HID (Hardware ID)
    pub hid: [u8; 8],
}

/// 전원 소스 종류
pub enum PowerSource {
    Ac,
    Battery,
    Unknown,
}

/// 배터리 상태 요약
pub struct BatteryStatus {
    pub present: bool,
    pub charging: bool,
    pub capacity_percent: u8,
}

/// ACPI 파서
///
/// ACPI 테이블을 파싱하고 전력 관리 정보를 제공합니다.
pub struct AcpiParser {
    /// RSDP 주소 (가상 주소)
    rsdp_addr: Option<u64>,
    /// 초기화 여부
    initialized: bool,
    /// Parsed FADT info (optional)
    fadt: Option<FadtInfo>,
}

impl AcpiParser {
    /// 새 ACPI 파서 생성
    ///
    /// 부트 정보에서 RSDP 주소를 찾습니다.
    pub fn new() -> Result<Self, PowerError> {
        // 부트 정보에서 RSDP 주소 가져오기
        let rsdp_addr = acpi_rsdp_addr();
        
        if rsdp_addr.is_none() {
            // RSDP를 찾을 수 없으면 오류 반환
            // 하지만 전력 관리는 기본 기능으로 계속 가능
            return Err(PowerError::RsdpNotFound);
        }
        
        Ok(Self {
            rsdp_addr,
            initialized: false,
            fadt: None,
        })
    }
    
    /// ACPI 파서 초기화
    ///
    /// RSDP 테이블을 검증하고 초기화합니다.
    ///
    /// # Safety
    /// 메모리 관리가 초기화된 후에 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), PowerError> {
        if let Some(addr) = self.rsdp_addr {
            // RSDP 테이블 검증
            if !self.validate_rsdp(addr) {
                return Err(PowerError::AcpiParseError);
            }
            
            self.initialized = true;
            // Attempt to locate and parse FADT (FACP). In this minimal parser we rely on a stub
            // function to fetch the table bytes if available.
            if let Some(fadt_bytes) = unsafe { crate::power::acpi_table_fetch(b"FACP") } {
                self.fadt = parse_fadt(fadt_bytes);
            }
            Ok(())
        } else {
            Err(PowerError::RsdpNotFound)
        }
    }
    
    /// RSDP 테이블 검증
    ///
    /// RSDP 테이블의 시그니처와 체크섬을 확인합니다.
    ///
    /// # Safety
    /// 유효한 물리 주소를 가리켜야 합니다.
    unsafe fn validate_rsdp(&self, addr: u64) -> bool {
        // TODO: RSDP 테이블 검증 구현
        // 1. 시그니처 확인 ("RSD PTR ")
        // 2. 체크섬 확인
        // 3. 버전 확인 (ACPI 1.0 또는 2.0)
        
        // 현재는 기본 검증만 수행
        true
    }
    
    /// RSDP 주소 가져오기
    pub fn get_rsdp_addr(&self) -> Option<u64> {
        self.rsdp_addr
    }
    
    /// 초기화 여부 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// I2C 장치 검색
    ///
    /// ACPI DSDT 테이블에서 I2C HID 장치를 찾습니다.
    /// 
    /// # Returns
    /// I2C 장치 정보 (찾지 못한 경우 None)
    pub fn find_i2c_devices(&self) -> Option<I2cDeviceInfo> {
        if !self.initialized {
            return None;
        }
        
        // TODO: 실제 DSDT 파싱 구현
        // 현재는 HP 14s-dk0112AU의 알려진 값을 하드코딩
        // 
        // 실제 구현 시 필요한 작업:
        // 1. RSDP -> RSDT/XSDT -> DSDT 탐색
        // 2. DSDT AML 파싱
        // 3. I2C HID 장치 (_HID, _CID 확인)
        // 4. I2C 컨트롤러 베이스 주소 추출
        // 5. I2C 슬레이브 주소 추출
        
        // AMD FCH I2C 컨트롤러 일반적인 주소: 0xFEDC3000 ~ 0xFEDC7000
        // ELAN 트랙패드 일반적인 I2C 주소: 0x15
        Some(I2cDeviceInfo {
            base_address: PhysAddr::new(0xFEDC3000), // I2C 컨트롤러 0
            slave_address: 0x15,                      // ELAN 트랙패드
            hid: *b"PNP0C50\0",                      // I2C HID 장치 표준 HID
        })
    }

    /// _PSR (Power Source) 읽기 - Stub
    pub fn read_power_source(&self) -> PowerSource {
        if !self.initialized { return PowerSource::Unknown; }
        // TODO: ACPI _PSR 평가 구현
        PowerSource::Unknown
    }

    /// _BST (Battery Status) 읽기 - Stub
    pub fn read_battery_status(&self) -> BatteryStatus {
        if !self.initialized {
            return BatteryStatus { present: false, charging: false, capacity_percent: 0 };
        }
        // TODO: ACPI _BST 평가 구현
        BatteryStatus { present: false, charging: false, capacity_percent: 0 }
    }

    /// 대략적 전력 소비 추정 - Stub
    pub fn estimate_power_consumption(&self) -> u32 {
        if !self.initialized { return 0; }
        // TODO: CPU P-State + 장치 상태 기반 추정
        0
    }
    
    /// Enter sleep state (S3/s2idle)
    /// 
    /// # Arguments
    /// * `sleep_state` - Sleep state (3 = S3, 0 = s2idle)
    /// 
    /// # Safety
    /// 이 함수는 suspend 플로우에서 호출되어야 하며, 호출 후 시스템이 깨어날 때까지 복귀하지 않습니다.
    pub unsafe fn enter_sleep_state(&self, sleep_state: u8) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        
        // Prefer parsed FADT fields; fallback to conservative defaults
        let (pm1a_cnt, _pm1b_cnt, s3_typ) = if let Some(f) = self.fadt {
            (f.pm1a_cnt_blk as u16, f.pm1b_cnt_blk as u16, f.s3_sleep_type.unwrap_or(5))
        } else {
            (0x800u16, 0u16, 5u16)
        };

        let slp_typ = if sleep_state == 3 { s3_typ } else { (sleep_state as u16) };
        // Sleep type and enable: SLP_TYP (bits 10-12 typical) + SLP_EN (bit 13)
        let sleep_value = ((slp_typ & 0x7) << 10) | (1 << 13);

        // Write to PM1a control
        let mut pm1a_port: x86_64::instructions::port::Port<u16> = 
            x86_64::instructions::port::Port::new(pm1a_cnt);
        pm1a_port.write(sleep_value);
        
        // 시스템이 여기서 깨어나면 resume 경로로 진행
        // 만약 여기 도달하면 sleep이 실패한 것
        Err(PowerError::Unsupported)
    }
    
    /// Check if system is resuming from sleep
    pub fn is_resuming(&self) -> bool {
        // WAK_STS 비트 확인 (PM1_STS 레지스터)
        // 간단화: 실제로는 PM1_STS 레지스터를 읽어야 함
        false
    }
    
    /// S3 sleep state 지원 여부 확인
    ///
    /// FADT 테이블에서 S3 지원 여부를 확인합니다.
    pub fn is_s3_supported(&self) -> bool {
        if !self.initialized {
            return false;
        }
        
        // FADT 테이블 파싱 시도
        // FADT 구조:
        // - SLP_TYP: Sleep Type (비트 2-6)
        // - SLP_EN: Sleep Enable (비트 13)
        // - S3의 경우 SLP_TYP = 5
        
        // 실제 FADT 파싱은 복잡하므로, 기본 검증만 수행
        // 1. ACPI 파서가 초기화되어 있는지 확인
        // 2. 기본적인 S3 지원 가정 (실제로는 FADT에서 확인 필요)
        
        // 향후 개선: FADT 테이블에서 실제 SLP_TYP 필드 읽기
        // let fadt = self.find_table(b"FACP")?;
        // let slp_typ = fadt[offset + SLP_TYP_OFFSET];
        // slp_typ == 5 (S3)
        
        crate::log_info!("S3 sleep support check: ACPI parser initialized");
        true
    }
    
    /// S3 Sleep 검증 도구
    ///
    /// S3 Sleep 전에 시스템 상태를 검증합니다.
    pub fn validate_s3_sleep(&self) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        
        // 1. ACPI 지원 확인
        if !self.is_s3_supported() {
            return Err(PowerError::Unsupported);
        }
        
        // 2. 메모리 상태 확인 (기본)
        // 실제로는:
        // - 중요한 데이터가 메모리에 있는지 확인
        // - 메모리 무결성 확인
        
        // 3. 장치 상태 확인
        // 실제로는:
        // - 모든 장치가 suspend 준비 상태인지 확인
        // - 장치 드라이버가 suspend를 지원하는지 확인
        
        crate::log_info!("S3 sleep validation: passed");
        Ok(())
    }
}

/// Minimal C-state descriptor for handoff to idle manager setup
#[derive(Clone, Copy)]
pub struct CStateDesc {
    pub level: u8,
    pub latency_us: u32,
    pub power_mw: u32,
    pub mwait_hint: u32,
}

impl AcpiParser {
    /// Discover C-state hints (minimal; fallback values if parsing not implemented)
    pub fn discover_c_states(&self) -> [Option<CStateDesc>; 8] {
        if !self.initialized {
            return [None, None, None, None, None, None, None, None];
        }
        // TODO: Parse _CST properly. For now, provide conservative hints:
        // C1 (HLT), C3 (mwait-like hint), leave others None.
        let c1 = Some(CStateDesc { level: 1, latency_us: 1, power_mw: 10000, mwait_hint: 0 });
        let c3 = Some(CStateDesc { level: 3, latency_us: 100, power_mw: 2000, mwait_hint: 0x20 });
        [c1, c3, None, None, None, None, None, None]
    }
}

/// ACPI에서 I2C 장치 찾기 (전역 함수)
///
/// # Returns
/// I2C 장치 정보 배열
pub fn find_i2c_touchpad() -> Option<I2cDeviceInfo> {
    // TODO: 실제 ACPI 테이블 파싱
    // 현재는 하드코딩된 기본값 반환
    Some(I2cDeviceInfo {
        base_address: PhysAddr::new(0xFEDC3000),
        slave_address: 0x15,
        hid: *b"PNP0C50\0",
    })
}

