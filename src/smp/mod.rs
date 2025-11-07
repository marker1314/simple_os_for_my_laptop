//! SMP (Symmetric Multiprocessing) 지원 모듈
//!
//! 멀티코어 CPU 지원을 위한 모듈입니다.

pub mod apic;
pub mod cpu;
pub mod ipi;
mod ap_boot;

use alloc::vec::Vec;
use spin::Mutex;
use crate::smp::cpu::CpuInfo;

/// 전역 CPU 정보 리스트
static CPUS: Mutex<Vec<CpuInfo>> = Mutex::new(Vec::new());

/// 현재 CPU 수
static CPU_COUNT: Mutex<usize> = Mutex::new(0);

/// SMP 시스템 초기화
///
/// # Safety
/// 메모리 관리와 인터럽트가 초기화된 후에 호출되어야 합니다.
pub unsafe fn init() -> Result<(), &'static str> {
    crate::log_info!("Initializing SMP support...");
    
    // 1. Local APIC 초기화 (BSP - Bootstrap Processor)
    apic::init_local_apic()?;
    crate::log_info!("Local APIC initialized on BSP");
    
    // 2. I/O APIC 초기화
    apic::init_io_apic()?;
    crate::log_info!("I/O APIC initialized");
    
    // 3. BSP CPU 정보 등록
    let bsp_info = cpu::CpuInfo::new(0, true);
    CPUS.lock().push(bsp_info);
    *CPU_COUNT.lock() = 1;
    
    // 4. AP (Application Processor) 초기화 준비
    // ACPI 테이블에서 CPU 정보 읽기
    let cpu_count = detect_cpu_count();
    crate::log_info!("Detected {} CPU(s)", cpu_count);
    
    // 5. AP 부트 코드 준비
    ap_boot::prepare_ap_boot_code()?;
    
    // 6. AP 초기화 (추가 CPU가 있는 경우)
    if cpu_count > 1 {
        crate::log_info!("Starting {} Application Processor(s)...", cpu_count - 1);
        
        // ACPI MADT에서 APIC ID 목록 읽기 (간단한 구현)
        let ap_apic_ids = detect_ap_apic_ids(cpu_count);
        
        for apic_id in ap_apic_ids {
            if let Err(e) = init_application_processor(apic_id) {
                crate::log_warn!("Failed to initialize AP with APIC ID {}: {}", apic_id, e);
            } else {
                crate::log_info!("AP with APIC ID {} initialized successfully", apic_id);
            }
        }
    }
    
    *CPU_COUNT.lock() = cpu_count;
    
    Ok(())
}

/// CPU 수 감지
///
/// ACPI 테이블에서 CPU 수를 읽습니다.
fn detect_cpu_count() -> usize {
    // TODO: ACPI MADT (Multiple APIC Description Table)에서 CPU 수 읽기
    // 현재는 CPUID를 통해 논리 프로세서 수 확인
    unsafe {
        use core::arch::x86_64::{__cpuid, __cpuid_count};
        let max_leaf = __cpuid(0).eax;
        if max_leaf >= 0xB {
            let r = __cpuid_count(0xB, 0);
            let logical = r.ebx & 0xFFFF;
            if logical > 0 { return logical as usize; }
        }
        1
    }
}

/// AP APIC ID 목록 감지
fn detect_ap_apic_ids(total_cpus: usize) -> Vec<u8> {
    // TODO: ACPI MADT에서 APIC ID 목록 읽기
    // 현재는 기본 구현 (BSP APIC ID 제외)
    let bsp_apic_id = apic::get_local_apic_id();
    let mut apic_ids = Vec::new();
    
    // 간단한 구현: 1부터 시작하는 연속된 APIC ID 가정
    for i in 0..total_cpus {
        let apic_id = i as u8;
        if apic_id != bsp_apic_id {
            apic_ids.push(apic_id);
        }
    }
    
    apic_ids
}

