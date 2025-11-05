//! Boot/runtime profile selection

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Balanced,
    PowerSaver,
    Performance,
    Headless,
}

use spin::Mutex;

// 기본 전력 모드를 저전력으로 설정
static CURRENT_PROFILE: Mutex<Profile> = Mutex::new(Profile::PowerSaver);

#[inline]
pub fn current_profile() -> Profile {
    *CURRENT_PROFILE.lock()
}

#[inline]
pub fn set_current_profile(p: Profile) {
    *CURRENT_PROFILE.lock() = p;
}


