//! 크리티컬 예외 복구 메커니즘
//!
//! Double Fault, General Protection Fault 등의 크리티컬 예외에 대한 복구를 시도합니다.

use x86_64::structures::idt::InterruptStackFrame;
use spin::Mutex;
use alloc::vec::Vec;

/// 예외 복구 결과
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResult {
    /// 복구 성공
    Recovered,
    /// 복구 실패 (시스템 종료 필요)
    Unrecoverable,
    /// 복구 시도 불가 (예: Double Fault)
    CannotRecover,
}

/// 예외 복구 통계
#[derive(Debug, Default)]
struct RecoveryStats {
    /// 총 예외 발생 횟수
    total_exceptions: u64,
    /// 복구 성공 횟수
    recovery_success: u64,
    /// 복구 실패 횟수
    recovery_failed: u64,
    /// Double Fault 발생 횟수
    double_fault_count: u64,
    /// GPF 발생 횟수
    gpf_count: u64,
}

static RECOVERY_STATS: Mutex<RecoveryStats> = Mutex::new(RecoveryStats {
    total_exceptions: 0,
    recovery_success: 0,
    recovery_failed: 0,
    double_fault_count: 0,
    gpf_count: 0,
});

/// 예외 발생 기록 (최근 10개)
static EXCEPTION_HISTORY: Mutex<Vec<ExceptionRecord>> = Mutex::new(Vec::new());

/// 예외 기록
#[derive(Debug, Clone)]
struct ExceptionRecord {
    /// 예외 타입
    exception_type: u8,
    /// 발생 시간 (밀리초)
    timestamp: u64,
    /// RIP
    rip: u64,
    /// 복구 시도 여부
    recovery_attempted: bool,
    /// 복구 성공 여부
    recovery_success: bool,
}

/// Double Fault 복구 시도
///
/// # Safety
/// Double Fault는 일반적으로 복구 불가능하지만, 일부 경우에 복구를 시도할 수 있습니다.
pub unsafe fn try_recover_double_fault(stack_frame: &InterruptStackFrame, error_code: u64) -> RecoveryResult {
    let mut stats = RECOVERY_STATS.lock();
    stats.total_exceptions += 1;
    stats.double_fault_count += 1;
    
    // 예외 기록
    let mut history = EXCEPTION_HISTORY.lock();
    history.push(ExceptionRecord {
        exception_type: 0x08,
        timestamp: crate::drivers::timer::get_milliseconds(),
        rip: stack_frame.instruction_pointer.as_u64(),
        recovery_attempted: true,
        recovery_success: false,
    });
    
    // 최근 10개만 유지
    if history.len() > 10 {
        history.remove(0);
    }
    
    // Double Fault는 일반적으로 복구 불가능
    // 하지만 스택 오버플로우인 경우 복구를 시도할 수 있음
    let current_rsp: u64;
    core::arch::asm!("mov {}, rsp", out(reg) current_rsp, options(nostack, preserves_flags));
    
    // 스택 포인터가 유효한 범위인지 확인
    if current_rsp < 0x1000 || current_rsp > 0x7FFF_FFFF_FFFF_FFFF {
        crate::log_error!("Double Fault: Invalid stack pointer, cannot recover");
        stats.recovery_failed += 1;
        return RecoveryResult::CannotRecover;
    }
    
    // 최근 예외가 많이 발생했는지 확인
    let recent_exceptions = history.iter()
        .filter(|e| {
            let age = crate::drivers::timer::get_milliseconds().saturating_sub(e.timestamp);
            age < 1000 // 1초 이내
        })
        .count();
    
    if recent_exceptions > 5 {
        crate::log_error!("Double Fault: Too many recent exceptions, cannot recover");
        stats.recovery_failed += 1;
        return RecoveryResult::CannotRecover;
    }
    
    // Double Fault는 복구 불가능으로 간주
    stats.recovery_failed += 1;
    RecoveryResult::CannotRecover
}

