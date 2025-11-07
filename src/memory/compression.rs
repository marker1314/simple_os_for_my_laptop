//! 메모리 압축 메커니즘
//!
//! 이 모듈은 스왑 전에 메모리 압축을 시도하여 메모리를 절약합니다.
//!
//! # 압축 전략
//!
//! 1. **페이지 압축**: 사용되지 않는 페이지를 압축하여 메모리 절약
//! 2. **스왑 전 압축**: 스왑 아웃 전에 압축 시도
//! 3. **압축률 모니터링**: 압축 효과 추적

use x86_64::structures::paging::{PhysFrame, Size4KiB};
use x86_64::VirtAddr;
use spin::Mutex;
use alloc::vec::Vec;
use alloc::vec;
use alloc::collections::BTreeMap;

// Do not access private FRAME_ALLOCATOR here; compression doesn't need it.

/// 압축된 페이지 엔트리
#[derive(Debug, Clone)]
struct CompressedPage {
    /// 원본 가상 주소
    virtual_addr: VirtAddr,
    /// 압축된 데이터
    compressed_data: Vec<u8>,
    /// 원본 크기 (바이트)
    original_size: usize,
    /// 압축된 크기 (바이트)
    compressed_size: usize,
    /// 압축 시간 (밀리초)
    compress_time: u64,
}

/// 메모리 압축 관리자
pub struct MemoryCompressor {
    /// 압축된 페이지 목록
    compressed_pages: BTreeMap<VirtAddr, CompressedPage>,
    /// 압축 통계
    total_compressed: u64,
    total_decompressed: u64,
    total_bytes_saved: u64,
    /// 최대 압축 페이지 수
    max_compressed_pages: usize,
}

impl MemoryCompressor {
    /// 새 압축 관리자 생성
    pub fn new(max_pages: usize) -> Self {
        Self {
            compressed_pages: BTreeMap::new(),
            total_compressed: 0,
            total_decompressed: 0,
            total_bytes_saved: 0,
            max_compressed_pages: max_pages,
        }
    }
    
    /// 페이지 압축 시도
    ///
    /// # Arguments
    /// * `virtual_addr` - 압축할 페이지의 가상 주소
    /// * `data` - 페이지 데이터 (4KB)
    ///
    /// # Returns
    /// 압축 성공 시 절약된 바이트 수, 실패 시 None
    pub fn try_compress_page(&mut self, virtual_addr: VirtAddr, data: &[u8]) -> Option<usize> {
        // 최대 페이지 수 확인
        if self.compressed_pages.len() >= self.max_compressed_pages {
            // 가장 오래된 페이지 해제
            if let Some((oldest_addr, _)) = self.compressed_pages.iter()
                .min_by_key(|(_, page)| page.compress_time) {
                let oldest = *oldest_addr;
                self.compressed_pages.remove(&oldest);
            }
        }
        
        // 간단한 압축 알고리즘: RLE (Run-Length Encoding) + Zero-page 최적화
        let compressed = compress_simple(data);
        let original_size = data.len();
        let compressed_size = compressed.len();
        
        // 압축 효과가 있는 경우만 저장 (최소 10% 압축)
        if compressed_size < original_size * 9 / 10 {
            let saved = original_size - compressed_size;
            
            self.compressed_pages.insert(virtual_addr, CompressedPage {
                virtual_addr,
                compressed_data: compressed,
                original_size,
                compressed_size,
                compress_time: crate::drivers::timer::get_milliseconds(),
            });
            
            self.total_compressed += 1;
            self.total_bytes_saved += saved as u64;
            
            crate::log_debug!("Compressed page at {:#016x}: {} -> {} bytes (saved: {})", 
                            virtual_addr.as_u64(), original_size, compressed_size, saved);
            
            Some(saved)
        } else {
            // 압축 효과가 없음
            None
        }
    }
    
    /// 압축된 페이지 복원
    ///
    /// # Arguments
    /// * `virtual_addr` - 복원할 페이지의 가상 주소
    ///
    /// # Returns
    /// 복원 성공 시 원본 데이터, 실패 시 None
    pub fn decompress_page(&mut self, virtual_addr: VirtAddr) -> Option<Vec<u8>> {
        if let Some(compressed_page) = self.compressed_pages.remove(&virtual_addr) {
            let decompressed = decompress_simple(&compressed_page.compressed_data, compressed_page.original_size);
            
            self.total_decompressed += 1;
            
            crate::log_debug!("Decompressed page at {:#016x}: {} -> {} bytes", 
                            virtual_addr.as_u64(), compressed_page.compressed_size, decompressed.len());
            
            Some(decompressed)
        } else {
            None
        }
    }
    
    /// 압축된 페이지가 있는지 확인
    pub fn is_compressed(&self, virtual_addr: VirtAddr) -> bool {
        self.compressed_pages.contains_key(&virtual_addr)
    }
    
