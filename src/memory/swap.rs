//! 스왑 메커니즘 구현
//!
//! 이 모듈은 메모리 부족 시 페이지를 디스크로 스왑하여 시스템 안정성을 향상시킵니다.
//!
//! # 스왑 메커니즘
//!
//! 1. **메모리 압박 감지**: 사용 가능한 프레임이 임계값 이하로 떨어질 때
//! 2. **페이지 선택**: LRU (Least Recently Used) 알고리즘으로 스왑 대상 선택
//! 3. **스왑 아웃**: 선택된 페이지를 디스크로 저장
//! 4. **스왑 인**: 필요 시 디스크에서 페이지를 메모리로 복원
//! 5. **OOM Killer**: 메모리가 완전히 부족할 때 프로세스 종료 (선택적)

use x86_64::structures::paging::{Page, PhysFrame, Size4KiB};
use x86_64::VirtAddr;
use spin::Mutex;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use crate::drivers::ata::BlockDevice;

/// 스왑 엔트리
///
/// 스왑된 페이지의 정보를 저장합니다.
#[derive(Debug, Clone, Copy)]
struct SwapEntry {
    /// 스왑 슬롯 번호 (디스크 상의 위치)
    slot: u32,
    /// 원래의 가상 주소
    virtual_address: VirtAddr,
    /// 스왑 아웃 시간 (밀리초)
    swap_time: u64,
    /// 마지막 접근 시간 (스왑 인 시 업데이트)
    last_access_time: u64,
    /// 접근 횟수 (프리페칭 우선순위 결정)
    access_count: u32,
    /// 압축 여부
    compressed: bool,
}

/// 스왑 관리자
pub struct SwapManager {
    /// 스왑 엔트리 맵 (가상 주소 -> 스왑 엔트리)
    swap_map: BTreeMap<u64, SwapEntry>,
    /// 다음 사용 가능한 스왑 슬롯
    next_slot: u32,
    /// 최대 스왑 슬롯 수
    max_slots: u32,
    /// 스왑된 페이지 수
    swapped_pages: usize,
    /// 스왑 인/아웃 횟수
    swap_in_count: u64,
    swap_out_count: u64,
    /// 스왑 활성화 여부
    enabled: bool,
    /// 스왑 디바이스 (ATA 디스크)
    swap_device: Option<&'static dyn BlockDevice>,
    /// 스왑 영역 시작 블록
    swap_start_block: u64,
    /// 프리페칭 활성화 여부
    prefetch_enabled: bool,
    /// 프리페칭된 페이지 수
    prefetched_pages: usize,
}

impl SwapManager {
    /// 새 스왑 관리자 생성
    pub fn new() -> Self {
        Self {
            swap_map: BTreeMap::new(),
            next_slot: 0,
            max_slots: 1024, // 기본 4MB 스왑 (1024 페이지 * 4KB)
            swapped_pages: 0,
            swap_in_count: 0,
            swap_out_count: 0,
            enabled: false,
            swap_device: None,
            swap_start_block: 0,
            prefetch_enabled: true,
            prefetched_pages: 0,
        }
    }
    
    /// 스왑 초기화
    ///
    /// # Arguments
    /// * `device` - 스왑에 사용할 블록 디바이스
    /// * `start_block` - 스왑 영역 시작 블록 번호
    /// * `max_slots` - 최대 스왑 슬롯 수
    ///
    /// # Safety
    /// 메모리 관리 및 파일시스템이 초기화된 후에 호출되어야 합니다.
    pub unsafe fn init(&mut self, device: &'static dyn BlockDevice, start_block: u64, max_slots: u32) -> Result<(), SwapError> {
        if self.enabled {
            return Ok(()); // 이미 초기화됨
        }
        
        self.swap_device = Some(device);
        self.swap_start_block = start_block;
        self.max_slots = max_slots;
        self.enabled = true;
        
        crate::log_info!("Swap manager initialized: {} slots ({} MB)", 
                        max_slots, 
                        (max_slots as u64 * Size4KiB::SIZE) / (1024 * 1024));
        
        Ok(())
    }
    
