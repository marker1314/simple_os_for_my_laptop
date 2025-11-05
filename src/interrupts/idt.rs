//! IDT (Interrupt Descriptor Table) 구현
//!
//! 이 모듈은 인터럽트 디스크립터 테이블을 설정하고 관리합니다.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::instructions::interrupts;

use crate::interrupts::pic;
use crate::{log_error, log_warn, log_debug, log_info};

/// 전역 IDT
pub static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// IDT 초기화
///
/// 모든 예외 및 인터럽트 핸들러를 등록합니다.
pub unsafe fn init() {
    IDT.divide_error.set_handler_fn(divide_error_handler);
    IDT.debug.set_handler_fn(debug_handler);
    IDT.non_maskable_interrupt.set_handler_fn(nmi_handler);
    IDT.breakpoint.set_handler_fn(breakpoint_handler);
    IDT.overflow.set_handler_fn(overflow_handler);
    IDT.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
    IDT.invalid_opcode.set_handler_fn(invalid_opcode_handler);
    IDT.device_not_available.set_handler_fn(device_not_available_handler);
    IDT.double_fault.set_handler_fn(double_fault_handler);
    // IDT.coprocessor_segment_overrun.set_handler_fn(coprocessor_segment_overrun_handler);
    IDT.invalid_tss.set_handler_fn(invalid_tss_handler);
    IDT.segment_not_present.set_handler_fn(segment_not_present_handler);
    IDT.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
    IDT.general_protection_fault.set_handler_fn(general_protection_fault_handler);
    IDT.page_fault.set_handler_fn(page_fault_handler);
    IDT.x87_floating_point.set_handler_fn(x87_floating_point_handler);
    IDT.alignment_check.set_handler_fn(alignment_check_handler);
    IDT.machine_check.set_handler_fn(machine_check_handler);
    IDT.simd_floating_point.set_handler_fn(simd_floating_point_handler);
    IDT.virtualization.set_handler_fn(virtualization_handler);
    IDT.security_exception.set_handler_fn(security_exception_handler);

    // 하드웨어 인터럽트 (PIC 인터럽트)
    // IRQ 0: 타이머 (인터럽트 32)
    IDT[32].set_handler_fn(crate::drivers::timer::timer_interrupt_handler);
    // IRQ 1: 키보드 (인터럽트 33)
    IDT[33].set_handler_fn(crate::drivers::keyboard::keyboard_interrupt_handler);
    // IRQ 12: PS/2 마우스 (인터럽트 44)
    IDT[44].set_handler_fn(mouse_interrupt_handler);

    // IDT 로드
    IDT.load();
}

/// 시스템 콜 핸들러 등록
///
/// # Arguments
/// * `interrupt_num` - 인터럽트 번호
/// * `handler` - 핸들러 함수
pub unsafe fn register_syscall_handler(interrupt_num: u8, handler: extern "x86-interrupt" fn(InterruptStackFrame)) {
    IDT[interrupt_num as usize].set_handler_fn(handler);
    log_info!("Registered syscall handler for interrupt 0x{:02x}", interrupt_num);
}

/// 예외 핸들러: Divide Error (0x00)
extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    log_error!("Divide Error Exception");
    log_error!("Stack Frame: {:#?}", stack_frame);
    loop {
        x86_64::instructions::hlt();
    }
}

/// 예외 핸들러: Debug (0x01)
extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    log_debug!("Debug Exception");
    // 디버깅을 위해 무한 루프하지 않음
}

/// 예외 핸들러: Non-Maskable Interrupt (0x02)
extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    log_warn!("Non-Maskable Interrupt");
}

/// 예외 핸들러: Breakpoint (0x03)
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::log_debug!("Breakpoint Exception");
    // 디버깅을 위해 무한 루프하지 않음
}

/// 예외 핸들러: Overflow (0x04)
extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    log_error!("Overflow Exception");
    log_error!("Stack Frame: {:#?}", stack_frame);
}

/// 예외 핸들러: Bound Range Exceeded (0x05)
extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    log_error!("Bound Range Exceeded Exception");
    log_error!("Stack Frame: {:#?}", stack_frame);
}

/// 예외 핸들러: Invalid Opcode (0x06)
extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    log_error!("Invalid Opcode Exception");
    log_error!("Stack Frame: {:#?}", stack_frame);
    log_error!("Instruction Pointer: {:#016x}", stack_frame.instruction_pointer.as_u64());
    crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x06);
    loop {
        x86_64::instructions::hlt();
    }
}

