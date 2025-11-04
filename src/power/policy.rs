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
}

impl PowerPolicy {
    /// 새 전력 정책 생성
    pub fn new(mode: PowerMode) -> Self {
        Self { mode }
    }
    
    /// 전력 모드 설정
    pub fn set_mode(&mut self, mode: PowerMode) {
        self.mode = mode;
    }
    
    /// 현재 전력 모드 가져오기
    pub fn get_mode(&self) -> PowerMode {
        self.mode
    }
}

