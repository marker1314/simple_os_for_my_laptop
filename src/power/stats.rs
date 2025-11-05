//! Power statistics collection

use spin::Mutex;
use alloc::vec::Vec;

const MOVING_AVG_WINDOW: usize = 100; // Moving average window size

#[derive(Clone, Copy)]
pub struct PowerStatistics {
    pub avg_power_mw: u32,
    pub peak_power_mw: u32,
    pub energy_consumed_mj: u64,
    pub p_state_histogram: [u32; 16],
    pub c_state_histogram: [u32; 8],
    pub uptime_ms: u64,
    last_rapl_nj: Option<u64>,
    // Moving average tracking
    power_samples: [u32; MOVING_AVG_WINDOW],
    sample_count: usize,
    sample_index: usize,
    // Wakeup tracking
    wakeup_count: u64,
    last_wakeup_time_ms: u64,
    // C-state/P-state residency tracking
    current_c_state: u8,
    current_p_state: u8,
    c_state_entry_time_ms: u64,
    p_state_entry_time_ms: u64,
    c_state_residency_ms: [u64; 8],
    p_state_residency_ms: [u64; 16],
}

impl Default for PowerStatistics {
    fn default() -> Self {
        Self {
            avg_power_mw: 0,
            peak_power_mw: 0,
            energy_consumed_mj: 0,
            p_state_histogram: [0; 16],
            c_state_histogram: [0; 8],
            uptime_ms: 0,
            last_rapl_nj: None,
            power_samples: [0; MOVING_AVG_WINDOW],
            sample_count: 0,
            sample_index: 0,
            wakeup_count: 0,
            last_wakeup_time_ms: 0,
            current_c_state: 0,
            current_p_state: 0,
            c_state_entry_time_ms: 0,
            p_state_entry_time_ms: 0,
            c_state_residency_ms: [0; 8],
            p_state_residency_ms: [0; 16],
        }
    }
}

impl PowerStatistics {
    /// Record a power sample with instant power, C-state, and P-state
    pub fn record_sample(&mut self, instant_power_mw: u32, c_state: u8, p_state: u8, now_ms: u64) {
        // Update peak power
        if instant_power_mw > self.peak_power_mw {
            self.peak_power_mw = instant_power_mw;
        }
        
        // Update moving average
        self.power_samples[self.sample_index] = instant_power_mw;
        self.sample_index = (self.sample_index + 1) % MOVING_AVG_WINDOW;
        if self.sample_count < MOVING_AVG_WINDOW {
            self.sample_count += 1;
        }
        
        // Calculate moving average
        let sum: u64 = self.power_samples[..self.sample_count].iter().map(|&x| x as u64).sum();
        self.avg_power_mw = (sum / self.sample_count as u64) as u32;
        
        // Update C-state residency
        if c_state != self.current_c_state {
            if self.current_c_state < 8 {
                let elapsed = now_ms.saturating_sub(self.c_state_entry_time_ms);
                self.c_state_residency_ms[self.current_c_state as usize] += elapsed;
            }
            self.current_c_state = c_state;
            self.c_state_entry_time_ms = now_ms;
            if c_state < 8 {
                self.c_state_histogram[c_state as usize] += 1;
            }
        }
        
        // Update P-state residency
        if p_state != self.current_p_state {
            if self.current_p_state < 16 {
                let elapsed = now_ms.saturating_sub(self.p_state_entry_time_ms);
                self.p_state_residency_ms[self.current_p_state as usize] += elapsed;
            }
            self.current_p_state = p_state;
            self.p_state_entry_time_ms = now_ms;
            if p_state < 16 {
                self.p_state_histogram[p_state as usize] += 1;
            }
        }
    }

    /// Record a wakeup event
    pub fn record_wakeup(&mut self, now_ms: u64) {
        self.wakeup_count += 1;
        self.last_wakeup_time_ms = now_ms;
    }

    /// Get wakeup rate (wakeups per second)
    pub fn get_wakeup_rate(&self) -> f32 {
        if self.uptime_ms == 0 {
            return 0.0;
        }
        (self.wakeup_count as f32) / (self.uptime_ms as f32 / 1000.0)
    }

    /// Get C-state residency percentage
    pub fn get_c_state_residency(&self, state: u8) -> f32 {
        if state >= 8 || self.uptime_ms == 0 {
            return 0.0;
        }
        (self.c_state_residency_ms[state as usize] as f32) / (self.uptime_ms as f32) * 100.0
    }

    /// Get P-state residency percentage
    pub fn get_p_state_residency(&self, state: u8) -> f32 {
        if state >= 16 || self.uptime_ms == 0 {
            return 0.0;
        }
        (self.p_state_residency_ms[state as usize] as f32) / (self.uptime_ms as f32) * 100.0
    }

