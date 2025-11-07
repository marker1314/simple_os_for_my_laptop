//! IPI (Inter-Processor Interrupt) 관리
//!
//! CPU 간 통신을 위한 인터럽트를 관리합니다.

use super::apic;

/// IPI 전달 모드
#[repr(u32)]
pub enum DeliveryMode {
    /// Fixed: 고정 벡터 전달
    Fixed = 0b000,
    /// LowestPriority: 가장 낮은 우선순위 CPU로 전달
    LowestPriority = 0b001,
    /// SMI: System Management Interrupt
    SMI = 0b010,
    /// NMI: Non-Maskable Interrupt
    NMI = 0b100,
    /// INIT: INIT IPI
    INIT = 0b101,
    /// StartUp: Startup IPI
    StartUp = 0b110,
}

/// IPI 목적지 단축키
#[repr(u32)]
pub enum DestinationShorthand {
    /// No Shorthand: 명시적 목적지
    NoShorthand = 0b00,
    /// Self: 자신에게만
    SelfOnly = 0b01,
    /// AllIncludingSelf: 모든 CPU (자신 포함)
    AllIncludingSelf = 0b10,
    /// AllExcludingSelf: 모든 CPU (자신 제외)
    AllExcludingSelf = 0b11,
}

/// 특정 CPU에 IPI 전송
///
/// # Arguments
/// * `dest_apic_id` - 대상 CPU의 APIC ID
/// * `vector` - 인터럽트 벡터 번호
pub fn send_ipi(dest_apic_id: u8, vector: u8) {
    send_ipi_full(
        dest_apic_id,
        vector,
        DeliveryMode::Fixed,
        DestinationShorthand::NoShorthand,
    );
}

/// 모든 CPU에 IPI 브로드캐스트 (자신 제외)
///
/// # Arguments
/// * `vector` - 인터럽트 벡터 번호
pub fn broadcast_ipi(vector: u8) {
    send_ipi_full(
        0,
        vector,
        DeliveryMode::Fixed,
        DestinationShorthand::AllExcludingSelf,
    );
}

/// INIT IPI 전송 (CPU 초기화)
///
/// # Arguments
/// * `dest_apic_id` - 대상 CPU의 APIC ID
pub fn send_init_ipi(dest_apic_id: u8) {
    send_ipi_full(
        dest_apic_id,
        0,
        DeliveryMode::INIT,
        DestinationShorthand::NoShorthand,
    );
}

/// SIPI (Startup IPI) 전송
///
/// # Arguments
/// * `dest_apic_id` - 대상 CPU의 APIC ID
/// * `start_page` - 시작 페이지 번호 (4KB 단위)
pub fn send_startup_ipi(dest_apic_id: u8, start_page: u8) {
    send_ipi_full(
        dest_apic_id,
        start_page,
        DeliveryMode::StartUp,
        DestinationShorthand::NoShorthand,
    );
}

/// IPI 전송 (전체 옵션)
///
/// # Arguments
/// * `dest_apic_id` - 대상 CPU의 APIC ID
/// * `vector` - 인터럽트 벡터 번호
/// * `delivery_mode` - 전달 모드
/// * `destination_shorthand` - 목적지 단축키
fn send_ipi_full(
    dest_apic_id: u8,
    vector: u8,
    delivery_mode: DeliveryMode,
    destination_shorthand: DestinationShorthand,
) {
    unsafe {
        // ICR (Interrupt Command Register) 설정
        // High 32비트: 목적지 APIC ID
        let icr_high = (dest_apic_id as u32) << 24;
        
        // Low 32비트: 벡터, 전달 모드, 목적지 모드 등
        let icr_low = (vector as u32)
            | ((delivery_mode as u32) << 8)
            | ((destination_shorthand as u32) << 18);
        
        // ICR에 쓰기 (먼저 High, 그 다음 Low)
        write_icr(icr_high, icr_low);
    }
}

/// ICR (Interrupt Command Register)에 쓰기
///
/// # Safety
/// Local APIC가 초기화된 후에 호출되어야 합니다.
unsafe fn write_icr(high: u32, low: u32) {
    const ICR_HIGH: u32 = 0x310;
    const ICR_LOW: u32 = 0x300;
    
    // Local APIC 레지스터에 직접 쓰기
    // 실제 구현에서는 apic 모듈의 함수 사용
    let base = 0xFEE0_0000u64;
    
    let high_ptr = (base + ICR_HIGH as u64) as *mut u32;
    let low_ptr = (base + ICR_LOW as u64) as *mut u32;
    
    core::ptr::write_volatile(high_ptr, high);
    core::ptr::write_volatile(low_ptr, low);
    
    // ICR 전송 완료 대기
    wait_for_icr_ready();
}

/// ICR 전송 완료 대기
unsafe fn wait_for_icr_ready() {
    const ICR_LOW: u32 = 0x300;
    const DELIVERY_STATUS_BIT: u32 = 1 << 12;
    
    let base = 0xFEE0_0000u64;
    let low_ptr = (base + ICR_LOW as u64) as *const u32;
    
    // Delivery Status 비트가 0이 될 때까지 대기
    while core::ptr::read_volatile(low_ptr) & DELIVERY_STATUS_BIT != 0 {
        core::hint::spin_loop();
    }
}

/// TLB (Translation Lookaside Buffer) 플러시 IPI
///
/// 모든 CPU의 TLB를 플러시합니다.
pub fn send_tlb_flush_ipi() {
    // TLB 플러시용 벡터 번호 (사용자 정의)
    const TLB_FLUSH_VECTOR: u8 = 0xFD;
    broadcast_ipi(TLB_FLUSH_VECTOR);
}

/// 스케줄러 재스케줄링 IPI
///
/// 특정 CPU에 재스케줄링을 요청합니다.
pub fn send_reschedule_ipi(dest_apic_id: u8) {
    // 재스케줄링용 벡터 번호 (사용자 정의)
    const RESCHEDULE_VECTOR: u8 = 0xFC;
    send_ipi(dest_apic_id, RESCHEDULE_VECTOR);
}




