//! 로깅 시스템
//!
//! 이 모듈은 커널 전역 로깅 시스템을 제공합니다.
//! 현재는 시리얼 포트를 통한 로깅만 지원합니다.

use core::fmt;

/// 로그 레벨
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// 에러 레벨 (항상 출력)
    Error = 0,
    /// 경고 레벨
    Warn = 1,
    /// 정보 레벨
    Info = 2,
    /// 디버그 레벨
    Debug = 3,
    /// 트레이스 레벨 (가장 상세)
    Trace = 4,
}

/// 현재 로그 레벨 (컴파일 타임에 설정 가능)
pub const LOG_LEVEL: LogLevel = LogLevel::Debug;

/// 로그 출력 함수
pub fn log(level: LogLevel, args: fmt::Arguments) {
    if level <= LOG_LEVEL {
        let prefix = match level {
            LogLevel::Error => "[ERROR]",
            LogLevel::Warn => "[WARN] ",
            LogLevel::Info => "[INFO] ",
            LogLevel::Debug => "[DEBUG]",
            LogLevel::Trace => "[TRACE]",
        };
        
        // 시리얼 포트로 출력
        crate::serial_print!("{} ", prefix);
        crate::serial_print!("{}\n", args);
    }
}

/// 에러 레벨 로그 매크로
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Error, format_args!($($arg)*));
    };
}

/// 경고 레벨 로그 매크로
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Warn, format_args!($($arg)*));
    };
}

/// 정보 레벨 로그 매크로
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Info, format_args!($($arg)*));
    };
}

/// 디버그 레벨 로그 매크로
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Debug, format_args!($($arg)*));
    };
}

/// 트레이스 레벨 로그 매크로
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Trace, format_args!($($arg)*));
    };
}

