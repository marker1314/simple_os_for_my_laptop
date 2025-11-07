//! CPU 클럭 스케일링
//!
//! CPU 클럭 속도를 동적으로 조절하여 전력을 관리합니다.

use crate::power::PowerError;
use x86_64::registers::model_specific::{Efer, EferFlags};

// MSR registers for performance control/status
const IA32_PERF_STATUS: u32 = 0x198;
const IA32_PERF_CTL: u32 = 0x199;

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

/// CPU 스케일링 Governor 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingGovernor {
    /// On-demand: CPU 사용률에 따라 동적 조정
    Ondemand,
    /// Power-save: 최소 주파수 유지
    Powersave,
    /// Performance: 최대 주파수 유지
    Performance,
}

/// CPU 스케일링 관리자
///
/// CPU 클럭 속도를 제어합니다.
pub struct CpuScaling {
    /// 초기화 여부
    initialized: bool,
    /// 현재 P-State (Performance State)
    current_p_state: u8,
    /// 제어 지원 여부 (CPUID/MSR 가드)
    supported: bool,
    /// 현재 Governor
    governor: ScalingGovernor,
    /// Hysteresis: 이전 CPU 사용률 (빈번한 스위칭 방지)
    last_cpu_usage: u8,
    /// On-demand 업데이트 간격 (ms)
    update_interval_ms: u64,
    /// 마지막 업데이트 시간 (ms)
    last_update_time_ms: u64,
}

impl CpuScaling {
    /// 새 CPU 스케일링 관리자 생성
    pub fn new() -> Self {
        Self {
            initialized: false,
            current_p_state: 0,
            supported: true,
            governor: ScalingGovernor::Ondemand,
            last_cpu_usage: 50,
            update_interval_ms: 100, // 100ms마다 업데이트
            last_update_time_ms: 0,
        }
    }
    
    /// Governor 설정
    pub fn set_governor(&mut self, governor: ScalingGovernor) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        
        self.governor = governor;
        
        // Governor에 따라 즉시 적용
        match governor {
            ScalingGovernor::Performance => self.set_max_performance()?,
            ScalingGovernor::Powersave => self.set_power_saving()?,
            ScalingGovernor::Ondemand => {
                // On-demand는 현재 사용률에 따라 설정
                self.set_balanced()?;
            }
        }
        