/// Application Processor 초기화
///
/// # Arguments
/// * `apic_id` - 초기화할 AP의 APIC ID
///
/// # Safety
/// SMP 초기화 중에만 호출되어야 합니다.
unsafe fn init_application_processor(apic_id: u8) -> Result<(), &'static str> {
    crate::log_info!("Initializing AP with APIC ID {}...", apic_id);
    
    // 1. INIT IPI 전송 (CPU 리셋)
    ipi::send_init_ipi(apic_id);
    
    // INIT IPI 전송 후 10ms 대기
    let start_ms = crate::drivers::timer::get_milliseconds();
    while crate::drivers::timer::get_milliseconds() - start_ms < 10 {
        core::hint::spin_loop();
    }
    
    // 2. SIPI 전송 (첫 번째)
    let boot_page = ap_boot::ap_boot_code_page();
    ipi::send_startup_ipi(apic_id, boot_page);
    
    // SIPI 전송 후 200us 대기
    let start_ms = crate::drivers::timer::get_milliseconds();
    while crate::drivers::timer::get_milliseconds() - start_ms < 1 {
        core::hint::spin_loop();
    }
    
    // 3. SIPI 전송 (두 번째, 첫 번째가 실패했을 경우)
    // AP가 시작되지 않았는지 확인
    let mut timeout = 1000; // 1초 타임아웃
    while timeout > 0 {
        // AP가 시작되었는지 확인 (AP가 초기화 완료 플래그 설정)
        if is_ap_initialized(apic_id) {
            break;
        }
        
        timeout -= 1;
        for _ in 0..100 {
            core::hint::spin_loop();
        }
    }
    
    if timeout == 0 {
        // 두 번째 SIPI 전송
        ipi::send_startup_ipi(apic_id, boot_page);
        
        // 추가 대기
        timeout = 1000;
        while timeout > 0 {
            if is_ap_initialized(apic_id) {
                break;
            }
            
            timeout -= 1;
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }
        
        if timeout == 0 {
            return Err("AP initialization timeout");
        }
    }
    
    Ok(())
}

/// AP 초기화 완료 플래그 (APIC ID별)
static AP_INIT_FLAGS: Mutex<alloc::collections::BTreeMap<u8, bool>> = Mutex::new(alloc::collections::BTreeMap::new());

/// AP 초기화 완료 여부 확인
fn is_ap_initialized(apic_id: u8) -> bool {
    let flags = AP_INIT_FLAGS.lock();
    flags.get(&apic_id).copied().unwrap_or(false)
}

/// AP 초기화 완료 플래그 설정
pub fn set_ap_initialized(apic_id: u8) {
    let mut flags = AP_INIT_FLAGS.lock();
    flags.insert(apic_id, true);
}

/// CPU 등록 (AP에서 호출)
pub fn register_cpu(cpu_info: CpuInfo) {
    let mut cpus = CPUS.lock();
    cpus.push(cpu_info);
    *CPU_COUNT.lock() = cpus.len();
}

/// 현재 활성화된 CPU 수 반환
pub fn cpu_count() -> usize {
    *CPU_COUNT.lock()
}

/// 현재 CPU ID 반환
///
/// Local APIC ID를 읽어 현재 실행 중인 CPU를 식별합니다.
pub fn current_cpu_id() -> u8 {
    apic::get_local_apic_id()
}

/// 모든 CPU에 작업 분배
///
/// # Arguments
/// * `func` - 각 CPU에서 실행할 함수
pub fn broadcast_work<F>(func: F)
where
    F: Fn(u8) + Send + Sync,
{
    let count = cpu_count();
    for cpu_id in 0..count {
        func(cpu_id as u8);
    }
}

/// 특정 CPU에 인터럽트 전송 (IPI - Inter-Processor Interrupt)
///
/// # Arguments
/// * `cpu_id` - 대상 CPU ID
/// * `vector` - 인터럽트 벡터 번호
pub fn send_ipi(cpu_id: u8, vector: u8) {
    ipi::send_ipi(cpu_id, vector);
}

/// 모든 CPU에 인터럽트 브로드캐스트
///
/// # Arguments
/// * `vector` - 인터럽트 벡터 번호
pub fn broadcast_ipi(vector: u8) {
    ipi::broadcast_ipi(vector);
}


