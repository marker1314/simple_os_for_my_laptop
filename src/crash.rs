//! Minimal crash capture persisted in .noinit (best-effort)

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CrashDump {
    pub magic: u32,
    pub reason: u32,      // 1=panic, 2=exception
    pub rip: u64,
    pub code: u64,
}

#[link_section = ".noinit"]
static mut LAST_CRASH: CrashDump = CrashDump { magic: 0, reason: 0, rip: 0, code: 0 };

pub fn record_panic() {
    unsafe { LAST_CRASH = CrashDump { magic: 0x43525348, reason: 1, rip: 0, code: 0 }; }
}

pub fn record_exception(rip: u64, code: u64) {
    unsafe { LAST_CRASH = CrashDump { magic: 0x43525348, reason: 2, rip, code }; }
}

pub fn take() -> Option<CrashDump> {
    unsafe {
        if LAST_CRASH.magic == 0x43525348 {
            let dump = LAST_CRASH;
            LAST_CRASH.magic = 0; // clear after read
            Some(dump)
        } else {
            None
        }
    }
}


