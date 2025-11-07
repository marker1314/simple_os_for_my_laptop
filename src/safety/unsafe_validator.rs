//! Unsafe 블록 검증 및 문서화
//!
//! 이 모듈은 unsafe 블록의 안전성을 검증하고 문서화하는 도구를 제공합니다.
//!
//! # 사용 목적
//!
//! Rust의 unsafe 블록은 필수적이지만, 다음과 같은 검증이 필요합니다:
//! 1. 포인터 유효성 검사
//! 2. 메모리 범위 검증
//! 3. 하드웨어 접근 검증
//! 4. 동시성 안전성 확인

use core::fmt;

/// Unsafe 블록 타입
/// 
/// unsafe 블록의 사용 목적을 분류합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsafeBlockType {
    /// 포인터 역참조
    PointerDeref,
    /// 하드웨어 레지스터 접근
    HardwareAccess,
    /// 메모리 매핑
    MemoryMapping,
    /// FFI (Foreign Function Interface)
    FFI,
    /// 동시성 제어 (Mutex, Atomic 등)
    Concurrency,
    /// 기타
    Other,
}

impl fmt::Display for UnsafeBlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnsafeBlockType::PointerDeref => write!(f, "PointerDeref"),
            UnsafeBlockType::HardwareAccess => write!(f, "HardwareAccess"),
            UnsafeBlockType::MemoryMapping => write!(f, "MemoryMapping"),
            UnsafeBlockType::FFI => write!(f, "FFI"),
            UnsafeBlockType::Concurrency => write!(f, "Concurrency"),
            UnsafeBlockType::Other => write!(f, "Other"),
        }
    }
}

/// Unsafe 블록 검증 컨텍스트
/// 
/// unsafe 블록 실행 전후의 검증 정보를 저장합니다.
pub struct UnsafeContext {
    /// 블록 타입
    block_type: UnsafeBlockType,
    /// 파일명
    file: &'static str,
    /// 줄 번호
    line: u32,
    /// 설명
    description: &'static str,
    /// 검증 여부
    validated: bool,
}

impl UnsafeContext {
    /// 새 unsafe 컨텍스트 생성
    pub fn new(
        block_type: UnsafeBlockType,
        file: &'static str,
        line: u32,
        description: &'static str,
    ) -> Self {
        Self {
            block_type,
            file,
            line,
            description,
            validated: false,
        }
    }
    
    /// 검증 완료 표시
    pub fn mark_validated(&mut self) {
        self.validated = true;
    }
    
    /// 검증 여부 확인
    pub fn is_validated(&self) -> bool {
        self.validated
    }
    
    /// 블록 타입 가져오기
    pub fn block_type(&self) -> UnsafeBlockType {
        self.block_type
    }
}

/// 포인터 검증 헬퍼
/// 
/// unsafe 포인터 역참조 전에 검증합니다.
pub struct PointerValidator {
    ptr: u64,
    size: usize,
}

impl PointerValidator {
    /// 새 포인터 검증기 생성
    pub fn new(ptr: u64, size: usize) -> Self {
        Self { ptr, size }
    }
    
    /// 포인터 유효성 검증
    /// 
    /// # Safety
    /// 포인터가 유효한 메모리 영역을 가리켜야 합니다.
    pub unsafe fn validate(&self) -> Result<(), &'static str> {
        // Null 포인터 검사
        if self.ptr == 0 {
            return Err("Null pointer");
        }
        
        // 커널 공간 접근 방지
        let kernel_base = 0xFFFF800000000000u64;
        if self.ptr >= kernel_base {
            return Err("Kernel space access");
        }
        
        // 범위 검사
        let end_ptr = match self.ptr.checked_add(self.size as u64) {
            Some(end) => end,
            None => return Err("Pointer overflow"),
        };
        
        if end_ptr >= kernel_base {
            return Err("Buffer extends into kernel space");
        }
        
        Ok(())
    }
    
    /// 포인터를 *const T로 변환 (검증 후)
    /// 
    /// # Safety
    /// validate()가 성공한 후에만 호출해야 합니다.
    pub unsafe fn as_ptr<T>(&self) -> *const T {
        self.ptr as *const T
    }
    
    /// 포인터를 *mut T로 변환 (검증 후)
    /// 
    /// # Safety
    /// validate()가 성공한 후에만 호출해야 합니다.
    pub unsafe fn as_mut_ptr<T>(&self) -> *mut T {
        self.ptr as *mut T
    }
}

/// 하드웨어 레지스터 접근 검증
/// 
/// MMIO 레지스터 접근 전에 검증합니다.
pub struct HardwareValidator {
    /// 물리 주소
    phys_addr: u64,
    /// 크기
    size: usize,
}

