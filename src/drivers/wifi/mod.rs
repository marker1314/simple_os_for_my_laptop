//! Wi‑Fi drivers

pub mod rtl8821ce;

use spin::Mutex;

#[derive(Debug)]
pub enum WifiError { NotFound, InitFailed }

pub static WIFI_READY: Mutex<bool> = Mutex::new(false);

/// Initialize built‑in Wi‑Fi (detect supported chipset and bring minimal link up)
pub unsafe fn init() -> Result<(), WifiError> {
    if let Ok(()) = rtl8821ce::try_init() {
        *WIFI_READY.lock() = true;
        return Ok(());
    }
    Err(WifiError::NotFound)
}

/// Placeholder IRQ handler hook (would be registered via PCI/INTx/MSI in real impl)
pub fn handle_interrupt() {
    // For now, device-specific module should ack the IRQ
}

/// Periodic tick for Wi‑Fi polling
pub fn tick() {
    rtl8821ce::tick();
}


