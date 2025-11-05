//! Device power hooks registry (suspend/resume)

use alloc::vec::Vec;
use spin::Mutex;

type Hook = fn() -> Result<(), ()>;

static SUSPEND_HOOKS: Mutex<Vec<Hook>> = Mutex::new(Vec::new());
static RESUME_HOOKS: Mutex<Vec<Hook>> = Mutex::new(Vec::new());

pub fn register_suspend(h: Hook) { SUSPEND_HOOKS.lock().push(h); }
pub fn register_resume(h: Hook) { RESUME_HOOKS.lock().push(h); }

pub fn run_suspend_hooks() -> Result<(), ()> {
    for h in SUSPEND_HOOKS.lock().iter().copied() { h()?; }
    Ok(())
}

pub fn run_resume_hooks() -> Result<(), ()> {
    for h in RESUME_HOOKS.lock().iter().copied() { h()?; }
    Ok(())
}


