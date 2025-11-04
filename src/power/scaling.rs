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
}

impl CpuScaling {
    /// 새 CPU 스케일링 관리자 생성
    pub fn new() -> Self {
        Self {
            initialized: false,
            current_p_state: 0,
            supported: true,
        }
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
    let mut eax: u32;
    let mut ebx: u32;
    let mut ecx: u32;
    let mut edx: u32;
    unsafe {
        core::arch::asm!(
            "cpuid",
            inlateout("eax") 0u32 => eax,
            out("ebx") ebx,
            out("ecx") ecx,
            out("edx") edx,
            options(nostack, preserves_flags)
        );
    }
    let mut vendor_bytes = [0u8; 12];
    vendor_bytes[0..4].copy_from_slice(&ebx.to_le_bytes());
    vendor_bytes[4..8].copy_from_slice(&edx.to_le_bytes());
    vendor_bytes[8..12].copy_from_slice(&ecx.to_le_bytes());
    let vendor = match core::str::from_utf8(&vendor_bytes).unwrap_or("") {
        "GenuineIntel" => Vendor::Intel,
        "AuthenticAMD" => Vendor::Amd,
        _ => Vendor::Other,
    };

    // Leaf 1 for feature flags
    let mut eax1: u32 = 1;
    let mut ebx1: u32;
    let mut ecx1: u32;
    let mut edx1: u32;
    unsafe {
        core::arch::asm!(
            "cpuid",
            inlateout("eax") eax1 => eax1,
            out("ebx") ebx1,
            out("ecx") ecx1,
            out("edx") edx1,
            options(nostack, preserves_flags)
        );
    }
    // ECX bit 7 = EST (Intel Enhanced SpeedStep Technology) on Intel
    let est = (ecx1 & (1 << 7)) != 0;
    (vendor, est)
}

