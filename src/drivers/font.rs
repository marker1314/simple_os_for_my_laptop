//! 비트맵 폰트 모듈
//!
//! 8x8 비트맵 폰트를 사용하여 텍스트를 렌더링합니다.

use super::framebuffer::Color;

/// 8x8 비트맵 폰트 데이터
/// 각 문자는 8바이트로 표현되며, 각 바이트는 한 행의 8픽셀을 나타냅니다.
const FONT_DATA: &[u8; 2048] = include_bytes!("../../assets/font8x8_basic.bin");

/// 문자를 프레임버퍼에 렌더링
pub fn draw_char(x: usize, y: usize, ch: char, color: Color) {
    let char_index = (ch as usize).min(255);
    let char_offset = char_index * 8;

    for row in 0..8 {
        let byte = FONT_DATA[char_offset + row];
        for col in 0..8 {
            if (byte & (1 << (7 - col))) != 0 {
                super::framebuffer::set_pixel(x + col, y + row, color);
            }
        }
    }
}

/// 문자열을 프레임버퍼에 렌더링
pub fn draw_str(x: usize, y: usize, s: &str, color: Color) {
    let mut x_offset = x;
    for ch in s.chars() {
        if ch == '\n' {
            // 줄바꿈은 무시 (호출자가 처리)
            continue;
        }
        draw_char(x_offset, y, ch, color);
        x_offset += 8;
    }
}

/// 배경과 함께 문자 렌더링
pub fn draw_char_with_bg(x: usize, y: usize, ch: char, fg_color: Color, bg_color: Color) {
    let char_index = (ch as usize).min(255);
    let char_offset = char_index * 8;

    for row in 0..8 {
        let byte = FONT_DATA[char_offset + row];
        for col in 0..8 {
            let color = if (byte & (1 << (7 - col))) != 0 {
                fg_color
            } else {
                bg_color
            };
            super::framebuffer::set_pixel(x + col, y + row, color);
        }
    }
}

/// 배경과 함께 문자열 렌더링
pub fn draw_str_with_bg(x: usize, y: usize, s: &str, fg_color: Color, bg_color: Color) {
    let mut x_offset = x;
    for ch in s.chars() {
        if ch == '\n' {
            continue;
        }
        draw_char_with_bg(x_offset, y, ch, fg_color, bg_color);
        x_offset += 8;
    }
}

/// 폰트 문자 너비
pub const CHAR_WIDTH: usize = 8;

/// 폰트 문자 높이
pub const CHAR_HEIGHT: usize = 8;

