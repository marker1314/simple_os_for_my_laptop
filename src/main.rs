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
        if let Some(pm) = simple_os::power::get_manager() {
            if let Some(manager) = pm.lock().as_ref() {
                unsafe { manager.enter_idle(); }
                continue;
            }
        }
        x86_64::instructions::hlt();
    }
}

/// 커널 초기화 함수
///
/// 모든 커널 모듈을 순서대로 초기화합니다.
fn kernel_init(boot_info: &'static mut BootInfo) {
    // 부팅 타임라인 시작
    simple_os::boot::mark_boot_start();
    
    // 1. 시리얼 포트 초기화 (가장 먼저, 로깅을 위해 필요)
    serial::init();
    simple_os::boot::mark_stage(simple_os::boot::BootStage::SerialInit);
    simple_os::log_info!("Simple OS Kernel Starting...");
    
    // 2. 부트 정보 저장
    unsafe {
        simple_os::boot::init_boot_info(boot_info);
    }
    simple_os::boot::mark_stage(simple_os::boot::BootStage::BootInfoInit);
    simple_os::log_info!("Boot info initialized");
    simple_os::log_info!("Memory map entries: {}", simple_os::boot::memory_map_len());
    // 이전 크래시 보고 (심볼화된 덤프)
    if let Some(dump) = simple_os::crash::take() {
        simple_os::crash::print_crash_dump(&dump);
    }
    
    // 3. PIC 리매핑
    unsafe {
        interrupts::pic::init();
    }
    simple_os::boot::mark_stage(simple_os::boot::BootStage::PicRemap);
    simple_os::log_info!("PIC remapped");
    
    // 4. IDT 설정
    unsafe {
        interrupts::idt::init();
    }
    simple_os::boot::mark_stage(simple_os::boot::BootStage::IdtInit);
    simple_os::log_info!("IDT initialized");
    
    // 5. 인터럽트 활성화
    interrupts::idt::enable_interrupts();
    simple_os::boot::mark_stage(simple_os::boot::BootStage::InterruptsEnabled);
    simple_os::log_info!("Interrupts enabled");
    
    // 6. 메모리 관리자 초기화
    unsafe {
        match simple_os::memory::init(boot_info) {
            Ok(()) => {
                simple_os::boot::mark_stage(simple_os::boot::BootStage::MemoryInit);
                simple_os::log_info!("Memory management initialized successfully");
                
                // ASLR 초기화 (메모리 관리 후)
                if let Err(e) = simple_os::memory::paging::enable_aslr() {
                    simple_os::log_warn!("Failed to enable ASLR: {}", e);
                    // ASLR 실패해도 커널은 계속 실행 가능
                } else {
                    simple_os::log_info!("ASLR enabled");
                }
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
    simple_os::boot::mark_stage(simple_os::boot::BootStage::TimerInit);
    simple_os::log_info!("Timer driver initialized");
    
    // 8. 키보드 드라이버 초기화
    unsafe {
        simple_os::drivers::keyboard::init();
        // PIC에서 키보드 인터럽트 활성화 (IRQ 1)
        interrupts::pic::set_mask(1, true);
    }
    simple_os::boot::mark_stage(simple_os::boot::BootStage::KeyboardInit);
    simple_os::log_info!("Keyboard driver initialized");
    
    // 9. VGA 드라이버 초기화
    simple_os::drivers::vga::init();
    simple_os::boot::mark_stage(simple_os::boot::BootStage::VgaInit);
    simple_os::vga_println!("Simple OS Kernel");
    simple_os::vga_println!("==================");
    simple_os::vga_println!("Uptime: {} ms", simple_os::drivers::timer::get_milliseconds());
    
    // 10. 스케줄러 초기화
    // 시간 할당량: 10 타이머 틱 (약 10ms)
    simple_os::scheduler::init(10);
    simple_os::boot::mark_stage(simple_os::boot::BootStage::SchedulerInit);
    simple_os::log_info!("Scheduler initialized");
    
    // 11. 시스템 콜 핸들러 초기화
    simple_os::syscall::init_syscall_handler();
    simple_os::boot::mark_stage(simple_os::boot::BootStage::SyscallInit);
    simple_os::log_info!("System call handler initialized");
    
    // 12. ATA 드라이버 초기화 (파일시스템 기능 사용 시에만)
    #[cfg(feature = "fs")]
    {
        unsafe { simple_os::drivers::ata::init(); }
        simple_os::boot::mark_stage(simple_os::boot::BootStage::AtaInit);
        simple_os::log_info!("ATA driver initialization attempted");
        match simple_os::config::profile::current_profile() {
            simple_os::config::profile::Profile::PowerSaver => {
                simple_os::drivers::ata::set_idle_timeout_ms(30000);
                simple_os::log_info!("ATA idle timeout set to 30000ms (power_saver)");
            }
            _ => {}
        }
    }
    
    // 13. 파일시스템 초기화 (ATA가 감지된 경우)
    // TODO: FAT32와 ATA를 완전히 통합한 후 활성화
    #[cfg(feature = "fs")]
    {
        simple_os::boot::mark_stage(simple_os::boot::BootStage::FilesystemReady);
        simple_os::log_info!("Filesystem module ready");
    }
    
    // 14. 전력 관리 초기화
    unsafe {
        match simple_os::power::init() {
            Ok(()) => {
                simple_os::boot::mark_stage(simple_os::boot::BootStage::PowerInit);
                simple_os::log_info!("Power management initialized");
                
                // RAPL 전력 측정 초기화
                simple_os::power::rapl::init_rapl_measurement();
                
                // CPU 온도 모니터링 초기화
                simple_os::power::temps::init_thermal_monitoring();
                
                // 디바이스별 전력 프로파일 초기화
                simple_os::power::device_policy::init_device_power_profiles();
                
                // 전력 프로파일 적용
                if let Err(e) = simple_os::power::device_policy::apply_device_power_policies() {
                    simple_os::log_warn!("Failed to apply device power policies: {:?}", e);
                }
                
                // 메모리 압축 초기화
                simple_os::memory::compression::init_compression(32); // 최대 32개 페이지 압축
                
                // 메모리 단편화 모니터링 초기화
                simple_os::memory::fragmentation::init_fragmentation_monitoring(100); // 최대 100개 히스토리
                
                // OOM Killer 초기화
                let oom_config = simple_os::memory::oom_killer::OomKillerConfig {
                    memory_threshold_percent: 5,
                    min_memory_bytes: 1024 * 1024, // 1MB
                    enabled: true,
                };
                simple_os::memory::oom_killer::init_oom_killer(oom_config);
                
                // 사용자 활동 감지기 초기화
                simple_os::power::user_activity::init_user_activity_detector(100, 30000); // 30초 유휴 임계값
                
                // 배터리 관리자 초기화
                if let Err(e) = simple_os::power::battery::init_battery_manager() {
                    simple_os::log_warn!("Failed to initialize battery manager: {:?}", e);
                }
            }
            Err(e) => {
                simple_os::log_warn!("Failed to initialize power management: {:?}", e);
                // 전력 관리 초기화 실패해도 커널은 계속 실행 가능
            }
        }
    }
    
    // 15. 네트워크 드라이버 초기화
    #[cfg(feature = "net")]
    unsafe {
        match simple_os::net::init_network() {
            Ok(()) => {
                simple_os::boot::mark_stage(simple_os::boot::BootStage::NetworkInit);
                simple_os::log_info!("Network driver initialized");
                if let Ok(mac) = simple_os::net::get_mac_address() {
                    simple_os::log_info!("Network MAC address: {}", mac);
                }
                match simple_os::config::profile::current_profile() {
                    simple_os::config::profile::Profile::PowerSaver => {
                        crate::drivers::rtl8139::set_idle_timeout_ms(10000);
                        simple_os::log_info!("Network idle timeout set to 10000ms (power_saver)");
                    }
                    _ => {}
                }
            }
            Err(e) => {
                simple_os::log_warn!("Failed to initialize network driver: {:?}", e);
            }
        }
    }
    
    // 15.5. USB 드라이버 초기화
    #[cfg(feature = "usb")]
    unsafe {
        match simple_os::drivers::usb::init() {
            Ok(()) => {
                simple_os::log_info!("USB subsystem initialized");
                // TODO: USB 디바이스 열거 (구현 후 활성화)
                // if let Err(e) = simple_os::drivers::usb::enumerate_devices() {
                //     simple_os::log_warn!("Failed to enumerate USB devices: {}", e);
                // }
            }
            Err(e) => {
                simple_os::log_warn!("Failed to initialize USB subsystem: {}", e);
                // USB 초기화 실패해도 커널은 계속 실행 가능
            }
        }
    }
    
    // 16. 프레임버퍼 초기화 (GUI 지원)
    #[cfg(feature = "gui")]
    unsafe {
        if let Some(framebuffer) = simple_os::boot::get_framebuffer() {
            simple_os::drivers::framebuffer::init(framebuffer);
            simple_os::boot::mark_stage(simple_os::boot::BootStage::FramebufferInit);
            simple_os::log_info!("Framebuffer initialized");
            match simple_os::gui::init() {
                Ok(()) => {
                    simple_os::boot::mark_stage(simple_os::boot::BootStage::GuiInit);
                    simple_os::log_info!("GUI system initialized");
                }
                Err(e) => {
                    simple_os::log_warn!("Failed to initialize GUI: {}", e);
                }
            }
        } else {
            simple_os::log_warn!("No framebuffer available, GUI disabled");
        }
    }
    
    // 17. 마우스 드라이버 초기화
    unsafe {
        simple_os::drivers::mouse::init();
        // PIC에서 마우스 인터럽트 활성화 (IRQ 12)
        simple_os::interrupts::pic::set_mask(12, true);
    }
    simple_os::boot::mark_stage(simple_os::boot::BootStage::MouseInit);
    simple_os::log_info!("Mouse driver initialized");
    
    // 18. I2C 및 트랙패드 드라이버 초기화 (선택적)
    #[cfg(feature = "touchpad")]
    unsafe {
        // ACPI에서 I2C 장치 정보 찾기
        if let Some(i2c_info) = simple_os::power::acpi::find_i2c_touchpad() {
            // I2C 컨트롤러 초기화
            match simple_os::drivers::i2c::init_controller(0, i2c_info.base_address) {
                Ok(()) => {
                    simple_os::log_info!("I2C controller initialized at 0x{:X}", i2c_info.base_address.as_u64());
                    
                    // 트랙패드 초기화
                    match simple_os::drivers::touchpad::init(i2c_info.slave_address) {
                        Ok(()) => {
                            simple_os::boot::mark_stage(simple_os::boot::BootStage::TouchpadInit);
                            simple_os::log_info!("ELAN touchpad initialized at I2C address 0x{:02X}", i2c_info.slave_address);
                        }
                        Err(e) => {
                            simple_os::log_warn!("Failed to initialize touchpad: {:?}", e);
                            simple_os::log_info!("Falling back to PS/2 mouse only");
                        }
                    }
                }
                Err(e) => {
                    simple_os::log_warn!("Failed to initialize I2C controller: {:?}", e);
                    simple_os::log_info!("Touchpad disabled, using PS/2 mouse only");
                }
            }
        } else {
            simple_os::log_info!("No I2C touchpad found, using PS/2 mouse only");
        }
    }
    
    simple_os::boot::mark_stage(simple_os::boot::BootStage::KernelInitComplete);
    simple_os::log_info!("Kernel initialization complete");
    
    // 부팅 타임라인 출력
    if let Some(total_ms) = simple_os::boot::get_total_boot_time_ms() {
        simple_os::log_info!("Total boot time: {}ms", total_ms);
    }
    simple_os::boot::print_timeline();
    
    // 19. GUI 데스크톱 환경 시작 (프레임버퍼가 사용 가능한 경우)
    #[cfg(feature = "gui")]
    if simple_os::drivers::framebuffer::is_initialized() {
        simple_os::boot::mark_stage(simple_os::boot::BootStage::DesktopStart);
        simple_os::log_info!("Starting desktop environment...");
        desktop_loop();
    } else {
        // 프레임버퍼가 없으면 Shell 시작
        simple_os::boot::mark_stage(simple_os::boot::BootStage::ShellStart);
        simple_os::log_info!("Starting shell...");
        let mut shell = simple_os::shell::Shell::new();
        shell.run();
    }
    #[cfg(not(feature = "gui"))]
    {
        simple_os::boot::mark_stage(simple_os::boot::BootStage::ShellStart);
        simple_os::log_info!("Starting shell...");
        let mut shell = simple_os::shell::Shell::new();
        shell.run();
    }
}

/// 데스크톱 환경 메인 루프
#[cfg(feature = "gui")]
fn desktop_loop() -> ! {
    use simple_os::drivers::mouse;
    use simple_os::drivers::touchpad;
    use simple_os::drivers::timer;
    
    let mut last_render_time = 0u64;
    let mut render_interval = 16u64; // adaptive: 16ms active, 33-100ms idle
    let mut last_input_time = 0u64;
    let display_blank_timeout_ms: u64 = match simple_os::config::profile::current_profile() {
        simple_os::config::profile::Profile::PowerSaver => 60_000,
        _ => 0,
    };
    
    loop {
        let current_time = timer::get_milliseconds();
        
        // 트랙패드 이벤트 폴링 (트랙패드가 초기화된 경우)
        if touchpad::is_initialized() {
            if let Some(event) = touchpad::poll_event() {
                simple_os::gui::desktop_manager::handle_mouse_event(event);
                last_input_time = current_time;
                if simple_os::drivers::framebuffer::is_blank() { simple_os::drivers::framebuffer::unblank(); }
            }
        }
        
        // PS/2 마우스 이벤트 처리 (백업 또는 외장 마우스)
        if let Some(event) = mouse::get_event() {
            simple_os::gui::desktop_manager::handle_mouse_event(event);
            last_input_time = current_time;
            if simple_os::drivers::framebuffer::is_blank() { simple_os::drivers::framebuffer::unblank(); }
        }
        
        // 주기적으로 화면 렌더링
        if current_time - last_render_time >= render_interval || simple_os::gui::compositor::needs_redraw() {
            if !simple_os::drivers::framebuffer::is_blank() {
                simple_os::gui::desktop_manager::render();
            }
            last_render_time = current_time;
            // 입력 유휴 시간에 따라 렌더 주기 조정
            let idle_ms = current_time.saturating_sub(last_input_time);
            render_interval = if idle_ms < 200 { 16 } else if idle_ms < 1000 { 33 } else { 100 };
        }
        // 디스플레이 블랭크 처리
        if display_blank_timeout_ms > 0 {
            let idle_ms = current_time.saturating_sub(last_input_time);
            if idle_ms >= display_blank_timeout_ms && !simple_os::drivers::framebuffer::is_blank() {
                simple_os::drivers::framebuffer::blank();
            }
        }
        
        // CPU 절전
        if let Some(pm) = simple_os::power::get_manager() {
            if let Some(manager) = pm.lock().as_ref() {
                unsafe { manager.enter_idle(); }
            } else {
                x86_64::instructions::hlt();
            }
        } else {
            x86_64::instructions::hlt();
        }
        // 디스크 유휴 관리
        #[cfg(feature = "fs")]
        simple_os::drivers::ata::maybe_enter_idle(current_time);
        // 네트워크 저전력 관리
        #[cfg(feature = "net")]
        simple_os::net::low_power_tick(current_time);
        // 전력 통계 틱
        simple_os::power::stats::tick(current_time);
        
        // Thermal 관리 (10초마다 체크)
        if current_time % 10_000 == 0 {
            simple_os::power::temps::check_thermal_and_throttle();
        }
        
        // On-demand governor 업데이트 (100ms마다)
        if let Some(manager) = simple_os::power::get_manager() {
            if let Some(pm) = manager.lock().as_mut() {
                // CPU 사용률 자동 계산
                let _ = pm.update_ondemand(None, current_time);
            }
        }
        
        // 전력 모니터링 대시보드 업데이트 (1초마다)
        simple_os::power::monitor::update_monitor(current_time);
    }
}