    pub fn calculate_average_power(&self) -> u32 {
        self.avg_power_mw
    }

    pub fn print_report(&self) {
        let wakeup_rate = self.get_wakeup_rate();
        crate::log_info!(
            "Power: avg={}mW peak={}mW energy={}mJ wakeups={}/s",
            self.avg_power_mw,
            self.peak_power_mw,
            self.energy_consumed_mj,
            wakeup_rate
        );
        
        // Print C-state residency
        for i in 0..8 {
            let residency = self.get_c_state_residency(i);
            if residency > 0.1 {
                crate::log_info!("C{} residency: {:.2}%", i, residency);
            }
        }
    }
}

static STATS: Mutex<PowerStatistics> = Mutex::new(PowerStatistics::default());

/// 전력 통계 가져오기
pub fn get_statistics() -> PowerStatistics {
    *STATS.lock()
}

/// Periodic tick to accumulate stats and occasionally print a report
pub fn tick(now_ms: u64) {
    let mut s = STATS.lock();
    s.uptime_ms = now_ms;
    
    // Get current power from RAPL if available (개선된 측정)
    let instant_power_mw = if let Some(power_watts) = crate::power::rapl::read_power_watts(now_ms) {
        let power_mw = (power_watts * 1000.0) as u32; // Watts -> mW
        
        // 피크 전력 업데이트
        if power_mw > s.peak_power_mw {
            s.peak_power_mw = power_mw;
        }
        
        // 이동 평균 계산
        s.power_samples[s.sample_index] = power_mw;
        s.sample_index = (s.sample_index + 1) % MOVING_AVG_WINDOW;
        if s.sample_count < MOVING_AVG_WINDOW {
            s.sample_count += 1;
        }
        
        // 평균 전력 계산
        let sum: u32 = s.power_samples.iter().sum();
        s.avg_power_mw = sum / s.sample_count as u32;
        
        power_mw
    } else {
        // RAPL 측정 불가능한 경우 기존 평균값 사용
        s.avg_power_mw
    };
    
    // Get current C-state and P-state
    let c_state = if let Some(manager) = crate::power::get_manager() {
        if let Some(pm) = manager.lock().as_ref() {
            // Get current C-state from power manager
            pm.get_current_c_state()
        } else {
            crate::power::idle::get_current_c_state()
        }
    } else {
        crate::power::idle::get_current_c_state()
    };
    
    let p_state = if let Some(manager) = crate::power::get_manager() {
        if let Some(pm) = manager.lock().as_ref() {
            // Get P-state from power manager
            pm.get_current_p_state()
        } else {
            0
        }
    } else {
        0
    };
    
    // Record sample
    s.record_sample(instant_power_mw, c_state, p_state, now_ms);
    
    if now_ms % 10_000 == 0 {
        s.print_report();
    }
}

/// Export power statistics to CSV format (power_idle.csv)
pub fn export_power_idle_csv() {
    let s = STATS.lock();
    crate::serial_println!("timestamp,pkg_w,core_cstate_residency,wakeups_per_s");
    
    // Calculate overall C-state residency (sum of all C-states)
    let total_c_residency = s.c_state_residency_ms.iter().sum::<u64>();
    let c_residency_percent = if s.uptime_ms > 0 {
        (total_c_residency as f32) / (s.uptime_ms as f32) * 100.0
    } else {
        0.0
    };
    
    let wakeup_rate = s.get_wakeup_rate();
    crate::serial_println!(
        "{},{},{:.2},{}",
        s.uptime_ms,
        s.avg_power_mw,
        c_residency_percent,
        wakeup_rate
    );
}

/// Suspend cycle statistics
struct SuspendCycle {
    cycle_id: u32,
    result: bool,  // true = success, false = failure
    resume_ms: u64,
    failures: u32,
}

static SUSPEND_CYCLES: Mutex<Vec<SuspendCycle>> = Mutex::new(Vec::new());

/// Record a suspend/resume cycle
pub fn record_suspend_cycle(cycle_id: u32, result: bool, resume_ms: u64, failures: u32) {
    let mut cycles = SUSPEND_CYCLES.lock();
    cycles.push(SuspendCycle {
        cycle_id,
        result,
        resume_ms,
        failures,
    });
}

/// Export suspend cycle statistics (suspend_cycles.csv)
pub fn export_suspend_cycles_csv() {
    let cycles = SUSPEND_CYCLES.lock();
    crate::serial_println!("cycle_id,result,resume_ms,failures");
    for cycle in cycles.iter() {
        let result_str = if cycle.result { "success" } else { "failure" };
        crate::serial_println!(
            "{},{},{},{}",
            cycle.cycle_id,
            result_str,
            cycle.resume_ms,
            cycle.failures
        );
    }
}


