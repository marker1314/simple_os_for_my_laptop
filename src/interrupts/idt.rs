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
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    log_error!("Double Fault Exception - System Halted");
    log_error!("Stack Frame: {:#?}", stack_frame);
    crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x08);
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
extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) {
    log_error!("General Protection Fault Exception");
    log_error!("Stack Frame: {:#?}", stack_frame);
    crate::crash::record_exception(stack_frame.instruction_pointer.as_u64(), 0x0D);
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
    
    let accessed_address = Cr2::read();
    // Attempt best-effort recovery for non-present faults within heap range by
    // mapping a zero-initialized page at the faulting address (page-aligned).
    if !error_code.contains(PageFaultErrorCode::PRESENT) {
        let addr_u64 = accessed_address.as_u64();
        let (heap_start, heap_size) = crate::memory::heap::heap_bounds();
        let heap_end = heap_start.saturating_add(heap_size);
        // 제한: 힙의 최상위 미매핑 페이지에 대한 첫 접근만 자동 복구 (grow-on-demand)
        let expected_page_base = (heap_end as u64).saturating_sub(0x1000);
        let page_base = addr_u64 & !0xfffu64;
        if page_base == expected_page_base {
            let page_base_va = x86_64::VirtAddr::new(page_base);
            unsafe {
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

    // Unrecoverable fault: log and halt
    log_error!("Page Fault Exception");
    log_error!("Accessed Address: {:#016x}", accessed_address.as_u64());
    log_error!("Error Code: {:?}", error_code);
    log_error!("Stack Frame: {:#?}", stack_frame);
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

