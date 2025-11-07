//! 전력 통계 실시간 모니터링 및 대시보드
//!
//! 전력 통계를 실시간으로 모니터링하고 대시보드 형태로 출력합니다.

use crate::power::stats::PowerStatistics;
use crate::power::temps;

/// 전력 모니터링 대시보드
pub struct PowerMonitor {
    /// 업데이트 간격 (ms)
    update_interval_ms: u64,
    /// 마지막 업데이트 시간
    last_update_ms: u64,
    /// 알림 활성화 여부
    alerts_enabled: bool,
}

impl PowerMonitor {
    /// 새 전력 모니터 생성
    pub fn new() -> Self {
        Self {
            update_interval_ms: 1000, // 1초마다 업데이트
            last_update_ms: 0,
            alerts_enabled: true,
        }
    }
    
    /// 대시보드 업데이트
    pub fn update(&mut self, now_ms: u64) {
        // 업데이트 간격 체크
        if now_ms - self.last_update_ms < self.update_interval_ms {
            return;
        }
        
        self.last_update_ms = now_ms;
        
        // 전력 통계 가져오기
        let stats = crate::power::stats::get_statistics();
        
        // 대시보드 출력
        self.print_dashboard(&stats);
        
        // 알림 체크
        if self.alerts_enabled {
            self.check_alerts(&stats);
        }
    }
    
    /// 대시보드 출력
    fn print_dashboard(&self, stats: &PowerStatistics) {
        crate::log_info!("=== Power Monitor Dashboard ===");
        crate::log_info!("Uptime: {} s", stats.uptime_ms / 1000);
        crate::log_info!("Power: avg={}mW peak={}mW energy={}mJ", 
                        stats.avg_power_mw, 
                        stats.peak_power_mw, 
                        stats.energy_consumed_mj);
        
        // C-State residency 출력
        crate::log_info!("C-State Residency:");
        for i in 0..8 {
            let residency = stats.get_c_state_residency(i);
            if residency > 0.1 {
                crate::log_info!("  C{}: {:.2}%", i, residency);
            }
        }
        
        // P-State residency 출력
        crate::log_info!("P-State Residency:");
        for i in 0..16 {
            let residency = stats.get_p_state_residency(i);
            if residency > 0.1 {
                crate::log_info!("  P{}: {:.2}%", i, residency);
            }
        }
        
        // Wakeup rate
        let wakeup_rate = stats.get_wakeup_rate();
        crate::log_info!("Wakeup rate: {:.2}/s", wakeup_rate);
        
        // 온도 정보
        if let Some(temp) = temps::read_package_temperature_c() {
            crate::log_info!("Temperature: {}°C", temp);
            if temps::is_thermal_throttling() {
                crate::log_warn!("Thermal throttling active!");
            }
        }
        
        crate::log_info!("===============================");
    }
    
    /// 알림 체크
    fn check_alerts(&self, stats: &PowerStatistics) {
        // 전력 알림
        if stats.avg_power_mw > 15000 {
            crate::log_warn!("High power consumption: {}mW", stats.avg_power_mw);
        }
        
        // Wakeup rate 알림
        let wakeup_rate = stats.get_wakeup_rate();
        if wakeup_rate > 100.0 {
            crate::log_warn!("High wakeup rate: {:.2}/s (may indicate power inefficiency)", wakeup_rate);
        }
        
        // C-State residency 알림
        let c0_residency = stats.get_c_state_residency(0);
        if c0_residency > 50.0 {
            crate::log_warn!("High C0 residency: {:.2}% (CPU not entering idle states)", c0_residency);
        }
    }
}

static POWER_MONITOR: spin::Mutex<PowerMonitor> = spin::Mutex::new(PowerMonitor {
    update_interval_ms: 1000,
    last_update_ms: 0,
    alerts_enabled: true,
});

/// 전력 모니터 업데이트
pub fn update_monitor(now_ms: u64) {
    POWER_MONITOR.lock().update(now_ms);
}

/// 알림 활성화/비활성화
pub fn set_alerts_enabled(enabled: bool) {
    POWER_MONITOR.lock().alerts_enabled = enabled;
}

/// 대시보드 수동 출력
pub fn print_dashboard() {
    let stats = crate::power::stats::get_statistics();
    let monitor = PowerMonitor::new();
    monitor.print_dashboard(&stats);
}



