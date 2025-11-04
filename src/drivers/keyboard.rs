//! PS/2 키보드 드라이버
//!
//! 이 모듈은 PS/2 키보드를 제어하고 키 입력을 처리합니다.

use x86_64::instructions::port::Port;
use spin::Mutex;
use crate::interrupts::pic;

/// 키보드 I/O 포트
const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_COMMAND_PORT: u16 = 0x64;
const KEYBOARD_STATUS_PORT: u16 = 0x64;

/// 키보드 상태 레지스터 비트
const STATUS_OUTPUT_BUFFER_FULL: u8 = 0x01;
const STATUS_INPUT_BUFFER_FULL: u8 = 0x02;

/// 키 코드 버퍼 (큐)
const BUFFER_SIZE: usize = 256;
static KEY_BUFFER: Mutex<KeyBuffer> = Mutex::new(KeyBuffer {
    buffer: [0u8; BUFFER_SIZE],
    read_index: 0,
    write_index: 0,
    count: 0,
});

struct KeyBuffer {
    buffer: [u8; BUFFER_SIZE],
    read_index: usize,
    write_index: usize,
    count: usize,
}

impl KeyBuffer {
    fn push(&mut self, key_code: u8) -> bool {
        if self.count >= BUFFER_SIZE {
            return false; // 버퍼 풀
        }
        self.buffer[self.write_index] = key_code;
        self.write_index = (self.write_index + 1) % BUFFER_SIZE;
        self.count += 1;
        true
    }

    fn pop(&mut self) -> Option<u8> {
        if self.count == 0 {
            return None;
        }
        let key_code = self.buffer[self.read_index];
        self.read_index = (self.read_index + 1) % BUFFER_SIZE;
        self.count -= 1;
        Some(key_code)
    }
}

/// 키보드 상태 확인
fn is_output_buffer_full() -> bool {
    unsafe {
        let mut status_port: Port<u8> = Port::new(KEYBOARD_STATUS_PORT);
        (status_port.read() & STATUS_OUTPUT_BUFFER_FULL) != 0
    }
}

/// 키보드에서 스캔 코드 읽기
fn read_scan_code() -> Option<u8> {
    if !is_output_buffer_full() {
        return None;
    }
    unsafe {
        let mut data_port: Port<u8> = Port::new(KEYBOARD_DATA_PORT);
        Some(data_port.read())
    }
}

/// 키보드 인터럽트 핸들러
///
/// IRQ 1 (인터럽트 33)에서 호출됩니다.
pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: x86_64::structures::idt::InterruptStackFrame) {
    // 스캔 코드 읽기
    if let Some(scan_code) = read_scan_code() {
        // 버퍼에 추가
        let mut buffer = KEY_BUFFER.lock();
        if !buffer.push(scan_code) {
            crate::log_warn!("Keyboard buffer full, dropping scan code: 0x{:02X}", scan_code);
        }
    }
    
    // PIC에 인터럽트 종료 신호 전송 (IRQ 1)
    unsafe {
        pic::end_of_interrupt(1);
    }
}

/// 키보드 초기화
///
/// 키보드 컨트롤러를 초기화합니다.
/// # Safety
/// 이 함수는 한 번만 호출되어야 하며, 인터럽트가 비활성화된 상태에서 호출되어야 합니다.
pub unsafe fn init() {
    // 키보드 컨트롤러 리셋 (선택사항)
    // 실제로는 키보드가 이미 작동 중일 수 있으므로,
    // 인터럽트만 활성화하면 됩니다.
    
    crate::log_info!("Keyboard driver initialized");
}

/// 키 코드 읽기 (논블로킹)
///
/// 버퍼에서 키 코드를 읽습니다. 버퍼가 비어있으면 None을 반환합니다.
pub fn read_key() -> Option<u8> {
    KEY_BUFFER.lock().pop()
}

/// 키 코드를 ASCII 문자로 변환 (간단한 구현)
///
/// 실제로는 더 복잡한 키맵핑이 필요하지만, 기본적인 키만 처리합니다.
pub fn scan_code_to_ascii(scan_code: u8) -> Option<char> {
    // 키가 눌린 경우 (최상위 비트가 0)
    // 키가 떼어진 경우 (최상위 비트가 1) - 0x80 이상
    if scan_code & 0x80 != 0 {
        return None; // 키 해제 이벤트는 무시
    }
    
    // 기본 US 키보드 레이아웃 (간단한 버전)
    match scan_code {
        0x01 => None, // Escape
        0x02 => Some('1'),
        0x03 => Some('2'),
        0x04 => Some('3'),
        0x05 => Some('4'),
        0x06 => Some('5'),
        0x07 => Some('6'),
        0x08 => Some('7'),
        0x09 => Some('8'),
        0x0A => Some('9'),
        0x0B => Some('0'),
        0x0C => Some('-'),
        0x0D => Some('='),
        0x0E => Some('\x08'), // Backspace
        0x0F => Some('\t'),    // Tab
        0x10 => Some('q'),
        0x11 => Some('w'),
        0x12 => Some('e'),
        0x13 => Some('r'),
        0x14 => Some('t'),
        0x15 => Some('y'),
        0x16 => Some('u'),
        0x17 => Some('i'),
        0x18 => Some('o'),
        0x19 => Some('p'),
        0x1A => Some('['),
        0x1B => Some(']'),
        0x1C => Some('\n'),    // Enter
        0x1E => Some('a'),
        0x1F => Some('s'),
        0x20 => Some('d'),
        0x21 => Some('f'),
        0x22 => Some('g'),
        0x23 => Some('h'),
        0x24 => Some('j'),
        0x25 => Some('k'),
        0x26 => Some('l'),
        0x27 => Some(';'),
        0x28 => Some('\''),
        0x29 => Some('`'),
        0x2B => Some('\\'),
        0x2C => Some('z'),
        0x2D => Some('x'),
        0x2E => Some('c'),
        0x2F => Some('v'),
        0x30 => Some('b'),
        0x31 => Some('n'),
        0x32 => Some('m'),
        0x33 => Some(','),
        0x34 => Some('.'),
        0x35 => Some('/'),
        0x39 => Some(' '),    // Space
        _ => None,
    }
}

/// 키 입력 읽기 (ASCII 문자)
///
/// 버퍼에서 키 코드를 읽고 ASCII 문자로 변환합니다.
pub fn read_char() -> Option<char> {
    read_key().and_then(scan_code_to_ascii)
}

