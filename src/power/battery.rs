//! 배터리 관리 및 배터리 수준 기반 정책
//!
//! 이 모듈은 배터리 상태를 확인하고, 배터리 수준에 따라 전력 프로파일을 자동으로 조정합니다.
//!
//! # 배터리 관리
//!
//! 1. **배터리 상태 확인**: ACPI를 통해 배터리 정보 읽기
//! 2. **배터리 수준 모니터링**: 배터리 잔량 추적
//! 3. **자동 전원 관리**: 배터리 수준에 따른 전력 프로파일 조정

use spin::Mutex;
use crate::power::policy::PowerMode;
use crate::power::PowerError;

/// 배터리 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    /// 배터리 없음 (AC 전원)
    NotPresent,
    /// 배터리 충전 중
    Charging,
    /// 배터리 방전 중
    Discharging,
    /// 배터리 완전 충전
    Full,
    /// 배터리 상태 알 수 없음
    Unknown,
}

/// 배터리 정보
#[derive(Debug, Clone, Copy)]
pub struct BatteryInfo {
    /// 배터리 상태
    pub status: BatteryStatus,
    /// 배터리 잔량 (퍼센트, 0-100)
    pub level_percent: u8,
    /// 배터리 용량 (mAh)
    pub capacity_mah: Option<u32>,
    /// 배터리 설계 용량 (mAh)
    pub design_capacity_mah: Option<u32>,
    /// 배터리 전압 (mV)
    pub voltage_mv: Option<u32>,
}

impl Default for BatteryInfo {
    fn default() -> Self {
        Self {
            status: BatteryStatus::Unknown,
            level_percent: 100,
            capacity_mah: None,
            design_capacity_mah: None,
            voltage_mv: None,
        }
    }
}

/// 배터리 관리자
pub struct BatteryManager {
    /// 현재 배터리 정보
    battery_info: BatteryInfo,
    /// 배터리 수준 기반 전원 관리 활성화 여부
    auto_power_management: bool,
    /// 저전력 모드 진입 임계값 (퍼센트)
    low_battery_threshold: u8,
    /// 위험 배터리 수준 임계값 (퍼센트)
    critical_battery_threshold: u8,
    /// 마지막 업데이트 시간 (밀리초)
    last_update_time: u64,
}

impl BatteryManager {
    /// 새 배터리 관리자 생성
    pub fn new() -> Self {
        Self {
            battery_info: BatteryInfo::default(),
            auto_power_management: true,
            low_battery_threshold: 20,  // 20% 이하
            critical_battery_threshold: 10, // 10% 이하
            last_update_time: 0,
        }
    }
    
    /// 배터리 정보 업데이트
    ///
    /// ACPI를 통해 배터리 정보를 읽습니다.
    pub fn update_battery_info(&mut self) -> Result<(), PowerError> {
        // ACPI 배터리 정보 읽기 (간단한 구현)
        // 실제로는 ACPI _BAT 또는 _BST 메서드를 호출해야 함
        
        // 현재는 기본값으로 설정
        // TODO: ACPI 배터리 정보 읽기 구현
        
        self.battery_info = BatteryInfo {
            status: BatteryStatus::Unknown,
            level_percent: 100, // 기본값: 100%
            capacity_mah: None,
            design_capacity_mah: None,
            voltage_mv: None,
        };
        
        self.last_update_time = crate::drivers::timer::get_milliseconds();
        
        Ok(())
    }
    
    /// 배터리 정보 가져오기
    pub fn get_battery_info(&self) -> BatteryInfo {
        self.battery_info
    }
    
