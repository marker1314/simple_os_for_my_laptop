//! 디바이스별 독립적인 전력 프로파일 관리
//!
//! 이 모듈은 각 디바이스의 독립적인 전력 프로파일을 관리합니다.
//!
//! # 디바이스별 전력 프로파일
//!
//! - **디스크**: 스핀 다운 타임아웃, 유휴 타임아웃
//! - **네트워크**: 저전력 모드, 유휴 타임아웃
//! - **디스플레이**: 백라이트 밝기, DPMS 설정
//! - **입력 장치**: 인터럽트 활성화/비활성화

use spin::Mutex;
use alloc::collections::BTreeMap;

use crate::power::PowerError;

/// 디바이스 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    Disk,
    Network,
    Display,
    Input,
    Audio,
    Usb,
}

/// 디바이스 전력 프로파일
#[derive(Debug, Clone)]
pub struct DevicePowerProfile {
    /// 디바이스 타입
    device_type: DeviceType,
    /// 유휴 타임아웃 (밀리초, 0 = 비활성화)
    idle_timeout_ms: u64,
    /// 저전력 모드 활성화 여부
    low_power_enabled: bool,
    /// 자동 전원 관리 활성화 여부
    auto_power_management: bool,
    /// 디바이스별 설정 (타입에 따라 다름)
    custom_settings: BTreeMap<&'static str, u64>,
}

impl DevicePowerProfile {
    /// 새 디바이스 전력 프로파일 생성
    pub fn new(device_type: DeviceType) -> Self {
        let (idle_timeout, low_power, auto_pm) = match device_type {
            DeviceType::Disk => (30_000, true, true),      // 30초 유휴 후 스핀 다운
            DeviceType::Network => (60_000, true, true),  // 60초 유휴 후 저전력
            DeviceType::Display => (300_000, true, true),  // 5분 유휴 후 DPMS
            DeviceType::Input => (0, false, false),        // 입력 장치는 항상 활성
            DeviceType::Audio => (10_000, true, true),    // 10초 유휴 후 비활성화
            DeviceType::Usb => (0, false, false),          // USB는 기본적으로 활성
        };
        
        Self {
            device_type,
            idle_timeout_ms: idle_timeout,
            low_power_enabled: low_power,
            auto_power_management: auto_pm,
            custom_settings: BTreeMap::new(),
        }
    }
    
    /// 유휴 타임아웃 설정
    pub fn set_idle_timeout(&mut self, timeout_ms: u64) {
        self.idle_timeout_ms = timeout_ms;
    }
    
    /// 저전력 모드 활성화 설정
    pub fn set_low_power(&mut self, enabled: bool) {
        self.low_power_enabled = enabled;
    }
    
    /// 자동 전원 관리 활성화 설정
    pub fn set_auto_power_management(&mut self, enabled: bool) {
        self.auto_power_management = enabled;
    }
    
    /// 커스텀 설정 추가
    pub fn set_custom(&mut self, key: &'static str, value: u64) {
        self.custom_settings.insert(key, value);
    }
    
    /// 커스텀 설정 가져오기
    pub fn get_custom(&self, key: &'static str) -> Option<u64> {
        self.custom_settings.get(key).copied()
    }
    
    /// 디바이스 타입 가져오기
    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }
    
    /// 유휴 타임아웃 가져오기
    pub fn idle_timeout(&self) -> u64 {
        self.idle_timeout_ms
    }
    
    /// 저전력 모드 활성화 여부
    pub fn is_low_power_enabled(&self) -> bool {
        self.low_power_enabled
    }
    
    /// 자동 전원 관리 활성화 여부
    pub fn is_auto_power_management_enabled(&self) -> bool {
        self.auto_power_management
    }
}

/// 디바이스 전력 프로파일 관리자
pub struct DevicePowerProfileManager {
    profiles: BTreeMap<DeviceType, DevicePowerProfile>,
}

