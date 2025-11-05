//! 시스템 콜 파라미터 검증
//!
//! 포인터 유효성, 버퍼 크기 등의 검증을 수행합니다.

use crate::syscall::SyscallError;

/// 포인터 유효성 검사
/// 
/// 포인터가 유효한 메모리 영역을 가리키는지 확인합니다.
/// 
/// # Arguments
/// * `ptr` - 검사할 포인터
/// * `size` - 접근할 크기 (바이트)
/// 
/// # Returns
/// 유효하면 Ok(()), 그렇지 않으면 에러
pub fn validate_pointer(ptr: u64, size: usize) -> Result<(), SyscallError> {
    // Null 포인터 검사
    if ptr == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // 커널 공간 접근 방지 (0xFFFF800000000000 이상)
    let kernel_base = 0xFFFF800000000000u64;
    if ptr >= kernel_base {
        crate::log_warn!("Syscall: Attempt to access kernel space at {:#016x}", ptr);
        return Err(SyscallError::PermissionDenied);
    }
    
    // 포인터 범위 검사 (오버플로우 방지)
    let end_ptr = match ptr.checked_add(size as u64) {
        Some(end) => end,
        None => {
            crate::log_warn!("Syscall: Pointer overflow: ptr={:#016x}, size={}", ptr, size);
            return Err(SyscallError::InvalidArgument);
        }
    };
    
    // 커널 공간 접근 방지 (end 포인터도)
    if end_ptr >= kernel_base {
        crate::log_warn!("Syscall: Buffer extends into kernel space: ptr={:#016x}, size={}", ptr, size);
        return Err(SyscallError::PermissionDenied);
    }
    
    // 유효한 사용자 공간 범위 확인 (0x0000000000000000 ~ 0x00007FFFFFFFFFFF)
    let user_space_end = 0x00007FFFFFFFFFFFu64;
    if ptr > user_space_end || end_ptr > user_space_end {
        crate::log_warn!("Syscall: Pointer outside user space: ptr={:#016x}, end={:#016x}", ptr, end_ptr);
        return Err(SyscallError::InvalidArgument);
    }
    
    Ok(())
}

/// 버퍼 크기 검증
/// 
/// 버퍼 크기가 합리적인 범위 내에 있는지 확인합니다.
/// 
/// # Arguments
/// * `size` - 버퍼 크기 (바이트)
/// * `max_size` - 최대 허용 크기 (바이트)
/// 
/// # Returns
/// 유효하면 Ok(size), 그렇지 않으면 에러
pub fn validate_buffer_size(size: u64, max_size: usize) -> Result<usize, SyscallError> {
    // 0 크기는 허용
    if size == 0 {
        return Ok(0);
    }
    
    // 최대 크기 제한
    if size > max_size as u64 {
        crate::log_warn!("Syscall: Buffer size too large: {} > {}", size, max_size);
        return Err(SyscallError::InvalidArgument);
    }
    
    // usize 변환 (안전성 검사)
    let size_usize = size as usize;
    if size_usize as u64 != size {
        crate::log_warn!("Syscall: Buffer size overflow: {}", size);
        return Err(SyscallError::InvalidArgument);
    }
    
    Ok(size_usize)
}

/// 포인터와 크기로 버퍼 검증
/// 
/// 포인터 유효성과 버퍼 크기를 함께 검증합니다.
/// 
/// # Arguments
/// * `ptr` - 버퍼 포인터
/// * `size` - 버퍼 크기 (바이트)
/// * `max_size` - 최대 허용 크기 (바이트)
/// 
/// # Returns
/// 유효하면 Ok((ptr, size)), 그렇지 않으면 에러
pub fn validate_buffer(ptr: u64, size: u64, max_size: usize) -> Result<(u64, usize), SyscallError> {
    let validated_size = validate_buffer_size(size, max_size)?;
    validate_pointer(ptr, validated_size)?;
    Ok((ptr, validated_size))
}

/// 문자열 검증
/// 
/// Null-terminated 문자열의 유효성을 검증합니다.
/// 
/// # Arguments
/// * `ptr` - 문자열 포인터
/// * `max_len` - 최대 문자열 길이 (Null 포함)
/// 
/// # Returns
/// 유효하면 Ok(()), 그렇지 않으면 에러
pub fn validate_string(ptr: u64, max_len: usize) -> Result<(), SyscallError> {
    // 포인터 유효성 검사
    validate_pointer(ptr, max_len)?;
    
    // 실제로는 문자열을 읽어서 Null-terminated인지 확인해야 하지만,
    // 현재는 포인터와 크기만 검증
    Ok(())
}

