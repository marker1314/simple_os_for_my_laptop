//! ASLR (Address Space Layout Randomization) 구현
//!
//! 이 모듈은 메모리 보안을 강화하기 위해 주소 공간 레이아웃을 랜덤화합니다.
//!
//! # ASLR 구현
//!
//! 1. **커널 시작 시 랜덤 오프셋 생성**: 페이지 테이블, 스택, 힙 주소 랜덤화
//! 2. **프로세스별 랜덤 주소**: 각 프로세스의 메모리 영역을 랜덤하게 배치
//! 3. **스택 랜덤화**: 스택 시작 주소를 랜덤하게 설정
//! 4. **힙 랜덤화**: 힙 시작 주소를 랜덤하게 설정
//! 5. **엔트로피 풀**: 여러 하드웨어 소스에서 엔트로피 수집 및 저장

use x86_64::VirtAddr;
use spin::Mutex;

/// 엔트로피 풀 크기 (바이트)
const ENTROPY_POOL_SIZE: usize = 64;
/// 엔트로피 풀 최소 엔트로피 (비트)
const MIN_ENTROPY_BITS: usize = 128;

/// 엔트로피 풀
/// 
/// 여러 하드웨어 소스에서 수집한 엔트로피를 저장하고 관리합니다.
struct EntropyPool {
    /// 엔트로피 데이터
    pool: [u8; ENTROPY_POOL_SIZE],
    /// 수집된 엔트로피 비트 수 (추정)
    entropy_bits: usize,
    /// 엔트로피 풀 위치 (읽기 위치)
    position: usize,
}

impl EntropyPool {
    /// 새 엔트로피 풀 생성
    fn new() -> Self {
        Self {
            pool: [0u8; ENTROPY_POOL_SIZE],
            entropy_bits: 0,
            position: 0,
        }
    }
    
    /// 엔트로피 추가
    /// 
    /// # Arguments
    /// * `data` - 추가할 엔트로피 데이터
    /// * `estimated_bits` - 추정 엔트로피 비트 수
    fn add_entropy(&mut self, data: &[u8], estimated_bits: usize) {
        // SHA-256 스타일 해시를 사용하여 엔트로피 혼합
        // 간단한 구현: XOR과 회전
        for (i, &byte) in data.iter().enumerate() {
            let pos = (self.position + i) % ENTROPY_POOL_SIZE;
            self.pool[pos] ^= byte;
            // 회전 및 추가 혼합
            self.pool[pos] = self.pool[pos].wrapping_add(byte.wrapping_mul(0x9E));
            self.pool[pos] = self.pool[pos].rotate_left(3);
        }
        
        // 엔트로피 비트 수 누적 (최대 제한)
        self.entropy_bits = self.entropy_bits.saturating_add(estimated_bits).min(ENTROPY_POOL_SIZE * 8);
        self.position = (self.position + data.len()) % ENTROPY_POOL_SIZE;
    }
    
    /// 엔트로피에서 랜덤 값 추출
    /// 
    /// # Returns
    /// 엔트로피가 충분하면 Some(u64), 그렇지 않으면 None
    fn extract_random(&mut self) -> Option<u64> {
        if self.entropy_bits < MIN_ENTROPY_BITS {
            return None;
        }
        
        // 엔트로피 풀에서 8바이트 추출
        let mut result: u64 = 0;
        for i in 0..8 {
            let byte = self.pool[(self.position + i) % ENTROPY_POOL_SIZE];
            result |= (byte as u64) << (i * 8);
            
            // 사용한 엔트로피 제거 (XOR로 다시 혼합)
            self.pool[(self.position + i) % ENTROPY_POOL_SIZE] ^= byte;
        }
        
        // 위치 이동
        self.position = (self.position + 8) % ENTROPY_POOL_SIZE;
        
        // 엔트로피 비트 감소 (8바이트 = 64비트)
        self.entropy_bits = self.entropy_bits.saturating_sub(64);
        
        Some(result)
    }
    
    /// 엔트로피 풀 상태 확인
    fn has_enough_entropy(&self) -> bool {
        self.entropy_bits >= MIN_ENTROPY_BITS
    }
}

