//! Minimal per-process syscall capability mask (global placeholder)
//! For now, a global bitmap is used until per-process structs are plumbed.

use crate::syscall::numbers::SyscallNumber;
use spin::Mutex;

// 256-bit bitmap (support up to 256 syscalls)
static CAPS: Mutex<[u64; 4]> = Mutex::new([!0u64; 4]); // allow all by default

pub fn set_syscall_cap_allow_all(allow: bool) {
    let mut m = CAPS.lock();
    let val = if allow { !0u64 } else { 0u64 };
    m[0] = val; m[1] = val; m[2] = val; m[3] = val;
}

pub fn deny_syscall(num: SyscallNumber) {
    let idx = (num as usize) / 64;
    let bit = (num as usize) % 64;
    let mut m = CAPS.lock();
    if idx < m.len() { m[idx] &= !(1u64 << bit); }
}

pub fn is_syscall_allowed(num: SyscallNumber) -> bool {
    let idx = (num as usize) / 64;
    let bit = (num as usize) % 64;
    let m = CAPS.lock();
    if idx >= m.len() { return false; }
    (m[idx] & (1u64 << bit)) != 0
}


