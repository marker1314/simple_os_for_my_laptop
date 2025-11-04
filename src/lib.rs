//! Simple OS Kernel Library
//!
//! 이 모듈은 Simple OS 커널의 라이브러리 루트입니다.
//! 각 커널 모듈은 여기서 export됩니다.

#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![feature(alloc_error_handler)]
#![feature(asm_const)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

/// 패닉 핸들러
///
/// 커널 패닉이 발생했을 때 호출됩니다.
/// 라이브러리와 바이너리 크레이트 모두에서 사용됩니다.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // 시리얼 포트를 통한 패닉 메시지 출력
    // serial_println!은 시리얼 포트가 초기화되지 않아도 안전하게 작동합니다
    crate::serial_println!("\n=== KERNEL PANIC ===");
    crate::serial_println!("{}", info);
    crate::serial_println!("===================\n");
    
    loop {
        // 패닉 발생 시 무한 루프
        x86_64::instructions::hlt();
    }
}

// 모듈 선언
pub mod boot;
pub mod memory;
pub mod scheduler;
pub mod power;
pub mod drivers;
pub mod interrupts;
// pub mod sync;
pub mod syscall;
pub mod shell;
pub mod fs;
pub mod net;
pub mod gui;
pub mod smp;

pub mod logging;

// 매크로는 자동으로 crate 루트에 사용 가능하므로 재export 불필요
// 사용: simple_os::serial_println!() 또는 simple_os::log_info!()

/// 테스트 러너 (통합 테스트용)
#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    // TODO: 테스트 실행 구현
    for test in tests {
        test();
    }
}