        Ok(())
    }
    
    /// 현재 Governor 가져오기
    pub fn get_governor(&self) -> ScalingGovernor {
        self.governor
    }
    
    /// On-demand governor 업데이트 (hysteresis 포함)
    /// 
    /// # Arguments
    /// * `cpu_usage_percent` - 현재 CPU 사용률 (0-100)
    /// * `now_ms` - 현재 시간 (ms)
    pub fn update_ondemand(&mut self, cpu_usage_percent: u8, now_ms: u64) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        
        if self.governor != ScalingGovernor::Ondemand {
            return Ok(()); // On-demand가 아니면 업데이트 안 함
        }
        
        // 업데이트 간격 체크
        if now_ms - self.last_update_time_ms < self.update_interval_ms {
            return Ok(());
        }
        
        self.last_update_time_ms = now_ms;
        
        // Hysteresis: 임계값에 약간의 여유를 두어 빈번한 스위칭 방지
        const HYSTERESIS_THRESHOLD: u8 = 5; // 5% 여유
        
        let target_p_state = if cpu_usage_percent > 80 + HYSTERESIS_THRESHOLD {
            // 높은 사용률: 성능 모드
            if self.last_cpu_usage <= 80 {
                // 이전에 낮은 사용률이었으면 즉시 전환
                0
            } else if cpu_usage_percent > 90 {
                // 매우 높은 사용률이면 즉시 전환
                0
            } else {
                // 점진적 증가
                self.current_p_state.saturating_sub(1)
            }
        } else if cpu_usage_percent < 20u8.saturating_sub(HYSTERESIS_THRESHOLD) {
            // 낮은 사용률: 절전 모드
            if self.last_cpu_usage >= 20 {
                // 이전에 높은 사용률이었으면 즉시 전환
                2
            } else if cpu_usage_percent < 10 {
                // 매우 낮은 사용률이면 즉시 전환
                2
            } else {
                // 점진적 감소
                self.current_p_state.saturating_add(1).min(2)
            }
        } else {
            // 중간 사용률: 균형 모드
            if self.last_cpu_usage < 20 || self.last_cpu_usage > 80 {
                // 극단에서 중간으로 전환
                1
            } else {
                // 유지
                self.current_p_state
            }
        };
        
        // P-state 변경이 필요하면 적용
        if target_p_state != self.current_p_state {
            match target_p_state {
                0 => self.set_max_performance()?,
                1 => self.set_balanced()?,
                2 => self.set_power_saving()?,
                _ => {}
            }
        }
        
        self.last_cpu_usage = cpu_usage_percent;
        Ok(())
    }
    
    /// CPU 스케일링 초기화
    ///
    /// MSR (Model Specific Register) 접근 가능 여부를 확인합니다.
    ///
    /// # Safety
    /// 이 함수는 한 번만 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), PowerError> {
        // Conservative detection: Only enable on GenuineIntel with EST feature bit
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            let (vendor, est) = cpuid_vendor_and_est();
            self.supported = vendor == Vendor::Intel && est;
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        { self.supported = false; }

        self.initialized = true;
        self.current_p_state = 0; // denote performance
        Ok(())
    }
    
    /// 최대 성능 모드 설정
    ///
    /// CPU 클럭을 최대로 설정합니다.
    pub fn set_max_performance(&mut self) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        if !self.supported {
            return Err(PowerError::Unsupported);
        }
        // Best effort: drive target ratio to current status' max observed ratio
        unsafe {
            let status = read_msr(IA32_PERF_STATUS);
            let current_ratio: u16 = (status & 0xFF00) as u16 >> 8; // typical encoding
            let target_ratio = if current_ratio == 0 { 0x20 } else { current_ratio };
            let mut ctl = read_msr(IA32_PERF_CTL);
            // clear target ratio bits [15:8] then set
            ctl &= !0xFF00u64;
            ctl |= ((target_ratio as u64) & 0xFF) << 8;
            write_msr(IA32_PERF_CTL, ctl);
        }
        self.current_p_state = 0; // denote performance
        Ok(())
    }
    
    /// 균형 모드 설정
    ///
    /// CPU 클럭을 중간 수준으로 설정합니다.
    pub fn set_balanced(&mut self) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        if !self.supported {
            return Err(PowerError::Unsupported);
        }
        unsafe {
            let status = read_msr(IA32_PERF_STATUS);
            let cur: u16 = (status & 0xFF00) as u16 >> 8;
            let max = if cur == 0 { 0x20 } else { cur };
            let balanced = core::cmp::max(0x08, max / 2);
            let mut ctl = read_msr(IA32_PERF_CTL);
            ctl &= !0xFF00u64;
            ctl |= ((balanced as u64) & 0xFF) << 8;
            write_msr(IA32_PERF_CTL, ctl);
        }
        self.current_p_state = 1;
        Ok(())
    }
    
    /// 전력 절약 모드 설정
    ///
    /// CPU 클럭을 낮춰 전력을 절약합니다.
    pub fn set_power_saving(&mut self) -> Result<(), PowerError> {
        if !self.initialized {
            return Err(PowerError::NotInitialized);
        }
        if !self.supported {
            return Err(PowerError::Unsupported);
        }
        unsafe {
            // Choose a conservative low ratio
            let low_ratio: u16 = 0x08; // safe floor on many systems
            let mut ctl = read_msr(IA32_PERF_CTL);
            ctl &= !0xFF00u64;
            ctl |= ((low_ratio as u64) & 0xFF) << 8;
            write_msr(IA32_PERF_CTL, ctl);
        }
        self.current_p_state = 2;
        Ok(())
    }
    
    /// 현재 P-State 가져오기
    pub fn get_current_p_state(&self) -> u8 {
        self.current_p_state
    }
}

#[derive(PartialEq, Eq)]
enum Vendor { Intel, Amd, Other }

#[inline]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn cpuid_vendor_and_est() -> (Vendor, bool) {
    let v0 = unsafe { core::arch::x86_64::__cpuid(0) };
    let mut vendor_bytes = [0u8; 12];
    vendor_bytes[0..4].copy_from_slice(&v0.ebx.to_le_bytes());
    vendor_bytes[4..8].copy_from_slice(&v0.edx.to_le_bytes());
    vendor_bytes[8..12].copy_from_slice(&v0.ecx.to_le_bytes());
    let vendor = match core::str::from_utf8(&vendor_bytes).unwrap_or("") {
        "GenuineIntel" => Vendor::Intel,
        "AuthenticAMD" => Vendor::Amd,
        _ => Vendor::Other,
    };

    let v1 = unsafe { core::arch::x86_64::__cpuid(1) };
    let est = (v1.ecx & (1 << 7)) != 0;
    (vendor, est)
}

