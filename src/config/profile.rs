//! Boot/runtime profile selection

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Balanced,
    PowerSaver,
    Performance,
    Headless,
}

#[inline]
pub const fn current_profile() -> Profile {
    // Order of checks ensures exactly one wins based on enabled feature
    #[cfg(feature = "power_saver")]
    { return Profile::PowerSaver; }
    #[cfg(feature = "performance")]
    { return Profile::Performance; }
    #[cfg(feature = "headless")]
    { return Profile::Headless; }
    // default
    Profile::Balanced
}


