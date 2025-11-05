//! 전력 정책 관리
//!
//! 전력 관리 정책 및 모드를 정의합니다.

/// 전력 모드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerMode {
    /// 성능 우선 모드
    /// CPU 클럭을 최대로 유지하여 최고 성능 제공
    Performance,
    /// 균형 모드
    /// 성능과 전력 소비의 균형을 맞춤
    Balanced,
    /// 전력 절약 모드
    /// CPU 클럭을 낮춰 전력 소비를 최소화
    PowerSaving,
}

impl PowerMode {
    /// 모드 이름을 문자열로 반환
    pub fn as_str(&self) -> &'static str {
        match self {
            PowerMode::Performance => "Performance",
            PowerMode::Balanced => "Balanced",
            PowerMode::PowerSaving => "Power Saving",
        }
    }
}

/// 전력 정책
///
/// 시스템의 전력 관리 정책을 관리합니다.
pub struct PowerPolicy {
    /// 현재 전력 모드
    mode: PowerMode,
    /// C-state 진입 임계값 (CPU 사용률이 이 값 이하면 깊은 C-state 진입)
    idle_threshold_percent: u8,
    /// 최소 C-state 레벨 (이 레벨 이상으로 진입)
    min_c_state_level: u8,
}

impl PowerPolicy {
    /// 새 전력 정책 생성
    pub fn new(mode: PowerMode) -> Self {
        let (idle_threshold, min_c_state) = match mode {
            PowerMode::Performance => (10, 1),  // 낮은 idle threshold, 얕은 C-state
            PowerMode::Balanced => (20, 2),      // 중간 threshold, 중간 C-state
            PowerMode::PowerSaving => (30, 3),  // 높은 threshold, 깊은 C-state
        };
        
        Self {
            mode,
            idle_threshold_percent: idle_threshold,
            min_c_state_level: min_c_state,
        }
    }
    
    /// 전력 모드 설정
    pub fn set_mode(&mut self, mode: PowerMode) {
        self.mode = mode;
        let (idle_threshold, min_c_state) = match mode {
            PowerMode::Performance => (10, 1),
            PowerMode::Balanced => (20, 2),
            PowerMode::PowerSaving => (30, 3),
        };
        self.idle_threshold_percent = idle_threshold;
        self.min_c_state_level = min_c_state;
    }
    
    /// 현재 전력 모드 가져오기
    pub fn get_mode(&self) -> PowerMode {
        self.mode
    }
    
    /// Idle threshold 가져오기
    pub fn get_idle_threshold(&self) -> u8 {
        self.idle_threshold_percent
    }
    
    /// 최소 C-state 레벨 가져오기
    pub fn get_min_c_state_level(&self) -> u8 {
        self.min_c_state_level
    }
    
    /// CPU 사용률에 따라 권장 C-state 레벨 계산
    pub fn recommended_c_state(&self, cpu_usage_percent: u8) -> u8 {
        if cpu_usage_percent <= self.idle_threshold_percent {
            // Idle 상태 - 깊은 C-state 권장
            self.min_c_state_level
        } else {
            // 활성 상태 - C0 (no idle)
            0
        }
    }
    
    /// Backlight 밝기 정책 가져오기
    pub fn get_backlight_brightness(&self) -> u8 {
        match self.mode {
            PowerMode::Performance => 100,  // 최대 밝기
            PowerMode::Balanced => 70,      // 중간 밝기
            PowerMode::PowerSaving => 40,   // 낮은 밝기
        }
    }
    
    /// Display blank timeout 가져오기 (ms)
    pub fn get_display_blank_timeout_ms(&self) -> u64 {
        match self.mode {
            PowerMode::Performance => 0,      // 블랭크 안 함
            PowerMode::Balanced => 120_000,    // 2분
            PowerMode::PowerSaving => 60_000,   // 1분
        }
    }
}


