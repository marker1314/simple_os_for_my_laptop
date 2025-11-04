//! VGA 텍스트 모드 드라이버
//!
//! 이 모듈은 VGA 텍스트 모드 (80x25)를 제어하고 화면에 텍스트를 출력합니다.

use volatile::Volatile;
use spin::Mutex;

/// VGA 텍스트 버퍼 시작 주소
const VGA_BUFFER_ADDRESS: usize = 0xB8000;

/// 화면 크기
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// VGA 색상 코드
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// 색상 코드 (전경 + 배경)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// 화면 문자
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// VGA 텍스트 버퍼
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// VGA 텍스트 모드 드라이버
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// 새로운 Writer 생성
    pub fn new(foreground_color: Color, background_color: Color) -> Self {
        Writer {
            column_position: 0,
            color_code: ColorCode::new(foreground_color, background_color),
            buffer: unsafe { &mut *(VGA_BUFFER_ADDRESS as *mut Buffer) },
        }
    }

    /// 단일 바이트 출력
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                Volatile::write(&mut self.buffer.chars[row][col], ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    /// 문자열 출력
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // 출력 가능한 ASCII 바이트 또는 개행
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // 출력 불가능한 바이트는 ■로 표시
                _ => self.write_byte(0xfe),
            }
        }
    }

    /// 새 줄 처리
    fn new_line(&mut self) {
        // 모든 행을 한 줄씩 위로 이동
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = Volatile::read(&self.buffer.chars[row][col]);
                Volatile::write(&mut self.buffer.chars[row - 1][col], character);
            }
        }
        // 마지막 줄을 공백으로 채움
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    /// 행을 공백으로 채움
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            Volatile::write(&mut self.buffer.chars[row][col], blank);
        }
    }

    /// 화면 전체 지우기
    pub fn clear_screen(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
        self.column_position = 0;
    }

    /// 커서 위치 설정
    pub fn set_cursor_position(&mut self, row: usize, col: usize) {
        if row < BUFFER_HEIGHT && col < BUFFER_WIDTH {
            self.column_position = col;
            // TODO: 실제 하드웨어 커서 위치 설정
        }
    }

    /// 색상 변경
    pub fn set_color(&mut self, foreground: Color, background: Color) {
        self.color_code = ColorCode::new(foreground, background);
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// 전역 Writer 인스턴스
/// 
/// 정적 초기화를 위해 const 함수 사용
static WRITER: Mutex<Writer> = Mutex::new(Writer {
    column_position: 0,
    color_code: ColorCode((Color::LightGray as u8) | ((Color::Black as u8) << 4)),
    buffer: unsafe { &mut *(VGA_BUFFER_ADDRESS as *mut Buffer) },
});

/// VGA 초기화
pub fn init() {
    WRITER.lock().clear_screen();
    crate::log_info!("VGA text mode driver initialized");
}

/// 단일 문자 출력 (시스템 콜용)
pub fn write_char(c: char) {
    WRITER.lock().write_byte(c as u8);
}

/// 새 줄 출력 (시스템 콜용)
pub fn newline() {
    WRITER.lock().write_byte(b'\n');
}

/// VGA 출력 매크로
#[macro_export]
macro_rules! vga_print {
    ($($arg:tt)*) => {
        use core::fmt::Write;
        $crate::drivers::vga::WRITER.lock().write_fmt(format_args!($($arg)*)).unwrap();
    };
}

/// VGA 출력 매크로 (줄바꿈 포함)
#[macro_export]
macro_rules! vga_println {
    () => ($crate::vga_print!("\n"));
    ($fmt:expr) => ($crate::vga_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::vga_print!(concat!($fmt, "\n"), $($arg)*));
}

