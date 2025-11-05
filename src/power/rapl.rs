//! Intel RAPL MSR reading (best-effort)

use crate::power::PowerError;

const MSR_RAPL_POWER_UNIT: u32 = 0x606; // energy units in bits 8..12
const MSR_PKG_ENERGY_STATUS: u32 = 0x611;

#[inline]
fn msr_supported() -> bool {
    let eax: u32 = 1; // CPUID leaf 1
    let mut eax_out: u32 = 0;
    let mut ebx_out: u32 = 0;
    let mut ecx_out: u32 = 0;
    let mut edx_out: u32 = 0;
    unsafe {
        core::arch::asm!(
            "cpuid",
            in("eax") eax,
            out("eax") eax_out,
            out("ebx") ebx_out,
            out("ecx") ecx_out,
            out("edx") edx_out,
            options(nostack, preserves_flags)
        );
    }
    // EDX bit 5 indicates MSR support
    (edx_out & (1 << 5)) != 0
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

pub fn read_package_energy_status() -> Option<u32> {
    if !msr_supported() { return None; }
    unsafe {
        let val = read_msr(MSR_PKG_ENERGY_STATUS) as u32;
        Some(val)
    }
}

/// Read energy unit denominator (2^eu) where eu = bits 8..12 of MSR_RAPL_POWER_UNIT
/// Returns microjoules per LSB approximately, as a fixed-point scaling factor (in nanojoules per LSB)
pub fn read_energy_unit_nanojoules() -> Option<u64> {
    if !msr_supported() { return None; }
    unsafe {
        let units = read_msr(MSR_RAPL_POWER_UNIT);
        let eu = ((units >> 8) & 0x1F) as u32;
        // Energy unit = 1 / 2^eu Joules per LSB
        // Convert to nanojoules: 1e9 / 2^eu
        let nj_per_lsb: u64 = 1_000_000_000u64 >> eu;
        Some(nj_per_lsb)
    }
}

/// Read power unit (watts per LSB)
pub fn read_power_unit_watts() -> Option<f32> {
    if !msr_supported() { return None; }
    unsafe {
        let units = read_msr(MSR_RAPL_POWER_UNIT);
        let pu = ((units >> 0) & 0x0F) as u32; // Power unit in bits 0-3
        // Power unit = 1 / 2^pu Watts per LSB
        let watts_per_lsb = 1.0f32 / (1u32 << pu) as f32;
        Some(watts_per_lsb)
    }
}

/// RAPL Power Limit MSRs
const MSR_PKG_POWER_LIMIT: u32 = 0x610;
const MSR_PKG_ENERGY_STATUS: u32 = 0x611;

/// RAPL Power Limit 설정
/// 
/// # Arguments
/// * `power_limit_watts` - 전력 제한 (Watts)
/// * `time_window_sec` - 시간 윈도우 (초)
pub fn set_power_limit(power_limit_watts: f32, time_window_sec: u32) -> Result<(), PowerError> {
    if !msr_supported() {
        return Err(PowerError::Unsupported);
    }
    
    let power_unit = match read_power_unit_watts() {
        Some(unit) => unit,
        None => return Err(PowerError::Unsupported),
    };
    
    // Power limit in LSB units
    let power_limit_lsb = (power_limit_watts / power_unit) as u32;
    
    // Time window encoding (typically 1.0s units)
    let time_window_lsb = time_window_sec;
    
    unsafe {
        let mut limit = read_msr(MSR_PKG_POWER_LIMIT);
        
        // Set power limit 1 (bits 14:0 for power, bits 23:17 for time window)
        // Limit 1 enable bit (bit 15)
        limit &= !0x7FFF; // Clear power limit bits
        limit &= !0xFE0000; // Clear time window bits
        limit |= (power_limit_lsb as u64) & 0x7FFF;
        limit |= ((time_window_lsb as u64) & 0x7F) << 17;
        limit |= 1 << 15; // Enable limit 1
        
        write_msr(MSR_PKG_POWER_LIMIT, limit);
    }
    
    Ok(())
}

#[inline]
unsafe fn write_msr(msr: u32, value: u64) {
    let low: u32 = value as u32;
    let high: u32 = (value >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nostack, preserves_flags)
    );
}

/// 전력 측정 컨텍스트 (이전 측정값 저장)
static LAST_POWER_MEASUREMENT: Mutex<Option<PowerMeasurement>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
struct PowerMeasurement {
    energy_nj: u64,
    timestamp_ms: u64,
}

/// Read current power consumption from RAPL (Watts)
///
/// 이전 측정값과 비교하여 실제 전력 소비를 계산합니다.
pub fn read_power_watts(now_ms: u64) -> Option<f32> {
    if !msr_supported() { return None; }
    
    let energy_unit_nj = read_energy_unit_nanojoules()?;
    let energy_status = read_package_energy_status()?;
    
    // 에너지를 나노줄로 변환
    let current_energy_nj = (energy_status as u64).wrapping_mul(energy_unit_nj);
    
    let mut last_measurement = LAST_POWER_MEASUREMENT.lock();
    
    if let Some(last) = *last_measurement {
        let delta_energy_nj = current_energy_nj.wrapping_sub(last.energy_nj);
        let delta_time_ms = now_ms.saturating_sub(last.timestamp_ms);
        
        if delta_time_ms > 0 && delta_time_ms < 10000 {
            // 전력 = 에너지 / 시간
            // delta_energy_nj는 나노줄, delta_time_ms는 밀리초
            // 전력 (W) = (나노줄 / 1e9) / (밀리초 / 1000) = (나노줄 * 1000) / (밀리초 * 1e9)
            let power_watts = (delta_energy_nj as f32 * 1000.0) / (delta_time_ms as f32 * 1_000_000_000.0);
            
            // 측정값 업데이트
            *last_measurement = Some(PowerMeasurement {
                energy_nj: current_energy_nj,
                timestamp_ms: now_ms,
            });
            
            Some(power_watts)
        } else {
            // 시간 간격이 너무 크거나 작음 (래핑 또는 초기 측정)
            *last_measurement = Some(PowerMeasurement {
                energy_nj: current_energy_nj,
                timestamp_ms: now_ms,
            });
            None
        }
    } else {
        // 첫 측정
        *last_measurement = Some(PowerMeasurement {
            energy_nj: current_energy_nj,
            timestamp_ms: now_ms,
        });
        None
    }
}

/// RAPL 전력 측정 초기화
pub fn init_rapl_measurement() {
    let mut last_measurement = LAST_POWER_MEASUREMENT.lock();
    
    if msr_supported() {
        if let (Some(energy_unit_nj), Some(energy_status)) = (
            read_energy_unit_nanojoules(),
            read_package_energy_status(),
        ) {
            let current_energy_nj = (energy_status as u64).wrapping_mul(energy_unit_nj);
            *last_measurement = Some(PowerMeasurement {
                energy_nj: current_energy_nj,
                timestamp_ms: crate::drivers::timer::get_milliseconds(),
            });
            crate::log_info!("RAPL power measurement initialized");
        } else {
            crate::log_warn!("RAPL MSR not available on this CPU");
        }
    }
}


