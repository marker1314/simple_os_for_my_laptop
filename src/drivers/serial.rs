//! 시리얼 포트 드라이버
//!
//! 이 모듈은 UART 16550 시리얼 포트를 사용하여 로깅 및 디버깅을 제공합니다.

use uart_16550::SerialPort;
use spin::Mutex;

/// COM1 시리얼 포트 (주소 0x3F8)
pub static SERIAL1: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3F8) });

/// 시리얼 포트 초기화
///
/// 커널 초기화 시 한 번 호출되어야 합니다.
pub fn init() {
    SERIAL1.lock().init();
}

/// 시리얼 포트를 통한 출력 매크로
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::drivers::serial::_print(format_args!($($arg)*));
    };
}

/// 시리얼 포트를 통한 출력 매크로 (줄바꿈 포함)
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}

/// 내부 출력 함수 (매크로에서 사용)
#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    let _ = SERIAL1.lock().write_fmt(args);
}

/// 단일 바이트 출력 (시스템 콜용)
pub fn write_byte(byte: u8) {
    SERIAL1.lock().send(byte);
}