    /// 압축 통계 가져오기
    pub fn get_stats(&self) -> (u64, u64, u64, usize) {
        (
            self.total_compressed,
            self.total_decompressed,
            self.total_bytes_saved,
            self.compressed_pages.len(),
        )
    }
    
    /// 오래된 압축 페이지 정리
    pub fn cleanup_old_compressed(&mut self, max_age_ms: u64) {
        let now = crate::drivers::timer::get_milliseconds();
        self.compressed_pages.retain(|_, page| {
            now.saturating_sub(page.compress_time) < max_age_ms
        });
    }
    
    /// 모든 압축된 페이지 해제
    pub fn clear(&mut self) {
        self.compressed_pages.clear();
    }
}

/// 간단한 압축 알고리즘 (RLE + Zero-page 최적화)
///
/// 실제로는 더 복잡한 알고리즘 (LZ4, Zstd 등)을 사용할 수 있지만,
/// 커널에서는 단순하고 빠른 알고리즘이 중요합니다.
fn compress_simple(data: &[u8]) -> Vec<u8> {
    // Zero-page 최적화: 모든 바이트가 0이면 1바이트만 저장
    if data.iter().all(|&b| b == 0) {
        return vec![0; 1]; // 0 = zero page
    }
    
    // RLE (Run-Length Encoding) - 간단한 구현
    let mut compressed = Vec::new();
    let mut i = 0;
    
    while i < data.len() {
        let byte = data[i];
        let mut count = 1;
        
        // 연속된 동일 바이트 개수 세기 (최대 255)
        while i + count < data.len() && count < 255 && data[i + count] == byte {
            count += 1;
        }
        
        if count > 3 || byte == 0 {
            // RLE 인코딩: 0x00 (RLE 마커), count, byte
            compressed.push(0x00); // RLE 마커
            compressed.push(count as u8);
            compressed.push(byte);
            i += count;
        } else {
            // 일반 바이트
            compressed.push(byte);
            i += 1;
        }
    }
    
    compressed
}

/// 간단한 압축 해제
fn decompress_simple(compressed: &[u8], original_size: usize) -> Vec<u8> {
    // Zero-page 처리
    if compressed.len() == 1 && compressed[0] == 0 {
        return vec![0; original_size];
    }
    
    let mut decompressed = Vec::with_capacity(original_size);
    let mut i = 0;
    
    while i < compressed.len() {
        if compressed[i] == 0x00 && i + 2 < compressed.len() {
            // RLE 디코딩
            let count = compressed[i + 1] as usize;
            let byte = compressed[i + 2];
            decompressed.extend(vec![byte; count]);
            i += 3;
        } else {
            // 일반 바이트
            decompressed.push(compressed[i]);
            i += 1;
        }
    }
    
    // 원본 크기까지 0으로 패딩
    while decompressed.len() < original_size {
        decompressed.push(0);
    }
    
    decompressed
}

/// 전역 메모리 압축 관리자
static MEMORY_COMPRESSOR: Mutex<MemoryCompressor> = Mutex::new(MemoryCompressor {
    compressed_pages: BTreeMap::new(),
    total_compressed: 0,
    total_decompressed: 0,
    total_bytes_saved: 0,
    max_compressed_pages: 32, // 최대 32개 페이지 압축 (128KB)
});

/// 메모리 압축 초기화
pub fn init_compression(max_pages: usize) {
    let mut compressor = MEMORY_COMPRESSOR.lock();
    *compressor = MemoryCompressor::new(max_pages);
    crate::log_info!("Memory compression initialized (max pages: {})", max_pages);
}

/// 페이지 압축 시도
///
/// 스왑 아웃 전에 호출하여 메모리를 압축합니다.
pub fn try_compress_page(virtual_addr: VirtAddr, data: &[u8]) -> Option<usize> {
    let mut compressor = MEMORY_COMPRESSOR.lock();
    compressor.try_compress_page(virtual_addr, data)
}

/// 압축된 페이지 복원
pub fn decompress_page(virtual_addr: VirtAddr) -> Option<Vec<u8>> {
    let mut compressor = MEMORY_COMPRESSOR.lock();
    compressor.decompress_page(virtual_addr)
}

/// 압축된 페이지 확인
pub fn is_page_compressed(virtual_addr: VirtAddr) -> bool {
    let compressor = MEMORY_COMPRESSOR.lock();
    compressor.is_compressed(virtual_addr)
}

/// 압축 통계 가져오기
pub fn get_compression_stats() -> (u64, u64, u64, usize) {
    let compressor = MEMORY_COMPRESSOR.lock();
    compressor.get_stats()
}

/// 오래된 압축 페이지 정리
pub fn cleanup_compressed_pages(max_age_ms: u64) {
    let mut compressor = MEMORY_COMPRESSOR.lock();
    compressor.cleanup_old_compressed(max_age_ms);
}