/// ASLR 상태
pub struct AslrState {
    /// 커널 랜덤 오프셋 (페이지 단위)
    kernel_offset: u64,
    /// 스택 랜덤 오프셋 (페이지 단위)
    stack_offset: u64,
    /// 힙 랜덤 오프셋 (페이지 단위)
    heap_offset: u64,
    /// 활성화 여부
    enabled: bool,
    /// 엔트로피 풀
    entropy_pool: EntropyPool,
}

impl AslrState {
    /// 새 ASLR 상태 생성
    fn new() -> Self {
        Self {
            kernel_offset: 0,
            stack_offset: 0,
            heap_offset: 0,
            enabled: false,
            entropy_pool: EntropyPool::new(),
        }
    }
    
    /// ASLR 초기화 및 랜덤 오프셋 생성
    ///
    /// # Safety
    /// 메모리 관리가 초기화되기 전에 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), AslrError> {
        if self.enabled {
            return Ok(()); // 이미 초기화됨
        }
        
        // 엔트로피 풀 채우기
        self.collect_entropy();
        
        // 엔트로피 풀에서 랜덤 오프셋 생성
        self.kernel_offset = self.generate_offset_from_pool(0x1000, 0x100000)
            .ok_or(AslrError::RandomGenerationFailed)?; // 4KB-4MB 범위
        self.stack_offset = self.generate_offset_from_pool(0x100, 0x10000)
            .ok_or(AslrError::RandomGenerationFailed)?;   // 256-64KB 범위
        self.heap_offset = self.generate_offset_from_pool(0x1000, 0x100000)
            .ok_or(AslrError::RandomGenerationFailed)?;   // 4KB-4MB 범위
        
        self.enabled = true;
        
        crate::log_info!("ASLR initialized with entropy pool: kernel_offset={:#x}, stack_offset={:#x}, heap_offset={:#x}",
                        self.kernel_offset, self.stack_offset, self.heap_offset);
        