impl DevicePowerProfileManager {
    /// 새 관리자 생성
    pub fn new() -> Self {
        let mut manager = Self {
            profiles: BTreeMap::new(),
        };
        
        // 기본 프로파일 생성
        manager.profiles.insert(DeviceType::Disk, DevicePowerProfile::new(DeviceType::Disk));
        manager.profiles.insert(DeviceType::Network, DevicePowerProfile::new(DeviceType::Network));
        manager.profiles.insert(DeviceType::Display, DevicePowerProfile::new(DeviceType::Display));
        manager.profiles.insert(DeviceType::Input, DevicePowerProfile::new(DeviceType::Input));
        manager.profiles.insert(DeviceType::Audio, DevicePowerProfile::new(DeviceType::Audio));
        manager.profiles.insert(DeviceType::Usb, DevicePowerProfile::new(DeviceType::Usb));
        
        manager
    }
    
    /// 프로파일 가져오기
    pub fn get_profile(&self, device_type: DeviceType) -> Option<&DevicePowerProfile> {
        self.profiles.get(&device_type)
    }
    
    /// 프로파일 가져오기 (가변)
    pub fn get_profile_mut(&mut self, device_type: DeviceType) -> Option<&mut DevicePowerProfile> {
        self.profiles.get_mut(&device_type)
    }
    
    /// 프로파일 설정
    pub fn set_profile(&mut self, profile: DevicePowerProfile) {
        let device_type = profile.device_type();
        self.profiles.insert(device_type, profile);
    }
    
    /// 모든 프로파일 가져오기
    pub fn get_all_profiles(&self) -> &BTreeMap<DeviceType, DevicePowerProfile> {
        &self.profiles
    }
    
    /// 전력 프로파일 기반으로 디바이스 전원 관리 적용
    pub fn apply_device_policies(&self) -> Result<(), PowerError> {
        for (device_type, profile) in &self.profiles {
            if !profile.is_auto_power_management_enabled() {
                continue;
            }
            
            match device_type {
                DeviceType::Disk => {
                    if profile.idle_timeout_ms > 0 {
                        let policy = crate::power::device::DiskPowerPolicy {
                            spin_down_ms: profile.idle_timeout_ms,
                        };
                        crate::power::device::apply_disk_power_policy(&policy);
                    }
                }
                DeviceType::Network => {
                    if profile.is_low_power_enabled() {
                        crate::power::device::enable_network_low_power();
                    }
                }
                DeviceType::Display => {
                    if let Some(brightness) = profile.get_custom("brightness") {
                        let brightness_percent = (brightness.min(100)) as u8;
                        let _ = crate::power::device::set_backlight_brightness(brightness_percent);
                    }
                }
                DeviceType::Input => {
                    // 입력 장치는 항상 활성 유지
                }
                DeviceType::Audio => {
                    // TODO: 오디오 전원 관리
                }
                DeviceType::Usb => {
                    // TODO: USB 전원 관리
                }
            }
        }
        
        Ok(())
    }
}

/// 전역 디바이스 전력 프로파일 관리자
static DEVICE_POWER_PROFILES: Mutex<DevicePowerProfileManager> = Mutex::new(DevicePowerProfileManager {
    profiles: BTreeMap::new(),
});

/// 디바이스 전력 프로파일 관리자 초기화
pub fn init_device_power_profiles() {
    let mut manager = DEVICE_POWER_PROFILES.lock();
    *manager = DevicePowerProfileManager::new();
    crate::log_info!("Device power profiles initialized");
}

/// 디바이스 프로파일 가져오기
pub fn get_device_profile(device_type: DeviceType) -> Option<DevicePowerProfile> {
    let manager = DEVICE_POWER_PROFILES.lock();
    manager.get_profile(device_type).cloned()
}

/// 디바이스 프로파일 설정
pub fn set_device_profile(profile: DevicePowerProfile) {
    let mut manager = DEVICE_POWER_PROFILES.lock();
    manager.set_profile(profile);
    crate::log_info!("Device power profile updated: {:?}", profile.device_type());
}

/// 전력 프로파일 적용
pub fn apply_device_power_policies() -> Result<(), PowerError> {
    let manager = DEVICE_POWER_PROFILES.lock();
    manager.apply_device_policies()
}

