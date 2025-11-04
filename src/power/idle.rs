//! CPU Idle (C-State) management

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
    pub const fn new() -> Self {
        Self { available: [None, None, None, None, None, None, None, None], current: 0 }
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
}


