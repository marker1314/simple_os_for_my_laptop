//! 부팅 타임라인 추적
//!
//! 커널 초기화 단계별 시간을 추적하여 부팅 성능을 분석합니다.

use spin::Mutex;
use alloc::vec::Vec;

/// 부팅 단계
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    /// 부트 시작
    BootStart,
    /// 시리얼 포트 초기화
    SerialInit,
    /// 부트 정보 저장
    BootInfoInit,
    /// PIC 리매핑
    PicRemap,
    /// IDT 초기화
    IdtInit,
    /// 인터럽트 활성화
    InterruptsEnabled,
    /// 메모리 관리 초기화
    MemoryInit,
    /// 타이머 초기화
    TimerInit,
    /// 키보드 드라이버 초기화
    KeyboardInit,
    /// VGA 초기화
    VgaInit,
    /// 스케줄러 초기화
    SchedulerInit,
    /// 시스템 콜 핸들러 초기화
    SyscallInit,
    /// ATA 드라이버 초기화
    AtaInit,
    /// 파일시스템 준비
    FilesystemReady,
    /// 전력 관리 초기화
    PowerInit,
    /// 네트워크 드라이버 초기화
    NetworkInit,
    /// 프레임버퍼 초기화
    FramebufferInit,
    /// GUI 시스템 초기화
    GuiInit,
    /// 마우스 드라이버 초기화
    MouseInit,
    /// I2C/터치패드 초기화
    TouchpadInit,
    /// 커널 초기화 완료
    KernelInitComplete,
    /// 데스크톱/GUI 시작
    DesktopStart,
    /// Shell 시작
    ShellStart,
}

/// 부팅 타임라인 엔트리
#[derive(Debug, Clone, Copy)]
struct TimelineEntry {
    stage: BootStage,
    timestamp_ms: u64,
    relative_ms: u64, // 부트 시작부터의 상대 시간
}

static TIMELINE: Mutex<Vec<TimelineEntry>> = Mutex::new(Vec::new());
static BOOT_START_MS: Mutex<Option<u64>> = Mutex::new(None);

/// 부팅 시작 기록
pub fn mark_boot_start() {
    // 타이머가 아직 초기화되지 않았을 수 있으므로, 0으로 시작
    let mut start = BOOT_START_MS.lock();
    if start.is_none() {
        *start = Some(0);
        let mut timeline = TIMELINE.lock();
        timeline.push(TimelineEntry {
            stage: BootStage::BootStart,
            timestamp_ms: 0,
            relative_ms: 0,
        });
    }
}

/// 타임라인 엔트리 기록
pub fn mark_stage(stage: BootStage) {
    // 타이머가 초기화된 후에만 정확한 타임스탬프 사용
    let timestamp_ms = crate::drivers::timer::get_milliseconds();
    let mut start = BOOT_START_MS.lock();
    let relative_ms = if let Some(start_ms) = *start {
        timestamp_ms.saturating_sub(start_ms)
    } else {
        // 아직 시작이 기록되지 않았으면 지금 기록
        *start = Some(timestamp_ms);
        timestamp_ms
    };
    
    let mut timeline = TIMELINE.lock();
    timeline.push(TimelineEntry {
        stage,
        timestamp_ms,
        relative_ms,
    });
}

/// 타임라인 리포트 출력
pub fn print_timeline() {
    let timeline = TIMELINE.lock();
    crate::log_info!("=== Boot Timeline ===");
    for entry in timeline.iter() {
        let stage_name = match entry.stage {
            BootStage::BootStart => "Boot Start",
            BootStage::SerialInit => "Serial Init",
            BootStage::BootInfoInit => "Boot Info Init",
            BootStage::PicRemap => "PIC Remap",
            BootStage::IdtInit => "IDT Init",
            BootStage::InterruptsEnabled => "Interrupts Enabled",
            BootStage::MemoryInit => "Memory Init",
            BootStage::TimerInit => "Timer Init",
            BootStage::KeyboardInit => "Keyboard Init",
            BootStage::VgaInit => "VGA Init",
            BootStage::SchedulerInit => "Scheduler Init",
            BootStage::SyscallInit => "Syscall Init",
            BootStage::AtaInit => "ATA Init",
            BootStage::FilesystemReady => "Filesystem Ready",
            BootStage::PowerInit => "Power Init",
            BootStage::NetworkInit => "Network Init",
            BootStage::FramebufferInit => "Framebuffer Init",
            BootStage::GuiInit => "GUI Init",
            BootStage::MouseInit => "Mouse Init",
            BootStage::TouchpadInit => "Touchpad Init",
            BootStage::KernelInitComplete => "Kernel Init Complete",
            BootStage::DesktopStart => "Desktop Start",
            BootStage::ShellStart => "Shell Start",
        };
        crate::log_info!("{}: {}ms (absolute: {}ms)", stage_name, entry.relative_ms, entry.timestamp_ms);
    }
    crate::log_info!("===================");
}

/// 타임라인을 CSV 형식으로 내보내기
pub fn export_timeline_csv() {
    let timeline = TIMELINE.lock();
    crate::serial_println!("stage,timestamp_ms,relative_ms");
    for entry in timeline.iter() {
        let stage_name = match entry.stage {
            BootStage::BootStart => "BootStart",
            BootStage::SerialInit => "SerialInit",
            BootStage::BootInfoInit => "BootInfoInit",
            BootStage::PicRemap => "PicRemap",
            BootStage::IdtInit => "IdtInit",
            BootStage::InterruptsEnabled => "InterruptsEnabled",
            BootStage::MemoryInit => "MemoryInit",
            BootStage::TimerInit => "TimerInit",
            BootStage::KeyboardInit => "KeyboardInit",
            BootStage::VgaInit => "VgaInit",
            BootStage::SchedulerInit => "SchedulerInit",
            BootStage::SyscallInit => "SyscallInit",
            BootStage::AtaInit => "AtaInit",
            BootStage::FilesystemReady => "FilesystemReady",
            BootStage::PowerInit => "PowerInit",
            BootStage::NetworkInit => "NetworkInit",
            BootStage::FramebufferInit => "FramebufferInit",
            BootStage::GuiInit => "GuiInit",
            BootStage::MouseInit => "MouseInit",
            BootStage::TouchpadInit => "TouchpadInit",
            BootStage::KernelInitComplete => "KernelInitComplete",
            BootStage::DesktopStart => "DesktopStart",
            BootStage::ShellStart => "ShellStart",
        };
        crate::serial_println!("{},{},{}", stage_name, entry.timestamp_ms, entry.relative_ms);
    }
}

/// 총 부팅 시간 가져오기
pub fn get_total_boot_time_ms() -> Option<u64> {
    let timeline = TIMELINE.lock();
    timeline.last().map(|entry| entry.relative_ms)
}


