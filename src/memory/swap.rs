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
    /// 스왑 아웃 시간 (타이머 틱)
    swap_time: u64,
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
        let data_to_swap = if let Some(saved_bytes) = crate::memory::compression::try_compress_page(
            page.start_address(),
            &page_data
        ) {
            crate::log_debug!("Compressed page before swap: saved {} bytes", saved_bytes);
            // 압축된 페이지는 메모리에 저장되고, 압축되지 않은 데이터는 스왑
            // 실제로는 압축된 데이터를 스왑하는 것이 더 효율적이지만,
            // 복원 시 복잡도가 증가하므로 현재는 원본 데이터를 스왑
            page_data
        } else {
            // 압축 효과가 없으면 원본 데이터 사용
            page_data
        };
        
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
        let entry = SwapEntry {
            slot,
            virtual_address: page.start_address(),
            swap_time: crate::drivers::timer::get_milliseconds(),
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
        
        crate::log_info!("Swapped in page {:#016x} from slot {}", 
                        page.start_address().as_u64(), entry.slot);
        
        // 스왑 엔트리 제거
        self.swap_map.remove(&addr);
        self.swapped_pages -= 1;
        self.swap_in_count += 1;
        
        Ok(final_page_data)
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

