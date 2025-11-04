//! 컨텍스트 스위칭 구현
//!
//! 이 모듈은 CPU 컨텍스트 스위칭을 처리합니다.
//! 레지스터를 저장하고 복원하여 스레드 간 전환을 수행합니다.
//!
//! 현재는 기본 구조만 구현되어 있으며, 실제 컨텍스트 스위칭은
//! 향후 멀티스레딩이 필요할 때 완전히 구현될 예정입니다.

use crate::scheduler::thread::ThreadContext;

/// 컨텍스트 스위칭 함수
///
/// 현재 스레드의 컨텍스트를 저장하고 다음 스레드의 컨텍스트를 복원합니다.
///
/// # Arguments
/// * `from` - 현재 스레드의 컨텍스트를 저장할 포인터
/// * `to` - 다음 스레드의 컨텍스트를 복원할 포인터
///
/// # Safety
/// 이 함수는 unsafe입니다. 올바른 컨텍스트 포인터를 전달해야 합니다.
///
/// # Note
/// 현재는 기본 구조만 구현되어 있습니다. 실제 컨텍스트 스위칭은
/// 향후 멀티스레딩이 필요할 때 완전히 구현될 예정입니다.
#[no_mangle]
pub unsafe extern "C" fn context_switch(_from: *mut ThreadContext, _to: *const ThreadContext) {
    // TODO: 실제 컨텍스트 스위칭 구현
    // 현재는 스케줄러가 준비되었지만 실제 컨텍스트 스위칭은
    // 멀티스레딩이 필요할 때 구현됩니다.
}

/// 컨텍스트 저장 함수
///
/// 현재 CPU 상태를 컨텍스트에 저장합니다.
///
/// # Arguments
/// * `ctx` - 컨텍스트를 저장할 포인터
/// * `entry_point` - 진입점 주소
/// * `stack_pointer` - 스택 포인터
///
/// # Safety
/// 이 함수는 unsafe입니다. 올바른 컨텍스트 포인터를 전달해야 합니다.
pub unsafe fn save_context(ctx: *mut ThreadContext, entry_point: u64, stack_pointer: u64) {
    (*ctx).rip = entry_point;
    (*ctx).rsp = stack_pointer;
    // 나머지 레지스터는 컨텍스트 스위칭 시 저장됨
}