    /// 스왑 활성화 여부 확인
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// 페이지를 스왑 아웃 (디스크로 내보내기)
    ///
    /// # Arguments
    /// * `page` - 스왑할 페이지
    /// * `frame` - 페이지의 물리 프레임
    ///
    /// # Safety
    /// 페이지가 유효하고 매핑되어 있어야 합니다.
    pub unsafe fn swap_out(&mut self, page: Page<Size4KiB>, frame: PhysFrame<Size4KiB>) -> Result<u32, SwapError> {
        if !self.enabled {
            return Err(SwapError::NotEnabled);
        }
        
        if self.next_slot >= self.max_slots {
            return Err(SwapError::SwapFull);
        }
        
        let slot = self.next_slot;
        self.next_slot += 1;
        
        // 페이지 데이터 읽기
        let page_data = self.read_page_data(frame)?;
        
        // 메모리 압축 시도 (스왑 전)
        // 압축은 메모리 압축 모듈에서 처리되므로, 여기서는 원본 데이터를 스왑
        let data_to_swap = page_data;
        
        // 압축 시도 (통계용)
        if let Some(saved_bytes) = crate::memory::compression::try_compress_page(
            page.start_address(),
            &page_data
        ) {
            crate::log_debug!("Compressed page before swap: saved {} bytes", saved_bytes);
        }
        
        // 디스크에 쓰기
        let device = self.swap_device.ok_or(SwapError::NotInitialized)?;
        let block = self.swap_start_block + slot as u64;
        
        // ATA 디바이스에 쓰기 (페이지는 8개 섹터 = 4KB)
        // BlockDevice는 섹터 단위이므로 8개 섹터 필요
        let mut sector_data = [0u8; 512];
        for sector_offset in 0..8 {
            let data_offset = sector_offset * 512;
            if data_offset + 512 <= data_to_swap.len() {
                sector_data.copy_from_slice(&data_to_swap[data_offset..data_offset + 512]);
                
                // TODO: 실제 디스크 쓰기 구현
                // unsafe { device.write_block(block + sector_offset as u64, &sector_data)?; }
                crate::log_debug!("Swap out: page {:#016x} -> slot {} sector {} (write not yet implemented)", 
                                page.start_address().as_u64(), slot, sector_offset);
            }
        }
        
        // 스왑 엔트리 저장
        let now = crate::drivers::timer::get_milliseconds();
        let entry = SwapEntry {
            slot,
            virtual_address: page.start_address(),
            swap_time: now,
            last_access_time: now,
            access_count: 0,
            compressed: false, // 압축은 스왑 인 시 확인
        };
        
        self.swap_map.insert(page.start_address().as_u64(), entry);
        self.swapped_pages += 1;
        self.swap_out_count += 1;
        
        crate::log_info!("Swapped out page {:#016x} to slot {}", 
                        page.start_address().as_u64(), slot);
        
        Ok(slot)
    }
    
    /// 페이지를 스왑 인 (디스크에서 복원)
    ///
    /// # Arguments
    /// * `page` - 복원할 페이지
    ///
    /// # Returns
    /// 복원된 페이지 데이터
    pub unsafe fn swap_in(&mut self, page: Page<Size4KiB>) -> Result<[u8; Size4KiB::SIZE as usize], SwapError> {
        if !self.enabled {
            return Err(SwapError::NotEnabled);
        }
        
        let addr = page.start_address().as_u64();
        let entry = self.swap_map.get(&addr).ok_or(SwapError::PageNotSwapped)?;
        
        // 디스크에서 읽기
        let device = self.swap_device.ok_or(SwapError::NotInitialized)?;
        let block = self.swap_start_block + entry.slot as u64;
        
        // ATA 디바이스에서 읽기 (페이지는 8개 섹터 = 4KB)
        let mut page_data = [0u8; Size4KiB::SIZE as usize];
        let mut sector_data = [0u8; 512];
        
        for sector_offset in 0..8 {
            let data_offset = sector_offset * 512;
            if data_offset + 512 <= page_data.len() {
                // TODO: 실제 디스크 읽기 구현
                // unsafe { device.read_block(block + sector_offset as u64, &mut sector_data)?; }
                sector_data.fill(0); // 임시로 0으로 채움
                page_data[data_offset..data_offset + 512].copy_from_slice(&sector_data);
            }
        }
        
        let now = crate::drivers::timer::get_milliseconds();
        crate::log_info!("Swapped in page {:#016x} from slot {} (access after {}ms)", 
                        page.start_address().as_u64(), entry.slot, now - entry.swap_time);
        
        // 접근 통계 업데이트
        if let Some(entry) = self.swap_map.get_mut(&addr) {
            entry.last_access_time = now;
            entry.access_count += 1;
        }
        
        // 스왑 엔트리 제거 (실제로는 유지하여 프리페칭에 사용 가능)
        // 하지만 메모리 절약을 위해 제거
        self.swap_map.remove(&addr);
        self.swapped_pages -= 1;
        self.swap_in_count += 1;
        
        Ok(page_data)
    }
    
