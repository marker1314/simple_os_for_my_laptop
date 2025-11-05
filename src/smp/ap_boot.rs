//! AP (Application Processor) 부트 코드
//!
//! AP가 시작할 때 실행되는 코드입니다.

use x86_64::instructions::interrupts;

/// AP 부트 코드 주소 (1MB 미만, 4KB 정렬)
const AP_BOOT_CODE_ADDR: u64 = 0x1000;

/// AP 부트 코드 진입점
///
/// # Safety
/// 이 코드는 AP가 시작할 때 실행됩니다.
/// 페이지 테이블, 스택 등이 설정된 후에 실행되어야 합니다.
///
/// # Note
/// 실제 구현에서는 어셈블리 코드를 메모리에 복사해야 합니다.
/// 여기서는 C 함수로 래핑하여 구현합니다.
#[no_mangle]
pub unsafe extern "C" fn ap_boot_entry() -> ! {
    // 인터럽트 비활성화
    x86_64::instructions::interrupts::disable();
    
    // 페이지 테이블 설정 (BSP와 동일한 페이지 테이블 사용)
    use x86_64::registers::control::Cr3;
    let (frame, _) = Cr3::read();
    Cr3::write(frame, _);
    
    // 스택 설정 (임시, 이후 스케줄러가 할당)
    // 실제로는 각 AP마다 별도의 스택이 필요
    
    // Local APIC 초기화
    ap_init_local_apic();
    
    // AP 초기화 완료
    ap_init_complete();
    
    // 무한 루프 (스케줄러가 작업 할당할 때까지)
    loop {
        x86_64::instructions::hlt();
    }
}

/// AP Local APIC 초기화 (C 함수)
///
/// # Safety
/// AP가 시작할 때 호출됩니다.
#[no_mangle]
pub unsafe extern "C" fn ap_init_local_apic() {
    // Local APIC 초기화 (BSP와 동일)
    crate::smp::apic::init_local_apic().unwrap_or_else(|e| {
        crate::log_error!("AP Local APIC initialization failed: {}", e);
    });
}

/// AP 초기화 완료
///
/// # Safety
/// AP가 시작할 때 호출됩니다.
#[no_mangle]
pub unsafe extern "C" fn ap_init_complete() {
    let apic_id = crate::smp::apic::get_local_apic_id();
    crate::log_info!("AP with APIC ID {} initialized", apic_id);
    
    // 초기화 완료 플래그 설정
    crate::smp::set_ap_initialized(apic_id);
    
    // CPU 정보 등록
    let cpu_info = crate::smp::cpu::CpuInfo::new(apic_id as u64, false);
    crate::smp::register_cpu(cpu_info);
}

/// AP 부트 코드를 메모리에 복사
///
/// # Safety
/// 메모리 관리가 초기화된 후에 호출되어야 합니다.
pub unsafe fn prepare_ap_boot_code() -> Result<(), &'static str> {
    // AP 부트 코드를 메모리에 복사
    // 실제로는 어셈블리 코드를 바이너리에 포함시켜야 함
    // 여기서는 기본 구조만 제공
    
    // 부트 코드 페이지 매핑 확인
    let boot_page = x86_64::structures::paging::Page::<x86_64::structures::paging::Size4KiB>::containing_address(
        x86_64::VirtAddr::new(AP_BOOT_CODE_ADDR)
    );
    
    // 페이지가 매핑되어 있는지 확인
    // TODO: 페이지 매핑 확인 및 필요시 매핑
    
    crate::log_info!("AP boot code prepared at 0x{:X}", AP_BOOT_CODE_ADDR);
    
    Ok(())
}

/// AP 부트 코드 시작 페이지 번호 (4KB 단위)
pub fn ap_boot_code_page() -> u8 {
    (AP_BOOT_CODE_ADDR >> 12) as u8
}

