//! Stack Canary 구현
//!
//! 스택 버퍼 오버플로우를 감지하기 위한 스택 카나리 기능입니다.
//!
//! # 작동 원리
//!
//! 1. 스레드 생성 시 랜덤 카나리 값 생성
//! 2. 스택 프레임 시작 부분에 카나리 값 저장
//! 3. 함수 종료 시 카나리 값 검증
//! 4. 카나리 값이 변경되면 스택 오버플로우로 간주하고 크래시

use spin::Mutex;
use x86_64::VirtAddr;

/// 스택 카나리 값 (8바이트)
/// 
/// NULL 바이트를 포함하여 문자열 오버플로우도 감지할 수 있도록 설계
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackCanary {
    /// 카나리 값 (8바이트)
    value: [u8; 8],
}

impl StackCanary {
    /// 새 카나리 값 생성
    /// 
    /// 엔트로피 풀에서 랜덤 값을 생성합니다.
    pub fn generate() -> Self {
        // 엔트로피 소스에서 랜덤 값 생성
        let mut canary = [0u8; 8];
        
        unsafe {
            // RDRAND 또는 타임스탬프 카운터 사용
            let mut seed = Self::try_rdrand().unwrap_or_else(|| {
                core::arch::x86_64::_rdtsc()
            });
            
            // 8바이트 생성
            for i in 0..8 {
                // 각 바이트마다 엔트로피 추가
                let ts = core::arch::x86_64::_rdtsc();
                seed ^= ts.wrapping_mul(0x9E3779B97F4A7C15);
                seed = seed.wrapping_mul(0x9E3779B97F4A7C15);
                seed ^= seed >> 32;
                
                canary[i] = (seed >> (i * 8)) as u8;
            }
            
            // NULL 바이트를 포함하여 문자열 오버플로우도 감지
            // 첫 번째 바이트는 0x00으로 설정 (문자열 종료자)
            canary[0] = 0x00;
            
            // 마지막 바이트는 0x0A로 설정 (줄바꿈 문자, 추가 보호)
            canary[7] = 0x0A;
        }
        
        Self { value: canary }
    }
    
    /// RDRAND 시도
    fn try_rdrand() -> Option<u64> {
        unsafe {
            for _ in 0..10 {
                let mut result: u64 = 0;
                let mut carry: u8 = 0;
                
                core::arch::asm!(
                    "rdrand {}",
                    out(reg) result,
                    out("cf") carry,
                    options(nostack, preserves_flags)
                );
                
                if carry != 0 {
                    return Some(result);
                }
                
                core::arch::asm!("pause", options(nostack, preserves_flags));
            }
            None
        }
    }
    
    /// 카나리 값을 u64로 변환 (스택에 저장용)
    pub fn as_u64(&self) -> u64 {
        let mut result = 0u64;
        for (i, &byte) in self.value.iter().enumerate() {
            result |= (byte as u64) << (i * 8);
        }
        result
    }
    
    /// u64에서 카나리 값 생성
    pub fn from_u64(value: u64) -> Self {
        let mut canary = [0u8; 8];
        for i in 0..8 {
            canary[i] = ((value >> (i * 8)) & 0xFF) as u8;
        }
        Self { value: canary }
    }
    
    /// 카나리 값 비교
    pub fn verify(&self, other: &Self) -> bool {
        self.value == other.value
    }
    
    /// 카나리 값이 손상되었는지 확인
    pub fn is_corrupted(&self) -> bool {
        // NULL 바이트가 변경되었는지 확인
        if self.value[0] != 0x00 {
            return true;
        }
        
        // 마지막 바이트 확인
        if self.value[7] != 0x0A {
            return true;
        }
        
        false
    }
}

/// 스택 카나리 관리자
/// 
/// 각 스레드의 카나리 값을 관리합니다.
pub struct StackCanaryManager {
    /// 활성화 여부
    enabled: bool,
}

impl StackCanaryManager {
    /// 새 카나리 관리자 생성
    pub fn new() -> Self {
        Self {
            enabled: true,
        }
    }
    
    /// 카나리 활성화 여부 확인
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// 카나리 활성화/비활성화
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// 전역 카나리 관리자
static CANARY_MANAGER: Mutex<StackCanaryManager> = Mutex::new(StackCanaryManager {
    enabled: true,
});

/// 스택에 카나리 값 저장
/// 
/// # Arguments
/// * `stack_pointer` - 스택 포인터 (카나리를 저장할 위치)
/// 
/// # Safety
/// `stack_pointer`는 유효한 스택 주소여야 합니다.
pub unsafe fn set_stack_canary(stack_pointer: VirtAddr) -> Option<StackCanary> {
    let manager = CANARY_MANAGER.lock();
    if !manager.is_enabled() {
        return None;
    }
    drop(manager);
    
    // 카나리 값 생성
    let canary = StackCanary::generate();
    let canary_value = canary.as_u64();
    
    // 스택 포인터 아래에 카나리 저장 (스택은 높은 주소에서 낮은 주소로 자람)
    let canary_addr = stack_pointer - 8; // 8바이트 카나리
    let canary_ptr = canary_addr.as_mut_ptr::<u64>();
    
    // 카나리 값 쓰기
    core::ptr::write(canary_ptr, canary_value);
    
    Some(canary)
}

/// 스택 카나리 검증
/// 
/// 함수 종료 시 호출하여 카나리 값이 변경되지 않았는지 확인합니다.
/// 
/// # Arguments
/// * `stack_pointer` - 스택 포인터 (카나리가 저장된 위치)
/// * `original_canary` - 원본 카나리 값
/// 
/// # Safety
/// `stack_pointer`는 유효한 스택 주소여야 합니다.
/// 
/// # Returns
/// 카나리가 유효하면 Ok(()), 손상되었으면 Err
pub unsafe fn verify_stack_canary(stack_pointer: VirtAddr, original_canary: &StackCanary) -> Result<(), &'static str> {
    let manager = CANARY_MANAGER.lock();
    if !manager.is_enabled() {
        return Ok(()); // 비활성화된 경우 검증 스킵
    }
    drop(manager);
    
    // 카나리 위치에서 값 읽기
    let canary_addr = stack_pointer - 8;
    let canary_ptr = canary_addr.as_ptr::<u64>();
    let current_value = core::ptr::read(canary_ptr);
    let current_canary = StackCanary::from_u64(current_value);
    
    // 카나리 검증
    if !original_canary.verify(&current_canary) {
        crate::log_error!("Stack canary corruption detected! Original: {:?}, Current: {:?}",
                         original_canary, current_canary);
        return Err("Stack buffer overflow detected");
    }
    
    // 추가 검증: 카나리 손상 확인
    if current_canary.is_corrupted() {
        crate::log_error!("Stack canary structure corruption detected!");
        return Err("Stack canary structure corrupted");
    }
    
    Ok(())
}

/// 스택 카나리 활성화 여부 확인
pub fn is_canary_enabled() -> bool {
    let manager = CANARY_MANAGER.lock();
    manager.is_enabled()
}

/// 스택 카나리 활성화/비활성화
pub fn set_canary_enabled(enabled: bool) {
    let mut manager = CANARY_MANAGER.lock();
    manager.set_enabled(enabled);
}

