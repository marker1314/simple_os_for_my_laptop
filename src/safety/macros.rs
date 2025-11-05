//! Unsafe 블록 검증 매크로
//!
//! unsafe 블록을 더 안전하게 사용하기 위한 매크로를 제공합니다.

/// Unsafe 블록을 검증하고 기록하는 매크로
/// 
/// # 사용 예시
/// ```rust,ignore
/// unsafe_checked! {
///     type: HardwareAccess,
///     desc: "MMIO 레지스터 읽기",
///     validate: {
///         // 검증 로직
///     },
///     block: {
///         // 실제 unsafe 코드
///     }
/// }
/// ```
#[macro_export]
macro_rules! unsafe_checked {
    (
        type: $type:expr,
        desc: $desc:expr,
        validate: $validate:block,
        block: $block:block
    ) => {{
        let ctx = $crate::safety::UnsafeContext::new(
            $type,
            file!(),
            line!(),
            $desc,
        );
        
        // 검증 실행
        let validation_result = $validate;
        let validated = validation_result.is_ok();
        
        if let Err(e) = validation_result {
            $crate::log_warn!("Unsafe block validation failed at {}:{} - {}: {:?}", 
                            file!(), line!(), $desc, e);
        }
        
        // 통계 기록
        $crate::safety::record_unsafe_block($type, validated);
        
        // unsafe 블록 실행
        unsafe {
            $block
        }
    }};
}

/// 포인터 검증 매크로
/// 
/// # 사용 예시
/// ```rust,ignore
/// let ptr = unsafe_validate_ptr!(ptr_value, size, {
///     // 검증된 포인터 사용
/// });
/// ```
#[macro_export]
macro_rules! unsafe_validate_ptr {
    ($ptr:expr, $size:expr, $block:block) => {{
        let validator = $crate::safety::PointerValidator::new($ptr, $size);
        match unsafe { validator.validate() } {
            Ok(()) => {
                unsafe {
                    $block
                }
            }
            Err(e) => {
                $crate::log_error!("Pointer validation failed: {:?}", e);
                Err(e)
            }
        }
    }};
}

/// 하드웨어 접근 검증 매크로
/// 
/// # 사용 예시
/// ```rust,ignore
/// let value = unsafe_validate_hw!(phys_addr, size, {
///     // 검증된 하드웨어 접근
/// });
/// ```
#[macro_export]
macro_rules! unsafe_validate_hw {
    ($phys_addr:expr, $size:expr, $block:block) => {{
        let validator = $crate::safety::HardwareValidator::new($phys_addr, $size);
        match unsafe { validator.validate() } {
            Ok(()) => {
                unsafe {
                    $block
                }
            }
            Err(e) => {
                $crate::log_error!("Hardware validation failed: {:?}", e);
                Err(e)
            }
        }
    }};
}