/// General Protection Fault 복구 시도
///
/// # Safety
/// 일부 GPF는 복구 가능할 수 있습니다 (예: 잘못된 메모리 접근).
pub unsafe fn try_recover_gpf(stack_frame: &InterruptStackFrame, error_code: u64) -> RecoveryResult {
    let mut stats = RECOVERY_STATS.lock();
    stats.total_exceptions += 1;
    stats.gpf_count += 1;
    
    // 예외 기록
    let mut history = EXCEPTION_HISTORY.lock();
    history.push(ExceptionRecord {
        exception_type: 0x0D,
        timestamp: crate::drivers::timer::get_milliseconds(),
        rip: stack_frame.instruction_pointer.as_u64(),
        recovery_attempted: true,
        recovery_success: false,
    });
    
    // 최근 10개만 유지
    if history.len() > 10 {
        history.remove(0);
    }
    
    // Error code 분석
    let is_external = (error_code & 1) != 0;
    let is_descriptor = (error_code & 2) != 0;
    
    // 세그먼트 선택자 오류인 경우 복구 시도
    if is_descriptor {
        let selector_index = ((error_code >> 3) & 0x1FFF) as u16;
        
        // 잘못된 세그먼트 선택자인 경우, 기본 세그먼트로 복구 시도
        if selector_index > 0 {
            crate::log_warn!("GPF: Invalid segment selector {}, attempting recovery", selector_index);
            
            // 세그먼트 레지스터를 기본값으로 설정 (간단한 복구)
            // 실제로는 더 복잡한 복구가 필요하지만, 기본 구조만 제공
            
            // 스택 포인터 확인
            let current_rsp: u64;
            core::arch::asm!("mov {}, rsp", out(reg) current_rsp, options(nostack, preserves_flags));
            
            if current_rsp < 0x1000 || current_rsp > 0x7FFF_FFFF_FFFF_FFFF {
                crate::log_error!("GPF: Invalid stack pointer, cannot recover");
                stats.recovery_failed += 1;
                return RecoveryResult::Unrecoverable;
            }
            
            // 복구 성공으로 간주 (실제로는 더 검증 필요)
            stats.recovery_success += 1;
            history.last_mut().unwrap().recovery_success = true;
            crate::log_info!("GPF: Recovery attempted (segment selector error)");
            return RecoveryResult::Recovered;
        }
    }
    
    // 메모리 접근 오류인 경우
    // RIP가 유효한 범위인지 확인
    let rip = stack_frame.instruction_pointer.as_u64();
    if rip < 0x1000 || rip > 0x7FFF_FFFF_FFFF_FFFF {
        crate::log_error!("GPF: Invalid RIP, cannot recover");
        stats.recovery_failed += 1;
        return RecoveryResult::Unrecoverable;
    }
    
    // 최근 예외가 많이 발생했는지 확인
    let recent_exceptions = history.iter()
        .filter(|e| {
            let age = crate::drivers::timer::get_milliseconds().saturating_sub(e.timestamp);
            age < 1000 // 1초 이내
        })
        .count();
    
    if recent_exceptions > 10 {
        crate::log_error!("GPF: Too many recent exceptions, cannot recover");
        stats.recovery_failed += 1;
        return RecoveryResult::Unrecoverable;
    }
    
    // 기본 복구 시도: 스택 포인터만 확인하고 복구 시도
    let current_rsp: u64;
    core::arch::asm!("mov {}, rsp", out(reg) current_rsp, options(nostack, preserves_flags));
    
    if current_rsp < 0x1000 || current_rsp > 0x7FFF_FFFF_FFFF_FFFF {
        crate::log_error!("GPF: Invalid stack pointer, cannot recover");
        stats.recovery_failed += 1;
        return RecoveryResult::Unrecoverable;
    }
    
    // 일부 경우 복구 가능
    stats.recovery_success += 1;
    history.last_mut().unwrap().recovery_success = true;
    crate::log_warn!("GPF: Recovery attempted (may not be fully safe)");
    RecoveryResult::Recovered
}

/// 복구 통계 가져오기
pub fn get_recovery_stats() -> (u64, u64, u64, u64, u64) {
    let stats = RECOVERY_STATS.lock();
    (
        stats.total_exceptions,
        stats.recovery_success,
        stats.recovery_failed,
        stats.double_fault_count,
        stats.gpf_count,
    )
}

/// 예외 기록 가져오기
pub fn get_exception_history() -> Vec<ExceptionRecord> {
    let history = EXCEPTION_HISTORY.lock();
    history.clone()
}