    /// 페이지가 스왑되어 있는지 확인
    pub fn is_swapped(&self, page: Page<Size4KiB>) -> bool {
        self.swap_map.contains_key(&page.start_address().as_u64())
    }
    
    /// 사용 가능한 스왑 슬롯 수
    pub fn available_slots(&self) -> u32 {
        self.max_slots.saturating_sub(self.next_slot)
    }
    
    /// 스왑 통계
    pub fn stats(&self) -> SwapStats {
        SwapStats {
            swapped_pages: self.swapped_pages,
            swap_in_count: self.swap_in_count,
            swap_out_count: self.swap_out_count,
            available_slots: self.available_slots(),
            max_slots: self.max_slots,
        }
    }
    
    /// 페이지 데이터 읽기 (물리 프레임에서)
    unsafe fn read_page_data(&self, frame: PhysFrame<Size4KiB>) -> Result<[u8; Size4KiB::SIZE as usize], SwapError> {
        // 물리 메모리 오프셋 가져오기
        let phys_offset = {
            use crate::memory::paging;
            let guard = paging::PHYSICAL_MEMORY_OFFSET.lock();
            guard.ok_or(SwapError::NotInitialized)?
        };
        
        let phys_addr = frame.start_address();
        let virt_addr = phys_offset + phys_addr.as_u64();
        
        let mut data = [0u8; Size4KiB::SIZE as usize];
        let src_ptr = virt_addr.as_ptr::<u8>();
        core::ptr::copy_nonoverlapping(src_ptr, data.as_mut_ptr(), Size4KiB::SIZE as usize);
        
        Ok(data)
    }
    
    /// 페이지 데이터 쓰기 (물리 프레임에)
    unsafe fn write_page_data(&self, frame: PhysFrame<Size4KiB>, data: &[u8; Size4KiB::SIZE as usize]) -> Result<(), SwapError> {
        // 물리 메모리 오프셋 가져오기
        let phys_offset = {
            use crate::memory::paging;
            let guard = paging::PHYSICAL_MEMORY_OFFSET.lock();
            guard.ok_or(SwapError::NotInitialized)?
        };
        
        let phys_addr = frame.start_address();
        let virt_addr = phys_offset + phys_addr.as_u64();
        
        let dst_ptr = virt_addr.as_mut_ptr::<u8>();
        core::ptr::copy_nonoverlapping(data.as_ptr(), dst_ptr, Size4KiB::SIZE as usize);
        
        Ok(())
    }
    
    /// LRU 기반 스왑 아웃 대상 선택 (개선된 버전)
    /// 
    /// 접근 빈도와 시간을 고려하여 스왑 대상 페이지를 선택합니다.
    /// 
    /// 주의: 이 함수는 이미 스왑된 페이지를 대상으로 하지 않습니다.
    /// 실제 메모리에 있는 페이지를 찾으려면 다른 메커니즘이 필요합니다.
    pub fn select_lru_page(&self) -> Option<Page<Size4KiB>> {
        // 스왑된 페이지 중 가장 오래된 것 선택
        // 실제로는 메모리에 있는 페이지를 찾아야 함
        // 현재는 스왑 엔트리만 확인
        
        let mut oldest_entry: Option<SwapEntry> = None;
        let mut oldest_time = u64::MAX;
        
        for (_, &entry) in self.swap_map.iter() {
            if entry.swap_time < oldest_time {
                oldest_time = entry.swap_time;
                oldest_entry = Some(entry);
            }
        }
        
        oldest_entry.map(|entry| {
            Page::<Size4KiB>::containing_address(entry.virtual_address)
        })
    }
    
