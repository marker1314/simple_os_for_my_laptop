//! Power statistics collection (stub)

use spin::Mutex;

#[derive(Default, Clone, Copy)]
pub struct PowerStatistics {
    pub avg_power_mw: u32,
    pub peak_power_mw: u32,
    pub energy_consumed_mj: u64,
    pub p_state_histogram: [u32; 16],
    pub c_state_histogram: [u32; 8],
    pub uptime_ms: u64,
    last_rapl_nj: Option<u64>,
}

impl PowerStatistics {
    pub fn record_sample(&mut self, _instant_power_mw: u32) {
        // TODO: accumulate moving average and histograms
    }

    pub fn calculate_average_power(&self) -> u32 {
        self.avg_power_mw
    }

    pub fn print_report(&self) {
        crate::log_info!(
            "Power: avg={}mW peak={}mW energy={}mJ",
            self.avg_power_mw,
            self.peak_power_mw,
            self.energy_consumed_mj
        );
    }
}

static STATS: Mutex<PowerStatistics> = Mutex::new(PowerStatistics::default());

/// Periodic tick to accumulate stats and occasionally print a report
pub fn tick(now_ms: u64) {
    let mut s = STATS.lock();
    s.uptime_ms = now_ms;
    // Accumulate package energy from RAPL if available
    if let (Some(lsb_nj), Some(raw)) = (
        crate::power::rapl::read_energy_unit_nanojoules(),
        crate::power::rapl::read_package_energy_status(),
    ) {
        let current_nj = (raw as u64) * lsb_nj;
        if let Some(last) = s.last_rapl_nj {
            let delta = current_nj.wrapping_sub(last);
            // Convert nanojoules to millijoules and accumulate
            s.energy_consumed_mj = s.energy_consumed_mj.saturating_add(delta / 1_000_000);
        }
        s.last_rapl_nj = Some(current_nj);
    }
    if now_ms % 10_000 == 0 {
        s.print_report();
    }
}


