//! Device-level power management

use crate::power::PowerError;
use spin::Mutex;

/// 장치 상태 저장 구조체
#[derive(Clone, Copy, Default)]
pub struct DeviceState {
    /// 인터럽트 마스크 상태 (PIC)
    pic_mask: u16,
    /// 네트워크 드라이버 상태
    network_active: bool,
    /// 디스플레이 상태
    display_blanked: bool,
    /// 입력 장치 인터럽트 상태
    input_interrupts_enabled: bool,
}

static SAVED_DEVICE_STATE: Mutex<Option<DeviceState>> = Mutex::new(None);

/// Disk power policy
pub struct DiskPowerPolicy {
    pub spin_down_ms: u64,
}

/// Apply disk power policy
pub fn apply_disk_power_policy(_policy: &DiskPowerPolicy) {
    // Issue ATA STANDBY command via drivers::ata
    #[cfg(feature = "fs")]
    {
        // ATA standby 명령을 통해 디스크를 대기 상태로 전환
        // 실제 구현은 drivers::ata에서 처리
        crate::log_info!("Disk power policy applied: spin_down={}ms", _policy.spin_down_ms);
    }
}

/// Power off disk (suspend 준비)
pub fn power_off_disk() -> Result<(), PowerError> {
    #[cfg(feature = "fs")]
    {
        // ATA STANDBY 명령 전송
        crate::log_info!("Disk power off: entering standby");
        // 실제 구현: drivers::ata::standby()
    }
    Ok(())
}

/// Power on disk (resume)
pub fn power_on_disk() -> Result<(), PowerError> {
    #[cfg(feature = "fs")]
    {
        // 디스크 활성화 (IDLE 명령으로 깨우기)
        crate::log_info!("Disk power on: waking from standby");
        // 실제 구현: drivers::ata::wake()
    }
    Ok(())
}

/// Enable network low power mode
pub fn enable_network_low_power() {
    #[cfg(feature = "net")]
    {
        // RTL8139 low-power 모드 활성화
        crate::log_info!("Network low-power mode enabled");
        // 실제 구현: drivers::rtl8139::set_low_power()
    }
}

/// Power off network (suspend 준비)
pub fn power_off_network() -> Result<(), PowerError> {
    #[cfg(feature = "net")]
    {
        // 네트워크 인터럽트 비활성화 및 low-power 모드
        enable_network_low_power();
        crate::log_info!("Network power off");
    }
    Ok(())
}

/// Power on network (resume)
pub fn power_on_network() -> Result<(), PowerError> {
    #[cfg(feature = "net")]
    {
        // 네트워크 인터럽트 재활성화 및 normal 모드
        crate::log_info!("Network power on");
        // 실제 구현: drivers::rtl8139::wake()
    }
    Ok(())
}

/// Power off input devices (suspend 준비)
pub fn power_off_input() -> Result<(), PowerError> {
    // 입력 장치 인터럽트 비활성화
    unsafe {
        crate::interrupts::pic::set_mask(1, false);  // 키보드
        crate::interrupts::pic::set_mask(12, false);  // 마우스
    }
    crate::log_info!("Input devices power off");
    Ok(())
}

/// Power on input devices (resume)
pub fn power_on_input() -> Result<(), PowerError> {
    // 입력 장치 인터럽트 재활성화
    unsafe {
        crate::interrupts::pic::set_mask(1, true);   // 키보드
        crate::interrupts::pic::set_mask(12, true);   // 마우스
    }
    crate::log_info!("Input devices power on");
    Ok(())
}

/// DPMS: Display Power Management Signaling
pub fn dpms_set_display_sleep(sleep: bool) {
    if sleep {
        crate::drivers::framebuffer::blank();
        crate::log_info!("Display sleep: blanked");
    } else {
        crate::drivers::framebuffer::unblank();
        crate::log_info!("Display sleep: unblanked");
    }
}

/// Backlight 밝기 제어 (0-100%)
/// 
/// # Arguments
/// * `brightness_percent` - 백라이트 밝기 (0 = off, 100 = max)
pub fn set_backlight_brightness(brightness_percent: u8) -> Result<(), PowerError> {
    let brightness = brightness_percent.min(100);
    
    // 간단한 구현: 밝기를 프레임버퍼 색상 밝기로 시뮬레이션
    // 실제 하드웨어에서는 ACPI _BCM/_BCL을 사용해야 함
    if brightness == 0 {
        dpms_set_display_sleep(true);
    } else {
        dpms_set_display_sleep(false);
        // TODO: 실제 백라이트 제어 (ACPI 또는 다른 인터페이스)
        crate::log_info!("Backlight brightness: {}%", brightness);
    }
    
    Ok(())
}

/// Backlight 밝기 가져오기
pub fn get_backlight_brightness() -> u8 {
    if crate::drivers::framebuffer::is_blank() {
        0
    } else {
        100 // 기본값
    }
}

/// Power off display (suspend 준비)
pub fn power_off_display() -> Result<(), PowerError> {
    dpms_set_display_sleep(true);
    Ok(())
}

/// Power on display (resume)
pub fn power_on_display() -> Result<(), PowerError> {
    dpms_set_display_sleep(false);
    Ok(())
}

/// Save device state before suspend
pub fn save_device_state() -> Result<(), PowerError> {
    let mut state = DeviceState::default();
    
    // PIC 마스크 상태 저장
    crate::interrupts::pic::save_interrupt_mask();
    
    // 네트워크 상태 저장
    #[cfg(feature = "net")]
    {
        state.network_active = true; // TODO: 실제 상태 읽기
    }
    
    // 디스플레이 상태 저장
    state.display_blanked = crate::drivers::framebuffer::is_blank();
    
    // 입력 장치 상태 저장
    state.input_interrupts_enabled = true; // TODO: 실제 상태 읽기
    
    *SAVED_DEVICE_STATE.lock() = Some(state);
    crate::log_info!("Device state saved");
    Ok(())
}

/// Restore device state after resume
pub fn restore_device_state() -> Result<(), PowerError> {
    let state = SAVED_DEVICE_STATE.lock().take();
    
    // PIC 마스크 상태 복원
    crate::interrupts::pic::restore_interrupt_mask();
    
    if let Some(s) = state {
        // 디스플레이 상태 복원
        if s.display_blanked {
            dpms_set_display_sleep(true);
        } else {
            dpms_set_display_sleep(false);
        }
        
        crate::log_info!("Device state restored");
        Ok(())
    } else {
        crate::log_warn!("No saved device state to restore");
        Ok(())
    }
}

/// Quiesce all devices before suspend
pub fn quiesce_all_devices() -> Result<(), PowerError> {
    crate::log_info!("Quiescing all devices...");
    
    // 장치 상태 저장
    save_device_state()?;
    
    // 디스플레이 끄기
    power_off_display()?;
    
    // 네트워크 low-power 모드
    power_off_network()?;
    
    // 디스크 standby
    power_off_disk()?;
    
    // 입력 장치 인터럽트 비활성화
    power_off_input()?;
    
    crate::log_info!("All devices quiesced");
    Ok(())
}

/// Resume all devices after wake
pub fn resume_all_devices() -> Result<(), PowerError> {
    crate::log_info!("Resuming all devices...");
    
    // 입력 장치 재활성화
    power_on_input()?;
    
    // 디스크 활성화
    power_on_disk()?;
    
    // 네트워크 활성화
    power_on_network()?;
    
    // 디스플레이 켜기
    power_on_display()?;
    
    // 장치 상태 복원
    restore_device_state()?;
    
    crate::log_info!("All devices resumed");
    Ok(())
}


