//! 사용자 활동 감지 및 전원 관리 조정
//!
//! 이 모듈은 사용자 활동을 추적하고, 활동 패턴에 따라 자동으로 전원 관리를 조정합니다.
//!
//! # 사용자 활동 감지
//!
//! 1. **입력 장치 활동**: 키보드, 마우스 입력 추적
//! 2. **활동 패턴 분석**: 활동 빈도 및 패턴 분석
//! 3. **자동 전원 관리**: 활동 패턴에 따른 전력 프로파일 조정

use spin::Mutex;
use alloc::collections::VecDeque;

use crate::power::policy::PowerMode;
use crate::power::PowerError;

/// 사용자 활동 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityType {
    /// 키보드 입력
    Keyboard,
    /// 마우스 입력
    Mouse,
    /// 터치패드 입력
    Touchpad,
    /// 기타 입력
    Other,
}

/// 사용자 활동 이벤트
#[derive(Debug, Clone, Copy)]
struct ActivityEvent {
    /// 활동 타입
    activity_type: ActivityType,
    /// 활동 시간 (밀리초)
    timestamp: u64,
}

/// 사용자 활동 감지기
pub struct UserActivityDetector {
    /// 최근 활동 이벤트 히스토리
    activity_history: VecDeque<ActivityEvent>,
    /// 최대 히스토리 크기
    max_history_size: usize,
    /// 마지막 활동 시간 (밀리초)
    last_activity_time: u64,
    /// 유휴 상태 진입 임계값 (밀리초)
    idle_threshold_ms: u64,
    /// 활동 감지 활성화 여부
    enabled: bool,
}

impl UserActivityDetector {
    /// 새 활동 감지기 생성
    pub fn new(max_history: usize, idle_threshold_ms: u64) -> Self {
        Self {
            activity_history: VecDeque::new(),
            max_history_size: max_history,
            last_activity_time: 0,
            idle_threshold_ms,
            enabled: true,
        }
    }
    
    /// 활동 이벤트 기록
    pub fn record_activity(&mut self, activity_type: ActivityType) {
        if !self.enabled {
            return;
        }
        
        let now = crate::drivers::timer::get_milliseconds();
        
        // 활동 이벤트 추가
        let event = ActivityEvent {
            activity_type,
            timestamp: now,
        };
        
        self.activity_history.push_back(event);
        
        // 히스토리 크기 제한
        while self.activity_history.len() > self.max_history_size {
            self.activity_history.pop_front();
        }
        
        self.last_activity_time = now;
        
        crate::log_debug!("User activity recorded: {:?} at {}ms", activity_type, now);
    }
    
    /// 현재 유휴 상태 확인
    ///
    /// # Returns
    /// 유휴 상태이면 `true`, 활동 중이면 `false`
    pub fn is_idle(&self) -> bool {
        if !self.enabled {
            return false;
        }
        
        let now = crate::drivers::timer::get_milliseconds();
        now.saturating_sub(self.last_activity_time) >= self.idle_threshold_ms
    }
    
    /// 유휴 시간 가져오기 (밀리초)
    pub fn idle_time(&self) -> u64 {
        let now = crate::drivers::timer::get_milliseconds();
        now.saturating_sub(self.last_activity_time)
    }
    
    /// 최근 활동 빈도 계산 (활동/분)
    pub fn activity_rate(&self) -> f64 {
        if self.activity_history.len() < 2 {
            return 0.0;
        }
        
        let now = crate::drivers::timer::get_milliseconds();
        let oldest = self.activity_history.front().map(|e| e.timestamp).unwrap_or(now);
        let time_span = now.saturating_sub(oldest);
        
        if time_span == 0 {
            return 0.0;
        }
        
        // 활동 수를 분당 활동 수로 변환
        let activities_per_ms = self.activity_history.len() as f64 / time_span as f64;
        activities_per_ms * 60000.0 // 분당 활동 수
    }
    
    /// 활동 패턴 분석
    ///
    /// 활동 패턴에 따라 권장 전력 모드를 반환합니다.
    pub fn analyze_activity_pattern(&self) -> PowerMode {
        if !self.enabled {
            return PowerMode::Balanced;
        }
        
        let idle_time = self.idle_time();
        let activity_rate = self.activity_rate();
        
        // 유휴 상태
        if idle_time >= self.idle_threshold_ms * 2 {
            return PowerMode::PowerSaving;
        }
        
        // 높은 활동 빈도
        if activity_rate > 30.0 {
            return PowerMode::Performance;
        }
        
        // 중간 활동
        PowerMode::Balanced
    }
    
    /// 활동 통계 가져오기
    pub fn get_stats(&self) -> (u64, f64, bool) {
        (
            self.idle_time(),
            self.activity_rate(),
            self.is_idle(),
        )
    }
}

/// 전역 사용자 활동 감지기
static USER_ACTIVITY_DETECTOR: Mutex<UserActivityDetector> = Mutex::new(UserActivityDetector {
    activity_history: VecDeque::new(),
    max_history_size: 100,
    last_activity_time: 0,
    idle_threshold_ms: 30000, // 30초
    enabled: true,
});

/// 사용자 활동 감지기 초기화
pub fn init_user_activity_detector(max_history: usize, idle_threshold_ms: u64) {
    let mut detector = USER_ACTIVITY_DETECTOR.lock();
    *detector = UserActivityDetector::new(max_history, idle_threshold_ms);
    crate::log_info!("User activity detector initialized (idle threshold: {}ms)", idle_threshold_ms);
}

/// 활동 이벤트 기록
pub fn record_activity(activity_type: ActivityType) {
    let mut detector = USER_ACTIVITY_DETECTOR.lock();
    detector.record_activity(activity_type);
}

/// 유휴 상태 확인
pub fn is_user_idle() -> bool {
    let detector = USER_ACTIVITY_DETECTOR.lock();
    detector.is_idle()
}

/// 유휴 시간 가져오기
pub fn get_idle_time() -> u64 {
    let detector = USER_ACTIVITY_DETECTOR.lock();
    detector.idle_time()
}

/// 활동 패턴 분석
pub fn analyze_activity_pattern() -> PowerMode {
    let detector = USER_ACTIVITY_DETECTOR.lock();
    detector.analyze_activity_pattern()
}

/// 활동 통계 가져오기
pub fn get_activity_stats() -> (u64, f64, bool) {
    let detector = USER_ACTIVITY_DETECTOR.lock();
    detector.get_stats()
}

/// 활동 기반 전원 관리 조정
///
/// 활동 패턴을 분석하여 전력 프로파일을 자동으로 조정합니다.
pub fn adjust_power_based_on_activity() -> Result<(), PowerError> {
    let recommended_mode = analyze_activity_pattern();
    
    // 현재 전력 관리자 가져오기
    if let Some(pm) = crate::power::get_manager() {
        if let Some(ref mut manager) = pm.lock().as_mut() {
            let current_mode = manager.get_policy();
            
            // 권장 모드가 현재 모드와 다르면 변경
            if recommended_mode != current_mode {
                crate::log_info!("Adjusting power mode: {:?} -> {:?} (based on user activity)", 
                                current_mode, recommended_mode);
                
                manager.set_policy(recommended_mode)?;
            }
        }
    }
    
    Ok(())
}

