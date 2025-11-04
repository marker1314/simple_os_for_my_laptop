//! 파일시스템 캐시
//!
//! 파일 시스템 성능 향상을 위한 블록 캐시 구현

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;

/// 캐시 블록 크기 (512바이트 - 1섹터)
pub const BLOCK_SIZE: usize = 512;

/// 최대 캐시 블록 수
pub const MAX_CACHE_BLOCKS: usize = 256;

/// 캐시 블록
#[derive(Clone)]
pub struct CacheBlock {
    /// 블록 번호
    pub block_num: u64,
    /// 블록 데이터
    pub data: [u8; BLOCK_SIZE],
    /// 수정 여부 (dirty)
    pub dirty: bool,
    /// 마지막 접근 시간 (틱)
    pub last_access: u64,
}

impl CacheBlock {
    /// 새 캐시 블록 생성
    pub fn new(block_num: u64) -> Self {
        Self {
            block_num,
            data: [0; BLOCK_SIZE],
            dirty: false,
            last_access: 0,
        }
    }
    
    /// 데이터 읽기
    pub fn read(&self) -> &[u8] {
        &self.data
    }
    
    /// 데이터 쓰기
    pub fn write(&mut self, data: &[u8]) {
        let len = data.len().min(BLOCK_SIZE);
        self.data[..len].copy_from_slice(&data[..len]);
        self.dirty = true;
    }
    
    /// 접근 시간 업데이트
    pub fn touch(&mut self, time: u64) {
        self.last_access = time;
    }
    
    /// 수정 여부 확인
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// 수정 플래그 초기화
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

/// 블록 캐시
pub struct BlockCache {
    /// 캐시된 블록들 (블록 번호 -> 캐시 블록)
    blocks: BTreeMap<u64, CacheBlock>,
    /// 현재 시간 (틱)
    current_time: u64,
    /// 히트 수
    hits: usize,
    /// 미스 수
    misses: usize,
}

impl BlockCache {
    /// 새 블록 캐시 생성
    pub fn new() -> Self {
        Self {
            blocks: BTreeMap::new(),
            current_time: 0,
            hits: 0,
            misses: 0,
        }
    }
    
    /// 블록 가져오기
    ///
    /// # Arguments
    /// * `block_num` - 블록 번호
    ///
    /// # Returns
    /// 캐시된 블록 (없으면 None)
    pub fn get(&mut self, block_num: u64) -> Option<&mut CacheBlock> {
        self.current_time += 1;
        
        if let Some(block) = self.blocks.get_mut(&block_num) {
            block.touch(self.current_time);
            self.hits += 1;
            Some(block)
        } else {
            self.misses += 1;
            None
        }
    }
    
    /// 블록 추가/업데이트
    ///
    /// # Arguments
    /// * `block` - 추가할 블록
    pub fn put(&mut self, mut block: CacheBlock) {
        self.current_time += 1;
        block.touch(self.current_time);
        
        // 캐시가 가득 찬 경우 가장 오래된 블록 제거 (LRU)
        if self.blocks.len() >= MAX_CACHE_BLOCKS {
            self.evict_lru();
        }
        
        self.blocks.insert(block.block_num, block);
    }
    
    /// LRU (Least Recently Used) 블록 제거
    fn evict_lru(&mut self) {
        if let Some((&block_num, _)) = self.blocks.iter()
            .min_by_key(|(_, block)| block.last_access)
        {
            self.blocks.remove(&block_num);
        }
    }
    
    /// 모든 dirty 블록 가져오기
    ///
    /// # Returns
    /// dirty 블록 번호 리스트
    pub fn get_dirty_blocks(&self) -> Vec<u64> {
        self.blocks.iter()
            .filter(|(_, block)| block.is_dirty())
            .map(|(&block_num, _)| block_num)
            .collect()
    }
    
    /// 특정 블록을 clean으로 표시
    pub fn mark_clean(&mut self, block_num: u64) {
        if let Some(block) = self.blocks.get_mut(&block_num) {
            block.clear_dirty();
        }
    }
    
    /// 캐시 초기화
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.hits = 0;
        self.misses = 0;
    }
    
    /// 캐시 통계
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.blocks.len(),
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                (self.hits as f64) / ((self.hits + self.misses) as f64)
            } else {
                0.0
            },
        }
    }
}

/// 캐시 통계 정보
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 캐시된 블록 수
    pub size: usize,
    /// 히트 수
    pub hits: usize,
    /// 미스 수
    pub misses: usize,
    /// 히트율
    pub hit_rate: f64,
}

impl core::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Cache: {} blocks, {} hits, {} misses, {:.2}% hit rate",
               self.size, self.hits, self.misses, self.hit_rate * 100.0)
    }
}

/// 전역 블록 캐시
static GLOBAL_CACHE: Mutex<Option<BlockCache>> = Mutex::new(None);

/// 블록 캐시 초기화
pub fn init() {
    let mut cache = GLOBAL_CACHE.lock();
    *cache = Some(BlockCache::new());
    crate::log_info!("Block cache initialized");
}

/// 캐시에서 블록 가져오기
pub fn get_cached_block(block_num: u64) -> Option<[u8; BLOCK_SIZE]> {
    let mut cache = GLOBAL_CACHE.lock();
    if let Some(ref mut c) = *cache {
        c.get(block_num).map(|block| block.data)
    } else {
        None
    }
}

/// 캐시에 블록 추가
pub fn cache_block(block_num: u64, data: &[u8]) {
    let mut cache = GLOBAL_CACHE.lock();
    if let Some(ref mut c) = *cache {
        let mut block = CacheBlock::new(block_num);
        block.write(data);
        c.put(block);
    }
}

/// dirty 블록 리스트 가져오기
pub fn get_dirty_blocks() -> Vec<u64> {
    let cache = GLOBAL_CACHE.lock();
    if let Some(ref c) = *cache {
        c.get_dirty_blocks()
    } else {
        Vec::new()
    }
}

/// 캐시 통계 가져오기
pub fn get_cache_stats() -> Option<CacheStats> {
    let cache = GLOBAL_CACHE.lock();
    cache.as_ref().map(|c| c.stats())
}

