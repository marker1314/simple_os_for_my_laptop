//! SMP (Symmetric Multiprocessing) 지원 모듈
//!
//! 멀티코어 CPU 지원을 위한 모듈입니다.

pub mod apic;
pub mod cpu;
pub mod ipi;

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
    
    // 5. AP 초기화 (추가 CPU가 있는 경우)
    if cpu_count > 1 {
        crate::log_info!("Starting {} Application Processor(s)...", cpu_count - 1);
        // TODO: AP 초기화 코드 (IPI를 통한 INIT-SIPI-SIPI 시퀀스)
        // 현재는 기본 구조만 구현
    }
    
    *CPU_COUNT.lock() = cpu_count;
    
    Ok(())
}

/// CPU 수 감지
///
/// ACPI 테이블에서 CPU 수를 읽습니다.
fn detect_cpu_count() -> usize {
    // TODO: ACPI MADT (Multiple APIC Description Table)에서 CPU 수 읽기
    // 현재는 기본값 1 반환
    1
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


