//! APIC (Advanced Programmable Interrupt Controller) 드라이버
//!
//! Local APIC와 I/O APIC을 관리합니다.

use x86_64::structures::paging::{Page, PageTableFlags, PhysFrame};
use x86_64::PhysAddr;
use spin::Mutex;
use core::ptr::{read_volatile, write_volatile};

/// Local APIC 기본 물리 주소
const LOCAL_APIC_BASE: u64 = 0xFEE0_0000;

/// I/O APIC 기본 물리 주소 (ACPI MADT에서 읽어야 하지만 기본값 사용)
const IO_APIC_BASE: u64 = 0xFEC0_0000;

/// Local APIC 레지스터 오프셋
mod local_apic_reg {
    pub const ID: u32 = 0x20;           // Local APIC ID
    pub const VERSION: u32 = 0x30;      // APIC 버전
    pub const TPR: u32 = 0x80;          // Task Priority Register
    pub const EOI: u32 = 0xB0;          // End of Interrupt
    pub const SVR: u32 = 0xF0;          // Spurious Interrupt Vector Register
    pub const ICR_LOW: u32 = 0x300;     // Interrupt Command Register (Low)
    pub const ICR_HIGH: u32 = 0x310;    // Interrupt Command Register (High)
    pub const LVT_TIMER: u32 = 0x320;   // LVT Timer Register
    pub const LVT_LINT0: u32 = 0x350;   // LVT LINT0 Register
    pub const LVT_LINT1: u32 = 0x360;   // LVT LINT1 Register
    pub const LVT_ERROR: u32 = 0x370;   // LVT Error Register
    pub const TIMER_INIT: u32 = 0x380;  // Timer Initial Count
    pub const TIMER_CURRENT: u32 = 0x390; // Timer Current Count
    pub const TIMER_DIV: u32 = 0x3E0;   // Timer Divide Configuration
}

/// I/O APIC 레지스터
mod io_apic_reg {
    pub const IOREGSEL: u32 = 0x00;     // I/O Register Select
    pub const IOWIN: u32 = 0x10;        // I/O Window
    
    // I/O APIC 레지스터 인덱스
    pub const ID: u8 = 0x00;
    pub const VER: u8 = 0x01;
    pub const ARB: u8 = 0x02;
    pub const REDTBL_BASE: u8 = 0x10;   // Redirection Table Base
}

/// Local APIC 가상 주소
static LOCAL_APIC_ADDR: Mutex<Option<u64>> = Mutex::new(None);

/// I/O APIC 가상 주소
static IO_APIC_ADDR: Mutex<Option<u64>> = Mutex::new(None);

/// Local APIC 초기화
///
/// # Safety
/// 메모리 관리가 초기화된 후에 호출되어야 합니다.
pub unsafe fn init_local_apic() -> Result<(), &'static str> {
    // 1. MSR을 통해 Local APIC 활성화 확인
    let apic_base = read_apic_base_msr();
    
    if apic_base & (1 << 11) == 0 {
        // APIC가 비활성화된 경우 활성화
        write_apic_base_msr(apic_base | (1 << 11));
    }
    
    // 2. Local APIC 메모리 매핑
    // TODO: 페이지 테이블에 매핑 (현재는 직접 물리 주소 사용)
    // 실제로는 페이지 테이블을 통해 가상 주소에 매핑해야 함
    *LOCAL_APIC_ADDR.lock() = Some(LOCAL_APIC_BASE);
    
    // 3. Spurious Interrupt Vector Register 설정
    // APIC 활성화 (비트 8) + 스퓨리어스 벡터 (0xFF)
    write_local_apic_reg(local_apic_reg::SVR, 0x1FF);
    
    // 4. Task Priority Register를 0으로 설정 (모든 인터럽트 허용)
    write_local_apic_reg(local_apic_reg::TPR, 0);
    
    // 5. LVT 엔트리 마스킹 해제
    write_local_apic_reg(local_apic_reg::LVT_TIMER, 0x10000); // Masked
    write_local_apic_reg(local_apic_reg::LVT_LINT0, 0x10000); // Masked
    write_local_apic_reg(local_apic_reg::LVT_LINT1, 0x10000); // Masked
    write_local_apic_reg(local_apic_reg::LVT_ERROR, 0x10000); // Masked
    
    crate::log_info!("Local APIC initialized at 0x{:X}", LOCAL_APIC_BASE);
    
    Ok(())
}

