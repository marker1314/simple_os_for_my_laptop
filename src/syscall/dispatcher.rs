//! 시스템 콜 디스패처
//!
//! 시스템 콜 번호에 따라 적절한 핸들러를 호출합니다.

use crate::syscall::SyscallError;
use crate::syscall::numbers::SyscallNumber;
use crate::syscall::implementations;

/// 시스템 콜 디스패치
///
/// 시스템 콜 번호에 따라 적절한 핸들러를 호출합니다.
///
/// # Arguments
/// * `syscall_num` - 시스템 콜 번호 (RAX 레지스터)
/// * `arg1` ~ `arg6` - 시스템 콜 파라미터 (RDI, RSI, RDX, R10, R8, R9)
///
/// # Returns
/// 시스템 콜 결과 (성공 시 값, 실패 시 에러 코드)
pub fn dispatch_syscall(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> i64 {
    // 시스템 콜 번호 검증
    let syscall = match SyscallNumber::from_u64(syscall_num) {
        Some(s) => s,
        None => {
            crate::log_warn!("Invalid syscall number: {}", syscall_num);
            return SyscallError::InvalidSyscall.as_i64();
        }
    };
    
    // 시스템 콜 메트릭 기록
    crate::monitoring::record_syscall();
    
    // 시스템 콜 디버그 로그
    crate::log_debug!(
        "Syscall: {:?} (args: {:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x})",
        syscall, arg1, arg2, arg3, arg4, arg5, arg6
    );
    
    // 시스템 콜 번호에 따라 핸들러 호출
    let result = match syscall {
        SyscallNumber::Exit => {
            implementations::sys_exit(arg1)
        }
        SyscallNumber::Write => {
            implementations::sys_write(arg1, arg2, arg3)
        }
        SyscallNumber::Read => {
            implementations::sys_read(arg1, arg2, arg3)
        }
        SyscallNumber::Yield => {
            implementations::sys_yield()
        }
        SyscallNumber::Sleep => {
            implementations::sys_sleep(arg1)
        }
        SyscallNumber::GetTime => {
            implementations::sys_get_time()
        }
        SyscallNumber::GetPid => {
            implementations::sys_get_pid()
        }
    };
    
    // 결과를 i64로 변환
    match result {
        Ok(value) => value as i64,
        Err(err) => err.as_i64(),
    }
}

