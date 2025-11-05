//! 로깅 시스템
//!
//! 이 모듈은 커널 전역 로깅 시스템을 제공합니다.
//! 현재는 시리얼 포트를 통한 로깅만 지원합니다.

use core::fmt;
use spin::Mutex;

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

const RING_CAPACITY: usize = 256;
const MAX_LOG_LINE_LEN: usize = 128;

/// 구조화된 로그 엔트리
#[derive(Clone, Copy)]
struct LogEntry {
    timestamp_ms: u64,
    level: LogLevel,
    message: [u8; MAX_LOG_LINE_LEN],
    message_len: usize,
}

impl LogEntry {
    const fn new() -> Self {
        Self {
            timestamp_ms: 0,
            level: LogLevel::Info,
            message: [0; MAX_LOG_LINE_LEN],
            message_len: 0,
        }
    }

    fn set(&mut self, timestamp_ms: u64, level: LogLevel, msg: &str) {
        self.timestamp_ms = timestamp_ms;
        self.level = level;
        self.message_len = msg.len().min(MAX_LOG_LINE_LEN - 1);
        self.message[..self.message_len].copy_from_slice(msg.as_bytes());
        self.message[self.message_len] = 0;
    }

    fn get_message(&self) -> &str {
        // SAFETY: message is always valid UTF-8 and null-terminated
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                self.message.as_ptr(),
                self.message_len,
            ))
        }
    }
}

struct LogRing {
    entries: [LogEntry; RING_CAPACITY],
    head: usize,
    count: usize,
}

impl LogRing {
    const fn new() -> Self {
        Self {
            entries: [LogEntry::new(); RING_CAPACITY],
            head: 0,
            count: 0,
        }
    }

    fn push(&mut self, timestamp_ms: u64, level: LogLevel, msg: &str) {
        self.entries[self.head].set(timestamp_ms, level, msg);
        self.head = (self.head + 1) % RING_CAPACITY;
        if self.count < RING_CAPACITY {
            self.count += 1;
        }
    }

    fn for_each<F: FnMut(&LogEntry)>(&self, mut f: F) {
        let start = if self.count == RING_CAPACITY { self.head } else { 0 };
        for i in 0..self.count {
            let idx = (start + i) % RING_CAPACITY;
            f(&self.entries[idx]);
        }
    }

    fn filter_by_level<F: FnMut(&LogEntry)>(&self, level: LogLevel, mut f: F) {
        let start = if self.count == RING_CAPACITY { self.head } else { 0 };
        for i in 0..self.count {
            let idx = (start + i) % RING_CAPACITY;
            if self.entries[idx].level == level {
                f(&self.entries[idx]);
            }
        }
    }
}

static LOG_RING: Mutex<LogRing> = Mutex::new(LogRing::new());

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
        
        // 타임스탬프 가져오기
        let timestamp_ms = crate::drivers::timer::get_milliseconds();
        
        // 메시지를 문자열로 포맷팅 (임시 버퍼 사용)
        let mut buf = [0u8; MAX_LOG_LINE_LEN];
        let mut log_buf = LogBuffer { buf: &mut buf, pos: 0 };
        
        // 포맷팅 시도
        let fmt_result = core::fmt::Write::write_fmt(&mut log_buf, args);
        
        // 시리얼 포트로 출력
        crate::serial_print!("{} ", prefix);
        crate::serial_print!("{}\n", args);

        // 로그 버퍼에 저장 (포맷팅 성공 시에만)
        if fmt_result.is_ok() && log_buf.pos > 0 {
            let msg_str = core::str::from_utf8(&buf[..log_buf.pos.min(MAX_LOG_LINE_LEN - 1)])
                .unwrap_or("");
            LOG_RING.lock().push(timestamp_ms, level, msg_str);
        } else {
            // 포맷팅 실패 시 prefix만 저장
            LOG_RING.lock().push(timestamp_ms, level, prefix);
        }
    }
}

/// 임시 로그 버퍼 (포맷팅용)
struct LogBuffer<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> core::fmt::Write for LogBuffer<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buf.len().saturating_sub(self.pos);
        let to_write = bytes.len().min(remaining.saturating_sub(1));
        if to_write > 0 {
            self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
            self.pos += to_write;
        }
        Ok(())
    }
}

/// 최근 로그 덤프
pub fn dump_recent() {
    crate::serial_println!("\n--- Recent Logs ---");
    LOG_RING.lock().for_each(|entry| {
        let level_str = match entry.level {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN ",
            LogLevel::Info => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        };
        crate::serial_println!("[{}ms] [{}] {}", entry.timestamp_ms, level_str, entry.get_message());
    });
    crate::serial_println!("--- End Logs ---\n");
}

/// 특정 레벨의 로그만 필터링하여 출력
pub fn dump_by_level(level: LogLevel) {
    let level_str = match level {
        LogLevel::Error => "ERROR",
        LogLevel::Warn => "WARN ",
        LogLevel::Info => "INFO ",
        LogLevel::Debug => "DEBUG",
        LogLevel::Trace => "TRACE",
    };
    crate::serial_println!("\n--- Recent {} Logs ---", level_str);
    LOG_RING.lock().filter_by_level(level, |entry| {
        crate::serial_println!("[{}ms] {}", entry.timestamp_ms, entry.get_message());
    });
    crate::serial_println!("--- End {} Logs ---\n", level_str);
}

/// CSV 형식으로 로그 내보내기
pub fn export_csv() {
    crate::serial_println!("timestamp_ms,level,message");
    LOG_RING.lock().for_each(|entry| {
        let level_str = match entry.level {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        };
        let msg = entry.get_message();
        // CSV 이스케이프: 따옴표를 두 개로 변환 (수동 처리)
        let mut escaped_buf = [0u8; MAX_LOG_LINE_LEN * 2];
        let mut pos = 0;
        for byte in msg.bytes() {
            if byte == b'"' && pos < escaped_buf.len() - 1 {
                escaped_buf[pos] = b'"';
                pos += 1;
                if pos < escaped_buf.len() {
                    escaped_buf[pos] = b'"';
                    pos += 1;
                }
            } else if pos < escaped_buf.len() {
                escaped_buf[pos] = byte;
                pos += 1;
            }
        }
        let escaped = core::str::from_utf8(&escaped_buf[..pos]).unwrap_or(msg);
        crate::serial_println!("{},\"{}\",\"{}\"", entry.timestamp_ms, level_str, escaped);
    });
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

