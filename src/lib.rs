//! Simple OS Kernel Library
//!
//! 이 모듈은 Simple OS 커널의 라이브러리 루트입니다.
//! 각 커널 모듈은 여기서 export됩니다.

#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

// 모듈 선언 (추후 구현)
// pub mod boot;
// pub mod memory;
// pub mod scheduler;
// pub mod power;
// pub mod drivers;
// pub mod interrupts;
// pub mod sync;
// pub mod syscall;
// pub mod fs;
// pub mod net;

/// 테스트 러너 (통합 테스트용)
#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    // TODO: 테스트 실행 구현
    for test in tests {
        test();
    }
}

