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
}

impl IdleStateManager {
    pub fn new() -> Self {
        // Snapshot current global defaults; if empty, we keep HLT fallback
        let snapshot = get_default_cstates();
        Self { available: snapshot, current: 0 }
    }

    #[inline]
    pub unsafe fn enter_c_state(&self, target_level: u8) {
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


