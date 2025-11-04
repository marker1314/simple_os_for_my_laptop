//! 시스템 콜 핸들러 초기화
//!
//! IDT에 시스템 콜 인터럽트 핸들러를 등록합니다.

use crate::interrupts::idt;
use crate::syscall::syscall_handler;

/// 시스템 콜 인터럽트 번호
/// x86_64에서 일반적으로 사용하는 인터럽트 번호
pub const SYSCALL_INTERRUPT: u8 = 0x80;

/// 시스템 콜 핸들러 초기화
///
/// IDT에 시스템 콜 인터럽트 핸들러를 등록합니다.
/// 이 함수는 커널 초기화 시 한 번 호출되어야 합니다.
pub fn init_syscall_handler() {
    unsafe {
        // IDT에 시스템 콜 핸들러 등록
        // idt 모듈의 IDT에 직접 접근하여 등록
        idt::register_syscall_handler(SYSCALL_INTERRUPT, syscall_handler);
    }
    crate::log_info!("System call handler initialized (interrupt 0x{:02x})", SYSCALL_INTERRUPT);
}

