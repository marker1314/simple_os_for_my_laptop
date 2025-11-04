//! Simple OS Kernel Entry Point
//!
//! 이 파일은 커널의 엔트리 포인트입니다.
//! 부트로더가 커널을 로드한 후 `_start` 함수가 호출됩니다.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(simple_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate simple_os;
use simple_os::drivers::serial;
use simple_os::interrupts;
use bootloader_api::{BootInfo, entry_point};

entry_point!(kernel_main);

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
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
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
fn kernel_init(boot_info: &'static mut BootInfo) {
    // 1. 시리얼 포트 초기화 (가장 먼저, 로깅을 위해 필요)
    serial::init();
    simple_os::log_info!("Simple OS Kernel Starting...");
    
    // 2. 부트 정보 저장
    unsafe {
        simple_os::boot::init_boot_info(boot_info);
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
    
    // 6. 메모리 관리자 초기화
    unsafe {
        match simple_os::memory::init(boot_info) {
            Ok(()) => {
                simple_os::log_info!("Memory management initialized successfully");
            }
            Err(e) => {
                simple_os::log_error!("Failed to initialize memory management: {:?}", e);
                panic!("Memory initialization failed");
            }
        }
    }
    
    // 7. 타이머 드라이버 초기화
    unsafe {
        simple_os::drivers::timer::init();
        // PIC에서 타이머 인터럽트 활성화 (IRQ 0)
        interrupts::pic::set_mask(0, true);
    }
    simple_os::log_info!("Timer driver initialized");
    
    // 8. 키보드 드라이버 초기화
    unsafe {
        simple_os::drivers::keyboard::init();
        // PIC에서 키보드 인터럽트 활성화 (IRQ 1)
        interrupts::pic::set_mask(1, true);
    }
    simple_os::log_info!("Keyboard driver initialized");
    
    // 9. VGA 드라이버 초기화
    simple_os::drivers::vga::init();
    simple_os::vga_println!("Simple OS Kernel");
    simple_os::vga_println!("==================");
    simple_os::vga_println!("Uptime: {} ms", simple_os::drivers::timer::get_milliseconds());
    
    // 10. 스케줄러 초기화
    // 시간 할당량: 10 타이머 틱 (약 10ms)
    simple_os::scheduler::init(10);
    simple_os::log_info!("Scheduler initialized");
    
    // 11. 시스템 콜 핸들러 초기화
    simple_os::syscall::init_syscall_handler();
    simple_os::log_info!("System call handler initialized");
    
    // 12. ATA 드라이버 초기화
    unsafe {
        simple_os::drivers::ata::init();
    }
    simple_os::log_info!("ATA driver initialization attempted");
    
    // 13. 파일시스템 초기화 (ATA가 감지된 경우)
    // TODO: FAT32와 ATA를 완전히 통합한 후 활성화
    simple_os::log_info!("Filesystem module ready");
    
    // 14. 전력 관리 초기화
    unsafe {
        match simple_os::power::init() {
            Ok(()) => {
                simple_os::log_info!("Power management initialized");
            }
            Err(e) => {
                simple_os::log_warn!("Failed to initialize power management: {:?}", e);
                // 전력 관리 초기화 실패해도 커널은 계속 실행 가능
            }
        }
    }
    
    // 15. 네트워크 드라이버 초기화
    unsafe {
        match simple_os::net::init_network() {
            Ok(()) => {
                simple_os::log_info!("Network driver initialized");
                // MAC 주소 출력
                if let Ok(mac) = simple_os::net::get_mac_address() {
                    simple_os::log_info!("Network MAC address: {}", mac);
                }
            }
            Err(e) => {
                simple_os::log_warn!("Failed to initialize network driver: {:?}", e);
                // 네트워크 드라이버 초기화 실패해도 커널은 계속 실행 가능
            }
        }
    }
    
    simple_os::log_info!("Kernel initialization complete");
    
    // 16. Shell 시작
    simple_os::log_info!("Starting shell...");
    let mut shell = simple_os::shell::Shell::new();
    shell.run();
}