    /// 배터리 수준에 따른 권장 전력 모드
    ///
    /// 배터리 수준이 낮으면 저전력 모드를 권장합니다.
    pub fn recommended_power_mode(&self) -> PowerMode {
        if !self.auto_power_management {
            return PowerMode::Balanced;
        }
        
        match self.battery_info.status {
            BatteryStatus::NotPresent => {
                // AC 전원: 성능 모드 가능
                PowerMode::Performance
            }
            BatteryStatus::Charging => {
                // 충전 중: 균형 모드
                PowerMode::Balanced
            }
            BatteryStatus::Discharging => {
                // 방전 중: 배터리 수준에 따라
                if self.battery_info.level_percent <= self.critical_battery_threshold {
                    PowerMode::PowerSaving
                } else if self.battery_info.level_percent <= self.low_battery_threshold {
                    PowerMode::PowerSaving
                } else {
                    PowerMode::Balanced
                }
            }
            BatteryStatus::Full => {
                // 완전 충전: 균형 모드
                PowerMode::Balanced
            }
            BatteryStatus::Unknown => {
                // 상태 알 수 없음: 균형 모드
                PowerMode::Balanced
            }
        }
    }
    
    /// 배터리 수준 기반 전원 관리 조정
    ///
    /// 배터리 수준에 따라 전력 프로파일을 자동으로 조정합니다.
    pub fn adjust_power_based_on_battery(&mut self) -> Result<(), PowerError> {
        if !self.auto_power_management {
            return Ok(());
        }
        
        // 배터리 정보 업데이트
        self.update_battery_info()?;
        
        let recommended_mode = self.recommended_power_mode();
        
        // 현재 전력 관리자 가져오기
        if let Some(pm) = crate::power::get_manager() {
            if let Some(ref mut manager) = pm.lock().as_mut() {
                let current_mode = manager.get_policy();
                
                // 권장 모드가 현재 모드와 다르면 변경
                if recommended_mode != current_mode {
                    crate::log_info!("Adjusting power mode: {:?} -> {:?} (battery: {}%)", 
                                    current_mode, recommended_mode, 
                                    self.battery_info.level_percent);
                    
                    manager.set_policy(recommended_mode)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// 저전력 모드 임계값 설정
    pub fn set_low_battery_threshold(&mut self, threshold: u8) {
        self.low_battery_threshold = threshold.min(100);
    }
    
    /// 위험 배터리 수준 임계값 설정
    pub fn set_critical_battery_threshold(&mut self, threshold: u8) {
        self.critical_battery_threshold = threshold.min(100);
    }
    
    /// 자동 전원 관리 활성화/비활성화
    pub fn set_auto_power_management(&mut self, enabled: bool) {
        self.auto_power_management = enabled;
    }
}

/// 전역 배터리 관리자
static BATTERY_MANAGER: Mutex<BatteryManager> = Mutex::new(BatteryManager {
    battery_info: BatteryInfo::default(),
    auto_power_management: true,
    low_battery_threshold: 20,
    critical_battery_threshold: 10,
    last_update_time: 0,
});

/// 배터리 관리자 초기화
pub fn init_battery_manager() -> Result<(), PowerError> {
    let mut manager = BATTERY_MANAGER.lock();
    *manager = BatteryManager::new();
    
    // 초기 배터리 정보 읽기
    manager.update_battery_info()?;
    
    crate::log_info!("Battery manager initialized (low threshold: {}%, critical: {}%)", 
                    manager.low_battery_threshold, 
                    manager.critical_battery_threshold);
    
    Ok(())
}

/// 배터리 정보 가져오기
pub fn get_battery_info() -> BatteryInfo {
    let manager = BATTERY_MANAGER.lock();
    manager.get_battery_info()
}

/// 배터리 수준 기반 전원 관리 조정
pub fn adjust_power_based_on_battery() -> Result<(), PowerError> {
    let mut manager = BATTERY_MANAGER.lock();
    manager.adjust_power_based_on_battery()
}

/// 배터리 정보 업데이트
pub fn update_battery_info() -> Result<(), PowerError> {
    let mut manager = BATTERY_MANAGER.lock();
    manager.update_battery_info()
}

/// 배터리 수준에 따른 권장 전력 모드
pub fn recommended_power_mode_for_battery() -> PowerMode {
    let manager = BATTERY_MANAGER.lock();
    manager.recommended_power_mode()
}

