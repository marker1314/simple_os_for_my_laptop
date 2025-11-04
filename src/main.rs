//! Simple OS Kernel Entry Point
//!
//! 이 파일은 커널의 엔트리 포인트입니다.
//! 부트로더가 커널을 로드한 후 `_start` 함수가 호출됩니다.

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(simple_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use simple_os::drivers::serial;
use simple_os::interrupts;
use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);

/// 패닉 핸들러
///
/// 커널 패닉이 발생했을 때 호출됩니다.
/// 현재는 무한 루프에 빠지지만, 향후 로깅 및 복구 기능을 추가할 예정입니다.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 시리얼 포트를 통한 패닉 메시지 출력
    simple_os::serial_println!("\n=== KERNEL PANIC ===");
    simple_os::serial_println!("{}", info);
    simple_os::serial_println!("===================\n");
    
    loop {
        // 패닉 발생 시 무한 루프
        x86_64::instructions::hlt();
    }
}

/// 커널 엔트리 포인트
///
/// 부트로더가 커널을 로드한 후 이 함수가 호출됩니다.
/// bootloader 크레이트의 `entry_point!` 매크로가 실제 `_start` 함수를 생성합니다.
///
/// 초기화 순서:
/// 1. 인터럽트 디스크립터 테이블 (IDT) 설정
/// 2. 메모리 관리자 초기화
/// 3. 스케줄러 초기화
/// 4. 전력 관리 초기화
/// 5. 드라이버 초기화
/// 6. Shell/GUI 시작
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // 커널 초기화
    kernel_init(boot_info);
    
    // 메인 루프
    loop {
        // 유휴 상태에서 CPU를 대기 상태로 전환 (전력 절약)
        x86_64::instructions::hlt();
    }
}

/// 커널 초기화 함수
///
/// 모든 커널 모듈을 순서대로 초기화합니다.
fn kernel_init(boot_info: &'static BootInfo) {
    // 1. 시리얼 포트 초기화 (가장 먼저, 로깅을 위해 필요)
    serial::init();
    simple_os::log_info!("Simple OS Kernel Starting...");
    
    // 2. 부트 정보 저장
    unsafe {
        simple_os::boot::init(boot_info);
    }
    simple_os::log_info!("Boot info initialized");
    simple_os::log_info!("Memory map entries: {}", simple_os::boot::memory_map_len());
    
    // 3. PIC 리매핑
    unsafe {
        interrupts::pic::init();
    }
    simple_os::log_info!("PIC remapped");
    
    // 4. IDT 설정
    unsafe {
        interrupts::idt::init();
    }
    simple_os::log_info!("IDT initialized");
    
    // 5. 인터럽트 활성화
    interrupts::idt::enable_interrupts();
    simple_os::log_info!("Interrupts enabled");
    
    // TODO: 다음 단계 초기화
    // 6. 메모리 관리자 초기화
    // 7. 힙 할당자 초기화
    // 8. 드라이버 초기화
    // 9. 스케줄러 시작
    // 10. 전력 관리 초기화
    // 11. Shell/GUI 시작
    
    simple_os::log_info!("Kernel initialization complete");
}

