//! 메모리 맵 파싱 모듈
//!
//! 이 모듈은 부트로더로부터 받은 메모리 맵을 파싱하고 분류합니다.

use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use x86_64::PhysAddr;

/// 메모리 영역 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// 사용 가능한 메모리
    Usable,
    /// 예약된 메모리 (하드웨어나 펌웨어에 의해 사용됨)
    Reserved,
    /// 커널 코드 및 데이터
    Kernel,
    /// 부트로더 코드 및 데이터
    Bootloader,
}

/// 파싱된 메모리 영역
#[derive(Debug, Clone, Copy)]
pub struct ParsedMemoryRegion {
    /// 시작 주소 (물리 주소)
    pub start: PhysAddr,
    /// 길이 (바이트)
    pub length: u64,
    /// 메모리 타입
    pub memory_type: MemoryType,
}

/// 메모리 맵 파싱 및 분석
pub struct MemoryMap {
    regions: &'static [MemoryRegion],
}

impl MemoryMap {
    /// 부트 정보로부터 메모리 맵 생성
    ///
    /// # Safety
    /// `memory_regions`는 유효한 메모리 맵을 가리켜야 합니다.
    pub unsafe fn new(memory_regions: &'static [MemoryRegion]) -> Self {
        Self { regions: memory_regions }
    }

    /// 모든 메모리 영역 반복자
    pub fn iter(&self) -> impl Iterator<Item = ParsedMemoryRegion> + '_ {
        self.regions.iter().map(|region| Self::parse_region(region))
    }

    /// 사용 가능한 메모리 영역만 반복자
    pub fn usable_regions(&self) -> impl Iterator<Item = ParsedMemoryRegion> + '_ {
        self.iter()
            .filter(|region| region.memory_type == MemoryType::Usable)
    }

    /// 예약된 메모리 영역만 반복자
    pub fn reserved_regions(&self) -> impl Iterator<Item = ParsedMemoryRegion> + '_ {
        self.iter()
            .filter(|region| region.memory_type == MemoryType::Reserved)
    }

    /// 커널 메모리 영역만 반복자
    pub fn kernel_regions(&self) -> impl Iterator<Item = ParsedMemoryRegion> + '_ {
        self.iter()
            .filter(|region| region.memory_type == MemoryType::Kernel)
    }

    /// 부트로더 메모리 영역만 반복자
    pub fn bootloader_regions(&self) -> impl Iterator<Item = ParsedMemoryRegion> + '_ {
        self.iter()
            .filter(|region| region.memory_type == MemoryType::Bootloader)
    }

    /// 전체 사용 가능한 메모리 크기 계산 (바이트)
    pub fn total_usable_memory(&self) -> u64 {
        self.usable_regions().map(|r| r.length).sum()
    }

    /// 메모리 영역 파싱
    fn parse_region(region: &MemoryRegion) -> ParsedMemoryRegion {
        let start = PhysAddr::new(region.start);
        let length = region.len;
        let memory_type = match region.kind {
            MemoryRegionKind::Usable => MemoryType::Usable,
            MemoryRegionKind::Reserved => MemoryType::Reserved,
            MemoryRegionKind::Kernel => MemoryType::Kernel,
            MemoryRegionKind::Bootloader => MemoryType::Bootloader,
        };

        ParsedMemoryRegion {
            start,
            length,
            memory_type,
        }
    }

    /// 원본 메모리 영역 슬라이스 반환
    pub fn raw_regions(&self) -> &'static [MemoryRegion] {
        self.regions
    }
}

/// 전역 메모리 맵 인스턴스
static mut MEMORY_MAP: Option<MemoryMap> = None;

/// 메모리 맵 초기화
///
/// # Safety
/// `memory_regions`는 유효한 메모리 맵을 가리켜야 하며, 이 함수는 한 번만 호출되어야 합니다.
pub unsafe fn init(memory_regions: &'static [MemoryRegion]) {
    MEMORY_MAP = Some(MemoryMap::new(memory_regions));
}

/// 메모리 맵 가져오기
///
/// # Safety
/// `init`이 먼저 호출되어야 합니다.
pub unsafe fn get() -> &'static MemoryMap {
    MEMORY_MAP.as_ref().expect("Memory map not initialized")
}