/// 예외 핸들러: Device Not Available (0x07)
extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    log_warn!("Device Not Available Exception");
}

/// 예외 핸들러: Double Fault (0x08)
/// 
/// Double Fault는 스택 오버플로우나 다른 예외 처리 중 발생한 예외입니다.
/// 안전한 크래시 덤프를 생성하고 시스템을 종료합니다.
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    log_error!("=== Double Fault Exception ===");
    log_error!("Error Code: {:#016x}", error_code);
    log_error!("RIP: {:#016x}", stack_frame.instruction_pointer.as_u64());
    log_error!("Stack Frame: {:#?}", stack_frame);
    
    // 스택 오버플로우 가능성 확인
    let current_rsp: u64;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) current_rsp, options(nostack, preserves_flags));
    }
    log_error!("Current RSP: {:#016x}", current_rsp);
    
    // 크래시 덤프 기록
    crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x08);
    
    // 크래시 덤프 출력
    if let Some(dump) = crate::crash::take() {
        crate::crash::print_crash_dump(&dump);
    }
    
    log_error!("Double Fault is unrecoverable. System halted.");
    
    // 안전하게 시스템 종료
    loop {
        x86_64::instructions::hlt();
    }
}

/// 예외 핸들러: Coprocessor Segment Overrun (0x09)
extern "x86-interrupt" fn coprocessor_segment_overrun_handler(stack_frame: InterruptStackFrame) {
    log_error!("Coprocessor Segment Overrun Exception");
}

/// 예외 핸들러: Invalid TSS (0x0A)
extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, _error_code: u64) {
    log_error!("Invalid TSS Exception");
    log_error!("Stack Frame: {:#?}", stack_frame);
}

/// 예외 핸들러: Segment Not Present (0x0B)
extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, _error_code: u64) {
    log_error!("Segment Not Present Exception");
    log_error!("Stack Frame: {:#?}", stack_frame);
}

/// 예외 핸들러: Stack Segment Fault (0x0C)
extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) {
    log_error!("Stack Segment Fault Exception");
    log_error!("Stack Frame: {:#?}", stack_frame);
}

/// 예외 핸들러: General Protection Fault (0x0D)
/// 
/// GPF는 잘못된 메모리 접근, 세그먼트 위반 등을 감지합니다.
/// 가능한 경우 복구를 시도하고, 불가능하면 안전하게 종료합니다.
extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    log_error!("=== General Protection Fault ===");
    log_error!("Error Code: {:#016x}", error_code);
    log_error!("RIP: {:#016x}", stack_frame.instruction_pointer.as_u64());
    log_error!("Stack Frame: {:#?}", stack_frame);
    
    // Error code 분석
    // Bit 0: External event
    // Bit 1: Descriptor location (1=IDT/GDT/LDT, 0=other)
    // Bit 2: GDT (1) or LDT/IDT (0)
    // Bit 3: LDT (1) or GDT/IDT (0)
    // Bits 4-15: Segment selector index (if applicable)
    
    let is_external = (error_code & 1) != 0;
    let is_descriptor = (error_code & 2) != 0;
    
    if is_descriptor {
        let selector_index = ((error_code >> 3) & 0x1FFF) as u16;
        log_error!("Segment selector error: index={}, external={}", selector_index, is_external);
    } else {
        log_error!("Other protection violation, external={}", is_external);
    }
    
    // 현재 스택 상태 확인
    let current_rsp: u64;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) current_rsp, options(nostack, preserves_flags));
    }
    log_error!("Current RSP: {:#016x}", current_rsp);
    
    // 크래시 덤프 기록
    crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x0D);
    
    // 크래시 덤프 출력
    if let Some(dump) = crate::crash::take() {
        crate::crash::print_crash_dump(&dump);
    }
    
    log_error!("GPF is typically unrecoverable. System halted.");
    
    // 안전하게 시스템 종료
    loop {
        x86_64::instructions::hlt();
    }
}