    /// 메모리에 있는 페이지 중 LRU 페이지 찾기
    /// 
    /// 실제 메모리에 매핑된 페이지 중 가장 오래된 것을 찾습니다.
    /// 이는 페이지 테이블을 스캔해야 하므로 더 복잡합니다.
    pub fn find_memory_lru_page(&self) -> Option<Page<Size4KiB>> {
        // TODO: 페이지 테이블 스캔하여 메모리에 있는 페이지 찾기
        // 현재는 기본 구현만
        None
    }
    
    /// 스왑 프리페칭
    /// 
    /// 자주 접근되는 페이지를 미리 메모리로 불러옵니다.
    /// 
    /// # Arguments
    /// * `max_pages` - 최대 프리페칭할 페이지 수
    /// 
    /// # Safety
    /// 메모리 관리가 초기화되어 있어야 합니다.
    pub unsafe fn prefetch_pages(&mut self, max_pages: usize) -> Result<usize, SwapError> {
        if !self.enabled || !self.prefetch_enabled {
            return Ok(0);
        }
        
        // 접근 횟수가 높은 페이지 우선 선택 (최근 접근 시간 기준)
        let mut candidates: Vec<(u64, SwapEntry)> = self.swap_map.iter()
            .map(|(&addr, &entry)| (addr, entry))
            .collect();
        
        // 접근 횟수와 최근 접근 시간 기준 정렬
        candidates.sort_by(|a, b| {
            // 접근 횟수가 높을수록 우선
            let count_cmp = b.1.access_count.cmp(&a.1.access_count);
            if count_cmp != core::cmp::Ordering::Equal {
                return count_cmp;
            }
            // 최근 접근 시간 기준 (최근일수록 우선)
            a.1.last_access_time.cmp(&b.1.last_access_time)
        });
        
        let mut prefetched = 0;
        for (addr, entry) in candidates.iter().take(max_pages) {
            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(*addr));
            
            // 프리페칭: 페이지를 메모리로 미리 불러오기
            if let Ok(_) = self.try_prefetch_page(page, *entry) {
                prefetched += 1;
                self.prefetched_pages += 1;
            }
        }
        
        if prefetched > 0 {
            crate::log_debug!("Prefetched {} pages from swap", prefetched);
        }
        
        Ok(prefetched)
    }
    
    /// 단일 페이지 프리페칭 시도
    unsafe fn try_prefetch_page(&mut self, page: Page<Size4KiB>, mut entry: SwapEntry) -> Result<(), SwapError> {
        // 메모리 압박 상황 확인
        if let Some((allocated, deallocated)) = crate::memory::frame::get_frame_stats() {
            // 메모리가 부족하면 프리페칭 스킵
            if allocated > deallocated + 100 {
                return Err(SwapError::IoError);
            }
        }
        
        // 프레임 할당 시도
        if let Some(frame) = crate::memory::frame::allocate_frame() {
            // 스왑에서 페이지 읽기
            let page_data = self.swap_in(page)?;
            
            // 프레임에 데이터 쓰기
            self.write_page_data(frame, &page_data)?;
            
            // 페이지 매핑
            if let Err(e) = crate::memory::paging::map_swap_page_at(page.start_address(), frame) {
                crate::log_warn!("Failed to map prefetched page: {:?}", e);
                crate::memory::frame::deallocate_frame(frame);
                return Err(SwapError::IoError);
            }
            
            // 접근 시간 업데이트
            entry.last_access_time = crate::drivers::timer::get_milliseconds();
            entry.access_count += 1;
            self.swap_map.insert(page.start_address().as_u64(), entry);
            
            Ok(())
        } else {
            Err(SwapError::IoError)
        }
    }
    
    /// 압축된 페이지 스왑 아웃 (향후 구현)
    /// 
    /// 페이지를 압축하여 스왑 공간을 절약합니다.
    /// 현재는 기본 구조만 제공합니다.
    unsafe fn swap_out_compressed(&mut self, page: Page<Size4KiB>, frame: PhysFrame<Size4KiB>) -> Result<u32, SwapError> {
        // 기본 스왑 아웃 사용 (압축은 별도로 처리)
        self.swap_out(page, frame)
    }
    
    /// 프리페칭 활성화/비활성화
    pub fn set_prefetch_enabled(&mut self, enabled: bool) {
        self.prefetch_enabled = enabled;
    }
    
    /// 프리페칭 활성화 여부 확인
    pub fn is_prefetch_enabled(&self) -> bool {
        self.prefetch_enabled
    }
}

