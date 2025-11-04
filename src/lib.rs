//! Simple OS Kernel Library
//!
//! 이 모듈은 Simple OS 커널의 라이브러리 루트입니다.
//! 각 커널 모듈은 여기서 export됩니다.

#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

// 모듈 선언
// pub mod boot;
// pub mod memory;
// pub mod scheduler;
// pub mod power;
pub mod drivers;
// pub mod interrupts;
// pub mod sync;
// pub mod syscall;
// pub mod fs;
// pub mod net;

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