/// 예외 핸들러: Page Fault (0x0E)
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    
    // 페이지 폴트 메트릭 기록
    crate::monitoring::record_page_fault();
    
    let accessed_address = Cr2::read();
    let addr_u64 = accessed_address.as_u64();
    
    // 1. 읽기 전용 페이지 쓰기 시도 처리 (COW 가능)
    if error_code.contains(PageFaultErrorCode::PRESENT) && 
       error_code.contains(PageFaultErrorCode::WRITE) {
        // COW 페이지인지 확인 (읽기 전용이지만 복사 가능)
        // 현재는 COW 추적 메커니즘이 없으므로, 일단 에러로 처리
        // 향후 COW 페이지 추적 테이블 추가 시 여기서 복사 처리
        
        // 커널 코드 영역은 절대 쓰기 불가
        let kernel_code_base = 0xFFFF800000000000u64;
        let kernel_code_end = 0xFFFF800000100000u64;
        if addr_u64 >= kernel_code_base && addr_u64 < kernel_code_end {
            crate::log_error!("Page Fault: Write to kernel code region at {:#016x}", addr_u64);
            crate::log_error!("RIP: {:#016x}", stack_frame.instruction_pointer.as_u64());
            crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x0E);
            loop { x86_64::instructions::hlt(); }
        }
        
        // 다른 읽기 전용 페이지는 COW 시도 (향후 구현)
        // 현재는 에러로 처리
        crate::log_error!("Page Fault: Write to read-only page at {:#016x}", addr_u64);
        crate::log_error!("RIP: {:#016x}", stack_frame.instruction_pointer.as_u64());
        crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x0E);
        loop { x86_64::instructions::hlt(); }
    }
    
    // 2. 커널 영역 보호 위반 감지
    let kernel_base = 0xFFFF800000000000u64;
    let kernel_end = 0xFFFFFFFFFFFFFFFFu64;
    if addr_u64 >= kernel_base && addr_u64 < kernel_end {
        // 커널 코드 영역 (읽기 전용) 보호
        let code_base = 0xFFFF800000000000u64;
        let code_end = 0xFFFF800000100000u64; // 예상 커널 코드 영역
        if addr_u64 >= code_base && addr_u64 < code_end && 
           error_code.contains(PageFaultErrorCode::WRITE) {
            crate::log_error!("Page Fault: Write to kernel code region at {:#016x}", addr_u64);
            crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x0E);
            loop { x86_64::instructions::hlt(); }
        }
    }
    
    // 3. 스택 오버플로우 감지 (Guard page 접근)
    // 스택은 일반적으로 높은 주소에서 낮은 주소로 자라므로,
    // 스택 아래(낮은 주소) 접근은 오버플로우로 간주
    // 현재 스레드의 RSP 확인
    let current_rsp: u64;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) current_rsp, options(nostack, preserves_flags));
    }
    
    // 스택 영역 근처의 접근 감지 (Guard page)
    // 스택 포인터보다 낮은 주소 접근은 스택 오버플로우 가능성
    if addr_u64 < current_rsp && addr_u64 > current_rsp.saturating_sub(0x10000) {
        crate::log_error!("Page Fault: Possible stack overflow at {:#016x} (RSP: {:#016x})", 
                         addr_u64, current_rsp);
        crate::log_error!("This may indicate stack overflow or guard page access");
        crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x0E);
        loop { x86_64::instructions::hlt(); }
    }
    
    // 4. 스왑된 페이지 처리 (스왑 인)
    if !error_code.contains(PageFaultErrorCode::PRESENT) {
        use crate::memory::swap;
        use x86_64::structures::paging::Page;
        
        if swap::is_swap_enabled() {
            let page = Page::<x86_64::structures::paging::Size4KiB>::containing_address(accessed_address);
            
            // 페이지가 스왑되어 있는지 확인
            unsafe {
                match swap::try_swap_in(page) {
                    Ok(frame) => {
                        // 스왑에서 복원된 페이지를 메모리에 매핑
                        let page_base_va = x86_64::VirtAddr::new(addr_u64 & !0xfffu64);
                        match crate::memory::paging::map_swap_page_at(page_base_va, frame) {
                            Ok(()) => {
                                crate::log_info!("Swapped in page at {:#016x}", page_base_va.as_u64());
                                return;
                            }
                            Err(e) => {
                                crate::log_error!("Failed to map swapped page: {:?}", e);
                            }
                        }
                    }
                    Err(_) => {
                        // 스왑에 없음, 계속 진행
                    }
                }
            }
        }
    }
    
    // 5. 힙 확장 처리 (기존 로직)
    if !error_code.contains(PageFaultErrorCode::PRESENT) {
        let (heap_start, heap_size) = crate::memory::heap::heap_bounds();
        let heap_end = heap_start.saturating_add(heap_size);
        let heap_max = heap_start + (2 * 1024 * 1024); // 최대 2MB
        
        // 힙 범위 내의 미매핑 페이지 접근
        if addr_u64 >= heap_start as u64 && addr_u64 < heap_max as u64 {
            let page_base = addr_u64 & !0xfffu64;
            
            // 힙 확장 가능 범위 확인
            if page_base >= heap_end as u64 && page_base < heap_max as u64 {
                let page_base_va = x86_64::VirtAddr::new(page_base);
                unsafe {
                    // 메모리 부족 시 스왑 시도
                    if crate::memory::swap::is_swap_enabled() {
                        // 프레임 할당 실패 시 스왑 아웃 시도
                        if let Err(_) = crate::memory::paging::map_zero_page_at(page_base_va) {
                            // 스왑 아웃 후 재시도
                            if let Ok(_) = crate::memory::swap::try_swap_out_lru() {
                                // 스왑 아웃 성공, 재시도
                                match crate::memory::paging::map_zero_page_at(page_base_va) {
                                    Ok(()) => {
                                        crate::log_info!(
                                            "Handled heap grow-on-demand with swap at {:#016x}",
                                            page_base
                                        );
                                        return;
                                    }
                                    Err(e) => {
                                        crate::log_error!("Page fault recovery failed even after swap: {:?}", e);
                                    }
                                }
                            }
                        } else {
                            crate::log_info!(
                                "Handled heap grow-on-demand at {:#016x}",
                                page_base
                            );
                            return;
                        }
                    } else {
                        // 스왑 없이 일반 힙 확장
                        match crate::memory::paging::map_zero_page_at(page_base_va) {
                            Ok(()) => {
                                crate::log_info!(
                                    "Handled heap grow-on-demand at {:#016x}",
                                    page_base
                                );
                                return;
                            }
                            Err(e) => {
                                crate::log_error!("Page fault recovery failed: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. 복구 불가능한 오류: 로그 및 정지
    log_error!("Page Fault Exception - Unrecoverable");
    log_error!("Accessed Address: {:#016x}", accessed_address.as_u64());
    log_error!("Error Code: {:?}", error_code);
    log_error!("RIP: {:#016x}", stack_frame.instruction_pointer.as_u64());
    log_error!("RSP: {:#016x}", current_rsp);
    log_error!("Stack Frame: {:#?}", stack_frame);
    
    // 추가 컨텍스트 정보
    if error_code.contains(PageFaultErrorCode::USER) {
        log_error!("Fault occurred in user mode");
    } else {
        log_error!("Fault occurred in kernel mode");
    }
    
    crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x0E);
    loop { x86_64::instructions::hlt(); }
}

/// 예외 핸들러: x87 Floating Point (0x10)
extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
    log_warn!("x87 Floating Point Exception");
}

/// 예외 핸들러: Alignment Check (0x11)
extern "x86-interrupt" fn alignment_check_handler(stack_frame: InterruptStackFrame, _error_code: u64) {
    log_error!("Alignment Check Exception");
}

/// 예외 핸들러: Machine Check (0x12)
extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    log_error!("Machine Check Exception - System Halted");
    loop {
        x86_64::instructions::hlt();
    }
}

/// 예외 핸들러: SIMD Floating Point (0x13)
extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    log_warn!("SIMD Floating Point Exception");
}

/// 예외 핸들러: Virtualization (0x14)
extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    log_warn!("Virtualization Exception");
}

/// 예외 핸들러: Security Exception (0x1E)
extern "x86-interrupt" fn security_exception_handler(stack_frame: InterruptStackFrame, _error_code: u64) {
    log_error!("Security Exception");
}

/// 하드웨어 인터럽트: PS/2 마우스 (IRQ 12)
extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // 마우스 드라이버의 인터럽트 핸들러 호출
    crate::drivers::mouse::handle_interrupt();
    
    // 사용자 활동 기록
    crate::power::user_activity::record_activity(crate::power::user_activity::ActivityType::Mouse);
    
    // PIC에 EOI 전송 (IRQ 12는 슬레이브 PIC)
    unsafe {
        pic::end_of_interrupt(12);
    }
}

/// 인터럽트 활성화
pub fn enable_interrupts() {
    unsafe {
        interrupts::enable();
    }
}

/// 인터럽트 비활성화
pub fn disable_interrupts() {
    unsafe {
        interrupts::disable();
    }
}

