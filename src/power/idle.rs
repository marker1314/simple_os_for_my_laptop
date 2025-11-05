//! CPU Idle (C-State) management

use spin::Mutex;

/// Represents a CPU C-State
pub struct CState {
    pub level: u8,        // e.g., 0,1,2,3,6
    pub latency_us: u32,  // estimated entry/exit latency
    pub power_mw: u32,    // estimated power
    pub mwait_hint: u32,  // MWAIT hint (eax)
}

/// Manager for entering idle states
pub struct IdleStateManager {
    pub available: [Option<CState>; 8],
    pub current: u8,
    last_entry_time_ms: u64,
    /// 지연 시간 기반 동적 조정: 예상 유휴 시간 (ms)
    expected_idle_duration_ms: u64,
    /// 마지막 wakeup 시간
    last_wakeup_time_ms: u64,
}

static CURRENT_C_STATE: Mutex<u8> = Mutex::new(0);
static C_STATE_ENTRY_TIME: Mutex<u64> = Mutex::new(0);

impl IdleStateManager {
    pub fn new() -> Self {
        // Snapshot current global defaults; if empty, we keep HLT fallback
        let snapshot = get_default_cstates();
        let now_ms = crate::drivers::timer::get_milliseconds();
        Self { 
            available: snapshot, 
            current: 0, 
            last_entry_time_ms: 0,
            expected_idle_duration_ms: 0,
            last_wakeup_time_ms: now_ms,
        }
    }
    
    /// Get recommended C-state based on policy
    pub fn get_recommended_c_state(&self, policy_threshold: u8, cpu_usage: u8) -> u8 {
        if cpu_usage <= policy_threshold {
            // Find the deepest available C-state
            let mut deepest = 0u8;
            for state in self.available.iter().flatten() {
                if state.level > deepest {
                    deepest = state.level;
                }
            }
            deepest
        } else {
            0 // C0 - no idle
        }
    }
    
    /// 지연 시간 기반 동적 C-State 조정
    /// 
    /// 예상 유휴 시간에 따라 최적의 C-State를 선택합니다.
    /// 깊은 C-State는 더 많은 전력을 절약하지만, 깨어나는 데 더 오래 걸립니다.
    /// 
    /// # Arguments
    /// * `expected_idle_ms` - 예상 유휴 시간 (밀리초)
    /// 
    /// # Returns
    /// 권장 C-State 레벨
    pub fn get_c_state_by_latency(&self, expected_idle_ms: u64) -> u8 {
        let mut best_state = 0u8; // C0 기본값
        
        // 사용 가능한 C-State 중에서 예상 유휴 시간보다 exit latency가 작은 가장 깊은 상태 선택
        for state_opt in self.available.iter().flatten() {
            let exit_latency_ms = (state_opt.latency_us as u64) / 1000;
            
            // 예상 유휴 시간이 exit latency보다 충분히 크면 (최소 2배) 해당 상태 사용
            if expected_idle_ms >= exit_latency_ms * 2 && state_opt.level > best_state {
                best_state = state_opt.level;
            }
        }
        
        best_state
    }
    
    /// Wakeup 이벤트 기록 (동적 조정을 위해)
    pub fn record_wakeup(&mut self) {
        let now_ms = crate::drivers::timer::get_milliseconds();
        
        // 실제 유휴 시간 계산
        if self.last_entry_time_ms > 0 {
            let actual_idle_ms = now_ms.saturating_sub(self.last_entry_time_ms);
            
            // 예상 유휴 시간 업데이트 (지수 이동 평균)
            // alpha = 0.7 (가중치)
            self.expected_idle_duration_ms = 
                (self.expected_idle_duration_ms * 3 + actual_idle_ms * 7) / 10;
        }
        
        self.last_wakeup_time_ms = now_ms;
        self.last_entry_time_ms = 0;
    }
    
    /// 지연 시간 기반 최적 C-State로 진입
    pub unsafe fn enter_optimal_c_state(&mut self) {
        let now_ms = crate::drivers::timer::get_milliseconds();
        
        // 예상 유휴 시간 계산 (마지막 wakeup 이후 경과 시간 기반)
        let time_since_wakeup = now_ms.saturating_sub(self.last_wakeup_time_ms);
        let expected_idle = if time_since_wakeup < 100 {
            // 방금 wakeup이면 짧은 유휴 예상
            10 // 10ms
        } else {
            // 점진적으로 증가
            self.expected_idle_duration_ms.max(time_since_wakeup)
        };
        
        // 최적 C-State 선택
        let optimal_level = self.get_c_state_by_latency(expected_idle);
        
        // C-State 진입
        self.enter_c_state(optimal_level);
    }

    #[inline]
    pub unsafe fn enter_c_state(&mut self, target_level: u8) {
        // Record C-state entry
        let now_ms = crate::drivers::timer::get_milliseconds();
        let mut current = CURRENT_C_STATE.lock();
        let mut entry_time = C_STATE_ENTRY_TIME.lock();
        
        // Update residency tracking if state changed
        if *current != target_level {
            *current = target_level;
            *entry_time = now_ms;
        }
        
        // Entry time 기록 (동적 조정용)
        self.last_entry_time_ms = now_ms;
        
        // Try to use MWAIT if a hint exists; otherwise fallback to HLT
        if let Some(state) = self.available.iter().flatten().find(|s| s.level == target_level) {
            // MONITOR/MWAIT pair - best-effort
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            {
                // Hint in EAX, extensions in ECX (0)
                core::arch::asm!(
                    "mfence; mov eax, {hint:e}; xor ecx, ecx; mwait",
                    hint = in(reg) state.mwait_hint,
                    options(nostack, preserves_flags)
                );
                return;
            }
        }
        x86_64::instructions::hlt();
    }

    #[inline]
    pub unsafe fn enter_deepest(&self) {
        // pick highest available level; fallback to HLT
        let mut best: Option<&CState> = None;
        for s in self.available.iter().flatten() {
            match best {
                Some(b) if s.level <= b.level => {}
                _ => { best = Some(s); }
            }
        }
        
        let target_level = best.map(|s| s.level).unwrap_or(0);
        
        // Record C-state entry
        let now_ms = crate::drivers::timer::get_milliseconds();
        let mut current = CURRENT_C_STATE.lock();
        let mut entry_time = C_STATE_ENTRY_TIME.lock();
        
        if *current != target_level {
            *current = target_level;
            *entry_time = now_ms;
        }
        
        if let Some(state) = best {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            {
                core::arch::asm!(
                    "mfence; mov eax, {hint:e}; xor ecx, ecx; mwait",
                    hint = in(reg) state.mwait_hint,
                    options(nostack, preserves_flags)
                );
                return;
            }
        }
        x86_64::instructions::hlt();
    }
}

/// Get current C-state
pub fn get_current_c_state() -> u8 {
    *CURRENT_C_STATE.lock()
}

/// Get C-state entry time
pub fn get_c_state_entry_time_ms() -> u64 {
    *C_STATE_ENTRY_TIME.lock()
}

// Global default C-state table populated by ACPI (or fallback)
static DEFAULT_CSTATES: Mutex<[Option<CState>; 8]> = Mutex::new([None, None, None, None, None, None, None, None]);

/// Update the global default C-states (called from power manager after ACPI init)
pub fn set_default_cstates(cstates: [Option<CState>; 8]) {
    let mut table = DEFAULT_CSTATES.lock();
    *table = cstates;
}

/// Helper to read current default cstates (used by external callers if needed)
pub fn get_default_cstates() -> [Option<CState>; 8] {
    *DEFAULT_CSTATES.lock()
}


