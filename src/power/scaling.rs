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
        // MSR 접근 가능 여부 확인
        // 보수적으로 지원 플래그만 설정 (실제 모델별 세부 감지는 TODO)
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // EFER 읽기가 가능하면 대체로 MSR 접근 가능
            let _ = Efer::read();
            self.supported = true;
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        {
            self.supported = false;
        }
        
        // 기본 초기화 (실제 MSR 접근은 CPU 모델에 따라 다름)
        self.initialized = true;
        self.current_p_state = 0; // 최고 성능 상태
        
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