/// I/O APIC 초기화
///
/// # Safety
/// Local APIC가 초기화된 후에 호출되어야 합니다.
pub unsafe fn init_io_apic() -> Result<(), &'static str> {
    // I/O APIC 메모리 매핑
    *IO_APIC_ADDR.lock() = Some(IO_APIC_BASE);
    
    // I/O APIC 버전 읽기
    let version = read_io_apic_reg(io_apic_reg::VER);
    let max_redirects = ((version >> 16) & 0xFF) + 1;
    
    crate::log_info!("I/O APIC initialized at 0x{:X}, max redirects: {}", 
                     IO_APIC_BASE, max_redirects);
    
    // TODO: 인터럽트 라우팅 설정
    // 각 IRQ를 적절한 벡터에 매핑
    
    Ok(())
}

/// Local APIC ID 반환
pub fn get_local_apic_id() -> u8 {
    unsafe {
        let id = read_local_apic_reg(local_apic_reg::ID);
        ((id >> 24) & 0xFF) as u8
    }
}

/// Local APIC EOI (End of Interrupt) 신호 전송
pub fn send_eoi() {
    unsafe {
        write_local_apic_reg(local_apic_reg::EOI, 0);
    }
}

/// Local APIC 레지스터 읽기
///
/// # Safety
/// Local APIC가 초기화된 후에 호출되어야 합니다.
unsafe fn read_local_apic_reg(offset: u32) -> u32 {
    let addr = LOCAL_APIC_ADDR.lock();
    if let Some(base) = *addr {
        let ptr = (base + offset as u64) as *const u32;
        read_volatile(ptr)
    } else {
        0
    }
}

/// Local APIC 레지스터 쓰기
///
/// # Safety
/// Local APIC가 초기화된 후에 호출되어야 합니다.
unsafe fn write_local_apic_reg(offset: u32, value: u32) {
    let addr = LOCAL_APIC_ADDR.lock();
    if let Some(base) = *addr {
        let ptr = (base + offset as u64) as *mut u32;
        write_volatile(ptr, value);
    }
}

/// I/O APIC 레지스터 읽기
unsafe fn read_io_apic_reg(reg: u8) -> u32 {
    let addr = IO_APIC_ADDR.lock();
    if let Some(base) = *addr {
        // IOREGSEL에 레지스터 인덱스 쓰기
        let sel_ptr = (base + io_apic_reg::IOREGSEL as u64) as *mut u32;
        write_volatile(sel_ptr, reg as u32);
        
        // IOWIN에서 값 읽기
        let win_ptr = (base + io_apic_reg::IOWIN as u64) as *const u32;
        read_volatile(win_ptr)
    } else {
        0
    }
}

/// I/O APIC 레지스터 쓰기
unsafe fn write_io_apic_reg(reg: u8, value: u32) {
    let addr = IO_APIC_ADDR.lock();
    if let Some(base) = *addr {
        // IOREGSEL에 레지스터 인덱스 쓰기
        let sel_ptr = (base + io_apic_reg::IOREGSEL as u64) as *mut u32;
        write_volatile(sel_ptr, reg as u32);
        
        // IOWIN에 값 쓰기
        let win_ptr = (base + io_apic_reg::IOWIN as u64) as *mut u32;
        write_volatile(win_ptr, value);
    }
}

/// APIC Base MSR 읽기
unsafe fn read_apic_base_msr() -> u64 {
    use x86_64::registers::model_specific::Msr;
    let msr = Msr::new(0x1B); // IA32_APIC_BASE
    msr.read()
}

/// APIC Base MSR 쓰기
unsafe fn write_apic_base_msr(value: u64) {
    use x86_64::registers::model_specific::Msr;
    let mut msr = Msr::new(0x1B); // IA32_APIC_BASE
    msr.write(value);
}

/// I/O APIC Redirection Table 엔트리 설정
///
/// # Arguments
/// * `irq` - IRQ 번호
/// * `vector` - 인터럽트 벡터
/// * `dest_apic_id` - 대상 CPU의 APIC ID
pub unsafe fn set_io_apic_redirect(irq: u8, vector: u8, dest_apic_id: u8) {
    let redirect_reg = io_apic_reg::REDTBL_BASE + (irq * 2);
    
    // Low 32비트: 벡터, 전달 모드, 목적지 모드, 극성, 트리거 모드
    let low = vector as u32;
    
    // High 32비트: 목적지 APIC ID
    let high = (dest_apic_id as u32) << 24;
    
    write_io_apic_reg(redirect_reg, low);
    write_io_apic_reg(redirect_reg + 1, high);
}

