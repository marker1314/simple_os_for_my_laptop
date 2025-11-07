//! IEEE 802.11 core (skeleton)

pub mod crypto;
pub mod ccmp;

#[derive(Debug, Clone)]
pub struct Ssid(pub alloc::string::String);

#[derive(Debug)]
pub enum WifiSec { Open, Wpa2Psk(alloc::string::String) }

#[derive(Debug)]
pub enum Ieee80211Error { NotImplemented }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssocState { Idle, Scanning, Associating, Associated }

static mut STATE: AssocState = AssocState::Idle;

pub fn start_scan() -> Result<(), Ieee80211Error> {
    unsafe { STATE = AssocState::Scanning; }
    crate::log_info!("Wi‑Fi: scanning...");
    // Send a probe request (placeholder)
    send_probe_request();
    Ok(())
}
pub fn connect(_ssid: &Ssid, _sec: &WifiSec) -> Result<(), Ieee80211Error> {
    // MVP: Validate inputs only
    if _ssid.0.is_empty() { return Err(Ieee80211Error::NotImplemented); }
    // If WPA2, compute PSK (placeholder)
    if let WifiSec::Wpa2Psk(ref pass) = _sec {
        let _ = crypto::derive_psk(pass, &_ssid.0).map_err(|_| Ieee80211Error::NotImplemented)?;
    }
    unsafe { STATE = AssocState::Associating; }
    crate::log_info!("Wi‑Fi: associating with {}", _ssid.0);
    // Send auth/assoc frames (placeholders)
    send_auth_request();
    send_assoc_request();
    // In real impl, send auth/assoc mgmt frames; on success:
    unsafe { STATE = AssocState::Associated; }
    crate::log_info!("Wi‑Fi: associated");
    // After association, attempt DHCP bring-up
    crate::net::driver::bringup_ipv4_via_dhcp();
    Ok(())
}

pub fn state() -> AssocState { unsafe { STATE } }

fn send_probe_request() {
    // Minimal 802.11 mgmt header placeholder (type/subtype fields zeroed)
    let buf: [u8; 24] = [0; 24];
    let _ = crate::drivers::wifi::rtl8821ce::tx_mgmt_frame(&buf);
}

fn send_auth_request() {
    let buf: [u8; 24] = [0; 24];
    let _ = crate::drivers::wifi::rtl8821ce::tx_mgmt_frame(&buf);
}

fn send_assoc_request() {
    let buf: [u8; 24] = [0; 24];
    let _ = crate::drivers::wifi::rtl8821ce::tx_mgmt_frame(&buf);
}