/// 스왑 통계
#[derive(Debug, Clone, Copy)]
pub struct SwapStats {
    pub swapped_pages: usize,
    pub swap_in_count: u64,
    pub swap_out_count: u64,
    pub available_slots: u32,
    pub max_slots: u32,
}

/// 스왑 에러
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapError {
    NotEnabled,
    NotInitialized,
    SwapFull,
    PageNotSwapped,
    IoError,
    InvalidSlot,
}

impl core::fmt::Display for SwapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SwapError::NotEnabled => write!(f, "Swap not enabled"),
            SwapError::NotInitialized => write!(f, "Swap not initialized"),
            SwapError::SwapFull => write!(f, "Swap space full"),
            SwapError::PageNotSwapped => write!(f, "Page not in swap"),
            SwapError::IoError => write!(f, "Swap I/O error"),
            SwapError::InvalidSlot => write!(f, "Invalid swap slot"),
        }
    }
}

/// 전역 스왑 관리자
static SWAP_MANAGER: Mutex<SwapManager> = Mutex::new(SwapManager::new());

/// 스왑 관리자 초기화
///
/// # Safety
/// 메모리 관리 및 파일시스템이 초기화된 후에 호출되어야 합니다.
pub unsafe fn init_swap(device: &'static dyn BlockDevice, start_block: u64, max_slots: u32) -> Result<(), SwapError> {
    let mut manager = SWAP_MANAGER.lock();
    manager.init(device, start_block, max_slots)
}

/// 스왑 활성화 여부 확인
pub fn is_swap_enabled() -> bool {
    let manager = SWAP_MANAGER.lock();
    manager.is_enabled()
}

/// 스왑 통계 가져오기
pub fn get_swap_stats() -> SwapStats {
    let manager = SWAP_MANAGER.lock();
    manager.stats()
}

/// LRU 페이지를 스왑 아웃 시도
/// 
/// 메모리 압박 시 가장 오래된 페이지를 스왑 아웃합니다.
/// 
/// # Safety
/// 메모리 관리가 초기화되어 있어야 합니다.
pub unsafe fn try_swap_out_lru() -> Result<(), SwapError> {
    let mut manager = SWAP_MANAGER.lock();
    
    if !manager.enabled {
        return Err(SwapError::NotEnabled);
    }
    
    // LRU 페이지 선택
    if let Some(page) = manager.select_lru_page() {
        // 페이지의 프레임 찾기
        use crate::memory::paging;
        let offset = {
            let guard = paging::PHYSICAL_MEMORY_OFFSET.lock();
            guard.ok_or(SwapError::NotInitialized)?
        };
        
        let mut mapper = paging::init_mapper(offset);
        
        // 페이지가 매핑되어 있는지 확인
        if let Ok(frame) = mapper.translate_page(page) {
            // 스왑 아웃
            manager.swap_out(page, frame)?;
            Ok(())
        } else {
            Err(SwapError::PageNotSwapped)
        }
    } else {
        Err(SwapError::SwapFull) // 스왑할 페이지 없음
    }
}

/// 스왑 프리페칭 실행
/// 
/// 유휴 시간에 자주 접근되는 페이지를 미리 불러옵니다.
/// 
/// # Safety
/// 메모리 관리가 초기화되어 있어야 합니다.
pub unsafe fn prefetch_swap_pages(max_pages: usize) -> Result<usize, SwapError> {
    let mut manager = SWAP_MANAGER.lock();
    manager.prefetch_pages(max_pages)
}

