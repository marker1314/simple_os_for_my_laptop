//! 시스템 콜 구현
//!
//! 각 시스템 콜의 실제 구현을 포함합니다.

use crate::syscall::{SyscallResult, SyscallError};
use crate::syscall::validation::{validate_buffer, validate_string};
use crate::drivers::{serial, timer, vga};
use crate::scheduler;

/// 시스템 콜: Exit
///
/// 현재 프로세스/스레드를 종료합니다.
///
/// # Arguments
/// * `exit_code` - 종료 코드
///
/// # Returns
/// 성공 시 0 (실제로는 반환되지 않음, 프로세스가 종료됨)
pub fn sys_exit(exit_code: u64) -> SyscallResult {
    crate::log_info!("Syscall: exit({})", exit_code);
    
    // 현재 스레드 종료
    if let Some(thread) = scheduler::current_thread() {
        let thread_id = {
            let t = thread.lock();
            t.id
        };
        scheduler::terminate_thread(thread_id);
        crate::log_info!("Thread {} terminated with exit code {}", thread_id, exit_code);
    }
    
    // TODO: 프로세스가 종료되면 다른 프로세스로 전환
    // 현재는 단일 스레드이므로 무한 루프로 대기
    Ok(0)
}

/// 시스템 콜: Write
///
/// 파일 디스크립터에 데이터를 씁니다.
///
/// # Arguments
/// * `fd` - 파일 디스크립터 (0: stdin, 1: stdout, 2: stderr)
/// * `buf` - 쓸 데이터의 포인터 (유저 공간)
/// * `count` - 쓸 바이트 수
///
/// # Returns
/// 실제로 쓴 바이트 수
pub fn sys_write(fd: u64, buf: u64, count: u64) -> SyscallResult {
    // 파일 디스크립터 검증
    if fd != 0 && fd != 1 && fd != 2 {
        crate::log_warn!("Syscall: write() called with invalid fd: {}", fd);
        return Err(SyscallError::InvalidArgument);
    }
    
    if count == 0 {
        return Ok(0);
    }
    
    // 버퍼 검증 (포인터 유효성 및 크기 검증)
    const MAX_WRITE_SIZE: usize = 1024;
    let (validated_ptr, validated_len) = match validate_buffer(buf, count, MAX_WRITE_SIZE) {
        Ok((ptr, len)) => (ptr, len),
        Err(e) => {
            crate::log_warn!("Syscall: write() buffer validation failed: {:?}", e);
            return Err(e);
        }
    };
    
    // 안전하게 버퍼 읽기
    // 실제로는 유저 공간에서 커널 공간으로 복사해야 하지만,
    // 현재는 커널 공간만 있으므로 직접 읽기
    unsafe {
        let ptr = validated_ptr as *const u8;
        let slice = core::slice::from_raw_parts(ptr, validated_len);
        
        // 시리얼 포트에 출력
        for &byte in slice {
            serial::write_byte(byte);
        }
        
        // VGA에 출력 (ASCII 문자만)
        for &byte in slice {
            if byte.is_ascii() && !byte.is_ascii_control() {
                vga::write_char(byte as char);
            } else if byte == b'\n' {
                vga::newline();
            }
        }
    }
    
    Ok(validated_len as u64)
}

/// 시스템 콜: Read
///
/// 파일 디스크립터에서 데이터를 읽습니다.
///
/// # Arguments
/// * `fd` - 파일 디스크립터 (0: stdin)
/// * `buf` - 데이터를 읽을 버퍼 포인터 (유저 공간)
/// * `count` - 읽을 최대 바이트 수
///
/// # Returns
/// 실제로 읽은 바이트 수
pub fn sys_read(fd: u64, buf: u64, count: u64) -> SyscallResult {
    // 파일 디스크립터 검증
    if fd != 0 {
        crate::log_warn!("Syscall: read() called with invalid fd: {}", fd);
        return Err(SyscallError::InvalidArgument);
    }
    
    if count == 0 {
        return Ok(0);
    }
    
    // 버퍼 검증
    const MAX_READ_SIZE: usize = 1024;
    let (validated_ptr, validated_len) = match validate_buffer(buf, count, MAX_READ_SIZE) {
        Ok((ptr, len)) => (ptr, len),
        Err(e) => {
            crate::log_warn!("Syscall: read() buffer validation failed: {:?}", e);
            return Err(e);
        }
    };
    
    // stdin: 키보드 입력
    // TODO: 키보드 입력 큐에서 읽기 구현
    // 현재는 키보드 드라이버가 있지만 입력 큐가 없음
    crate::log_debug!("Syscall: read() from stdin (not implemented yet)");
    
    // 실제로는 validated_ptr에 데이터를 쓰지만, 현재는 구현 없음
    Ok(0) // 현재는 항상 0 바이트 반환
}

/// 시스템 콜: Yield
///
/// 현재 스레드가 CPU를 양보하고 다른 스레드에게 실행 권한을 넘깁니다.
///
/// # Returns
/// 항상 0 (성공)
pub fn sys_yield() -> SyscallResult {
    crate::log_debug!("Syscall: yield()");
    
    // 스케줄러에 양보 신호 전달
    // TODO: 실제 컨텍스트 스위칭 구현
    // 현재는 로그만 출력
    Ok(0)
}

/// 시스템 콜: Sleep
///
/// 지정된 시간(밀리초) 동안 대기합니다.
///
/// # Arguments
/// * `milliseconds` - 대기할 시간 (밀리초)
///
/// # Returns
/// 남은 밀리초 (일반적으로 0)
pub fn sys_sleep(milliseconds: u64) -> SyscallResult {
    crate::log_debug!("Syscall: sleep({} ms)", milliseconds);
    
    if milliseconds == 0 {
        return Ok(0);
    }
    
    // 현재 시간 기록
    let start_time = timer::get_milliseconds();
    let target_time = start_time + milliseconds;
    
    // 목표 시간까지 대기
    // TODO: 스레드를 블로킹하고 타이머 인터럽트에서 깨우기
    // 현재는 busy-wait (실제 구현에서는 스레드 블로킹 사용)
    while timer::get_milliseconds() < target_time {
        // CPU 양보
        x86_64::instructions::hlt();
    }
    
    Ok(0)
}

/// 시스템 콜: GetTime
///
/// 부팅 이후 경과한 시간을 밀리초 단위로 반환합니다.
///
/// # Returns
/// 부팅 이후 경과한 밀리초
pub fn sys_get_time() -> SyscallResult {
    let time = timer::get_milliseconds();
    Ok(time)
}

/// 시스템 콜: GetPid
///
/// 현재 프로세스/스레드 ID를 반환합니다.
///
/// # Returns
/// 현재 프로세스/스레드 ID
pub fn sys_get_pid() -> SyscallResult {
    if let Some(thread) = scheduler::current_thread() {
        let pid = {
            let t = thread.lock();
            t.id
        };
        Ok(pid)
    } else {
        Ok(0) // 커널 스레드인 경우 0 반환
    }
}