        Ok(())
    }
    
    /// 여러 소스에서 엔트로피 수집
    /// 
    /// RDRAND, 타임스탬프 카운터, 인터럽트 타이밍 등을 사용합니다.
    unsafe fn collect_entropy(&mut self) {
        // 1. RDRAND로 엔트로피 수집 (여러 번 시도)
        let mut rdrand_count = 0;
        for _ in 0..32 {
            if let Some(value) = Self::try_rdrand() {
                let bytes = value.to_le_bytes();
                self.entropy_pool.add_entropy(&bytes, 64); // RDRAND는 높은 엔트로피
                rdrand_count += 1;
            }
        }
        
        if rdrand_count > 0 {
            crate::log_debug!("ASLR: Collected {} RDRAND samples", rdrand_count);
        }
        
        // 2. 타임스탬프 카운터로 엔트로피 추가 (낮은 엔트로피이지만 다양성 제공)
        for _ in 0..16 {
            let ts1 = core::arch::x86_64::_rdtsc();
            // 짧은 지연 후 다시 읽어 타이밍 변화 포착
            core::arch::asm!("pause", options(nostack, preserves_flags));
            let ts2 = core::arch::x86_64::_rdtsc();
            let diff = ts2.wrapping_sub(ts1);
            let bytes = diff.to_le_bytes();
            self.entropy_pool.add_entropy(&bytes, 8); // 낮은 엔트로피 (추정)
        }
        
        // 3. CPU ID 및 기타 하드웨어 정보
        let cp = unsafe { core::arch::x86_64::__cpuid(0) };
        let eax = cp.eax; let ebx = cp.ebx; let ecx = cp.ecx; let edx = cp.edx;
        
        let cpu_info = [
            eax.to_le_bytes(),
            ebx.to_le_bytes(),
            ecx.to_le_bytes(),
            edx.to_le_bytes(),
        ];
        for info in &cpu_info {
            self.entropy_pool.add_entropy(info, 4); // 하드웨어 정보는 낮은 엔트로피
        }
        
        crate::log_debug!("ASLR: Entropy pool status: {} bits", self.entropy_pool.entropy_bits);
    }
    
    /// 엔트로피 풀에서 오프셋 생성
    /// 
    /// # Arguments
    /// * `min` - 최소 오프셋 (페이지 단위)
    /// * `max` - 최대 오프셋 (페이지 단위)
    /// 
    /// # Returns
    /// 생성된 오프셋 (페이지 단위, 정렬됨)
    fn generate_offset_from_pool(&mut self, min: u64, max: u64) -> Option<u64> {
        // 엔트로피 풀에서 랜덤 값 추출
        let random = self.entropy_pool.extract_random()?;
        
        // 범위 내로 제한
        let range = max - min;
        let value = min + (random % range);
        
        // 페이지 단위로 정렬 (4KB)
        Some(value & !0xFFF)
    }
    
    /// RDRAND 시도 (Intel 하드웨어 랜덤 생성기)
    /// 
    /// 여러 번 재시도하여 성공률을 높입니다.
    fn try_rdrand() -> Option<u64> {
        unsafe {
            // 최대 10번 재시도
            for _ in 0..10 {
                let mut result: u64 = 0;
                let mut cf: u8;
                // RDRAND는 CF 플래그로 성공 여부 표시: setc로 CF를 변수에 저장
                core::arch::asm!(
                    "rdrand {res} ; setc {cfb}",
                    res = out(reg) result,
                    cfb = lateout(reg_byte) cf,
                    options(nostack, preserves_flags)
                );
                
                if cf != 0 {
                    return Some(result);
                }
                
                // 짧은 지연 후 재시도
                core::arch::asm!("pause", options(nostack, preserves_flags));
            }
            
            None // RDRAND 실패 또는 미지원
        }
    }
    
    /// RDRAND 지원 여부 확인 (CPUID)
    fn is_rdrand_supported() -> bool {
        unsafe {
            let r = core::arch::x86_64::__cpuid(1);
            (r.ecx & (1 << 30)) != 0
        }
    }
    
    /// 커널 랜덤 오프셋 가져오기
    pub fn kernel_offset(&self) -> u64 {
        self.kernel_offset
    }
    
    /// 스택 랜덤 오프셋 가져오기
    pub fn stack_offset(&self) -> u64 {
        self.stack_offset
    }
    
    /// 힙 랜덤 오프셋 가져오기
    pub fn heap_offset(&self) -> u64 {
        self.heap_offset
    }
    
    /// 활성화 여부 확인
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// 랜덤화된 스택 주소 생성
    pub fn randomize_stack_address(&self, base: VirtAddr) -> VirtAddr {
        if !self.enabled {
            return base;
        }
        
        let offset = self.stack_offset * 0x1000; // 페이지 크기로 변환
        VirtAddr::new(base.as_u64().wrapping_add(offset))
    }
    
    /// 랜덤화된 힙 주소 생성
    pub fn randomize_heap_address(&self, base: VirtAddr) -> VirtAddr {
        if !self.enabled {
            return base;
        }
        
        let offset = self.heap_offset * 0x1000; // 페이지 크기로 변환
        VirtAddr::new(base.as_u64().wrapping_add(offset))
    }
}

/// ASLR 에러
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AslrError {
    AlreadyInitialized,
    InvalidOffset,
    RandomGenerationFailed,
}

impl core::fmt::Display for AslrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AslrError::AlreadyInitialized => write!(f, "ASLR already initialized"),
            AslrError::InvalidOffset => write!(f, "Invalid ASLR offset"),
            AslrError::RandomGenerationFailed => write!(f, "ASLR random generation failed"),
        }
    }
}

/// 전역 ASLR 상태
static ASLR_STATE: Mutex<AslrState> = Mutex::new(AslrState {
    kernel_offset: 0,
    stack_offset: 0,
    heap_offset: 0,
    enabled: false,
    entropy_pool: EntropyPool {
        pool: [0u8; ENTROPY_POOL_SIZE],
        entropy_bits: 0,
        position: 0,
    },
});

/// ASLR 초기화
///
/// # Safety
/// 메모리 관리가 초기화되기 전에 호출되어야 합니다.
pub unsafe fn init_aslr() -> Result<(), AslrError> {
    let mut state = ASLR_STATE.lock();
    state.init()
}

/// ASLR 활성화 여부 확인
pub fn is_aslr_enabled() -> bool {
    let state = ASLR_STATE.lock();
    state.is_enabled()
}

/// 랜덤화된 스택 주소 생성
pub fn randomize_stack_address(base: VirtAddr) -> VirtAddr {
    let state = ASLR_STATE.lock();
    state.randomize_stack_address(base)
}

/// 랜덤화된 힙 주소 생성
pub fn randomize_heap_address(base: VirtAddr) -> VirtAddr {
    let state = ASLR_STATE.lock();
    state.randomize_heap_address(base)
}