impl HardwareValidator {
    /// 새 하드웨어 검증기 생성
    pub fn new(phys_addr: u64, size: usize) -> Self {
        Self { phys_addr, size }
    }
    
    /// 물리 주소 유효성 검증
    /// 
    /// # Safety
    /// 물리 주소가 유효한 하드웨어 레지스터를 가리켜야 합니다.
    pub unsafe fn validate(&self) -> Result<(), &'static str> {
        // 물리 주소 범위 검사
        // 일반적으로 MMIO는 0xE0000000 ~ 0xFFFFFFFF 범위
        let mmio_base = 0xE0000000u64;
        let mmio_end = 0xFFFFFFFFu64;
        
        // MMIO 범위가 아니면 경고 (하지만 허용)
        if self.phys_addr < mmio_base || self.phys_addr > mmio_end {
            crate::log_debug!("Hardware access at non-MMIO address: {:#016X}", self.phys_addr);
        }
        
        // 크기 검사 (일반적으로 1, 2, 4, 8 바이트)
        if !matches!(self.size, 1 | 2 | 4 | 8) {
            return Err("Invalid hardware access size");
        }
        
        Ok(())
    }
    
    /// 물리 주소를 가상 주소로 변환 (검증 후)
    /// 
    /// # Safety
    /// validate()가 성공한 후에만 호출해야 합니다.
    pub unsafe fn to_virt_addr(&self) -> u64 {
        // 물리 메모리 오프셋 추가
        let phys_offset = {
            use crate::memory::paging;
            let guard = paging::PHYSICAL_MEMORY_OFFSET.lock();
            guard.unwrap_or(paging::get_physical_memory_offset(
                crate::boot::get_boot_info().expect("boot info").into()
            ))
        };
        
        (phys_offset.as_u64()) + self.phys_addr
    }
}

/// Unsafe 블록 통계
/// 
/// unsafe 블록 사용 통계를 추적합니다.
pub struct UnsafeStats {
    /// 총 unsafe 블록 수
    total_blocks: u64,
    /// 검증된 블록 수
    validated_blocks: u64,
    /// 타입별 통계
    type_counts: [u64; 6],
}

impl UnsafeStats {
    /// 새 통계 생성
    pub fn new() -> Self {
        Self {
            total_blocks: 0,
            validated_blocks: 0,
            type_counts: [0; 6],
        }
    }
    
    /// Unsafe 블록 기록
    pub fn record_block(&mut self, block_type: UnsafeBlockType, validated: bool) {
        self.total_blocks += 1;
        if validated {
            self.validated_blocks += 1;
        }
        
        let index = match block_type {
            UnsafeBlockType::PointerDeref => 0,
            UnsafeBlockType::HardwareAccess => 1,
            UnsafeBlockType::MemoryMapping => 2,
            UnsafeBlockType::FFI => 3,
            UnsafeBlockType::Concurrency => 4,
            UnsafeBlockType::Other => 5,
        };
        self.type_counts[index] += 1;
    }
    
    /// 검증 비율 가져오기
    pub fn validation_rate(&self) -> f64 {
        if self.total_blocks == 0 {
            return 0.0;
        }
        (self.validated_blocks as f64) / (self.total_blocks as f64) * 100.0
    }
    
    /// 통계 출력
    pub fn print_stats(&self) {
        crate::log_info!("Unsafe Block Statistics:");
        crate::log_info!("  Total blocks: {}", self.total_blocks);
        crate::log_info!("  Validated blocks: {}", self.validated_blocks);
        crate::log_info!("  Validation rate: {:.2}%", self.validation_rate());
        crate::log_info!("  By type:");
        crate::log_info!("    PointerDeref: {}", self.type_counts[0]);
        crate::log_info!("    HardwareAccess: {}", self.type_counts[1]);
        crate::log_info!("    MemoryMapping: {}", self.type_counts[2]);
        crate::log_info!("    FFI: {}", self.type_counts[3]);
        crate::log_info!("    Concurrency: {}", self.type_counts[4]);
        crate::log_info!("    Other: {}", self.type_counts[5]);
    }
}

impl Default for UnsafeStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 전역 unsafe 통계
static UNSAFE_STATS: spin::Mutex<UnsafeStats> = spin::Mutex::new(UnsafeStats { total_blocks: 0, validated_blocks: 0, type_counts: [0; 6] });

/// Unsafe 블록 통계 기록
pub fn record_unsafe_block(block_type: UnsafeBlockType, validated: bool) {
    let mut stats = UNSAFE_STATS.lock();
    stats.record_block(block_type, validated);
}

/// Unsafe 블록 통계 출력
pub fn print_unsafe_stats() {
    let stats = UNSAFE_STATS.lock();
    stats.print_stats();
}

