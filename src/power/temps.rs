//! CPU temperature reading (Intel best-effort)

use spin::Mutex;

const MSR_IA32_THERM_STATUS: u32 = 0x19C;

#[inline]
fn msr_supported() -> bool {
    let r = unsafe { core::arch::x86_64::__cpuid(1) };
    // EDX bit 5 indicates MSR support
    (r.edx & (1 << 5)) != 0
}

#[inline]
unsafe fn read_msr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nostack, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Read approximate package temperature in Celsius assuming TjMax=100C
pub fn read_package_temperature_c() -> Option<u8> {
    if !msr_supported() { return None; }
    unsafe {
        let v = read_msr(MSR_IA32_THERM_STATUS);
        if (v & (1 << 31)) == 0 { return None; } // reading invalid
        let dts = ((v >> 16) & 0x7F) as u8; // delta to TjMax
        let tjmax = 100u8; // conservative default
        Some(tjmax.saturating_sub(dts))
    }
}

/// Thermal throttle 상태 확인
pub fn is_thermal_throttling() -> bool {
    if !msr_supported() { return false; }
    unsafe {
        let v = read_msr(MSR_IA32_THERM_STATUS);
        // PROCHOT# (Processor Hot) bit (bit 0)
        (v & 1) != 0
    }
}

/// Thermal 상태 모니터링 및 throttle hook
pub struct ThermalMonitor {
    /// 온도 임계값 (Celsius)
    throttle_threshold_c: u8,
    /// 최대 온도 (Celsius)
    max_temperature_c: u8,
    /// 이전 온도
    last_temperature_c: Option<u8>,
    /// Thermal runaway 카운터
    runaway_counter: u32,
}

impl ThermalMonitor {
    /// 새 Thermal 모니터 생성
    pub fn new() -> Self {
        Self {
            throttle_threshold_c: 80,  // 80도에서 throttle 시작
            max_temperature_c: 95,     // 95도 최대 온도
            last_temperature_c: None,
            runaway_counter: 0,
        }
    }
    
    /// 온도 임계값 설정
    pub fn set_throttle_threshold(&mut self, threshold_c: u8) {
        self.throttle_threshold_c = threshold_c;
    }
    
    /// Thermal 상태 확인 및 throttle 필요 여부 반환
    pub fn check_thermal_state(&mut self) -> ThermalAction {
        let current_temp = match read_package_temperature_c() {
            Some(temp) => temp,
            None => {
                // 온도 읽기 실패 시 이전 값 사용
                return if let Some(last) = self.last_temperature_c {
                    if last >= self.throttle_threshold_c {
                        ThermalAction::Throttle
                    } else {
                        ThermalAction::Normal
                    }
                } else {
                    ThermalAction::Normal
                };
            }
        };
        
        // Thermal runaway 감지: 온도가 계속 상승하면
        if let Some(last) = self.last_temperature_c {
            if current_temp > last && current_temp >= self.throttle_threshold_c {
                self.runaway_counter += 1;
            } else {
                self.runaway_counter = 0;
            }
        }
        
        self.last_temperature_c = Some(current_temp);
        
        // Thermal runaway 방지: 온도가 계속 상승하면 강제 throttle
        if self.runaway_counter > 5 {
            return ThermalAction::EmergencyThrottle;
        }
        
        // 온도 기반 throttle 결정
        if current_temp >= self.max_temperature_c {
            ThermalAction::EmergencyThrottle
        } else if current_temp >= self.throttle_threshold_c {
            ThermalAction::Throttle
        } else if is_thermal_throttling() {
            // 하드웨어 throttle 감지
            ThermalAction::Throttle
        } else {
            ThermalAction::Normal
        }
    }
}

/// Thermal 동작 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalAction {
    /// 정상 상태
    Normal,
    /// Throttle 필요 (온도 높음)
    Throttle,
    /// 긴급 Throttle (최대 온도 도달)
    EmergencyThrottle,
}

/// 전역 Thermal 모니터
static THERMAL_MONITOR: Mutex<ThermalMonitor> = Mutex::new(ThermalMonitor {
    throttle_threshold_c: 80,
    max_temperature_c: 95,
    last_temperature_c: None,
    runaway_counter: 0,
});

/// CPU 온도 모니터링 초기화
pub fn init_thermal_monitoring() {
    // 초기 온도 읽기
    if let Some(temp) = read_package_temperature_c() {
        let mut monitor = THERMAL_MONITOR.lock();
        monitor.last_temperature_c = Some(temp);
        crate::log_info!("Thermal monitoring initialized: {}°C", temp);
    } else {
        crate::log_warn!("Thermal monitoring not available (MSR not supported)");
    }
}

/// 주기적 온도 모니터링 (타이머 틱에서 호출)
pub fn periodic_thermal_check() {
    let action = check_thermal_and_throttle();
    
    // 온도 로깅 (디버그 모드 또는 주기적)
    #[cfg(debug_assertions)]
    {
        if let Some(temp) = read_package_temperature_c() {
            if temp >= 70 {
                crate::log_debug!("CPU temperature: {}°C", temp);
            }
        }
    }
    
    // Emergency throttle 시 추가 조치
    if matches!(action, ThermalAction::EmergencyThrottle) {
        // 강제로 CPU를 최저 주파수로 설정
        if let Some(manager) = crate::power::get_manager() {
            if let Some(pm) = manager.lock().as_mut() {
                // 최대 절전 모드로 전환 시도
                if let Err(e) = pm.apply_emergency_throttle() {
                    crate::log_error!("Emergency throttle failed: {:?}", e);
                }
            }
        }
    }
}

/// Thermal 상태 확인 및 throttle hook 호출
pub fn check_thermal_and_throttle() -> ThermalAction {
    let action = THERMAL_MONITOR.lock().check_thermal_state();
    
    match action {
        ThermalAction::Throttle => {
            crate::log_warn!("Thermal throttle: temperature high, reducing CPU frequency");
            // Power manager에 throttle 알림
            if let Some(manager) = crate::power::get_manager() {
                if let Some(pm) = manager.lock().as_mut() {
                    // CPU 주파수 감소
                    let _ = pm.apply_thermal_throttle();
                }
            }
        }
        ThermalAction::EmergencyThrottle => {
            crate::log_error!("Emergency thermal throttle: maximum temperature reached!");
            // 긴급 상황: 최대 절전 모드로 전환
            if let Some(manager) = crate::power::get_manager() {
                if let Some(pm) = manager.lock().as_mut() {
                    let _ = pm.apply_emergency_throttle();
                }
            }
        }
        ThermalAction::Normal => {}
    }
    
    action
}


