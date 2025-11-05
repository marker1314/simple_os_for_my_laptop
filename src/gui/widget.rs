//! GUI 위젯
//!
//! 버튼, 텍스트 박스 등의 GUI 위젯을 제공합니다.

use crate::drivers::framebuffer::Color;
use crate::drivers::font;
use alloc::string::String;

/// 위젯 트레이트
pub trait Widget {
    fn render(&self);
    fn contains_point(&self, x: isize, y: isize) -> bool;
}

/// 버튼 위젯
pub struct Button {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub text: String,
    pub is_pressed: bool,
    pub is_hovered: bool,
    pub enabled: bool,
}

impl Button {
    /// 새 버튼 생성
    pub fn new(x: usize, y: usize, width: usize, height: usize, text: &str) -> Self {
        Button {
            x,
            y,
            width,
            height,
            text: String::from(text),
            is_pressed: false,
            is_hovered: false,
            enabled: true,
        }
    }

    /// 버튼 색상 결정
    fn get_color(&self) -> Color {
        if !self.enabled {
            Color::GRAY
        } else if self.is_pressed {
            Color::new(0, 100, 180)
        } else if self.is_hovered {
            Color::new(0, 140, 220)
        } else {
            Color::new(0, 120, 215)
        }
    }
}

impl Widget for Button {
    fn render(&self) {
        // 버튼 배경
        crate::drivers::framebuffer::fill_rect(
            self.x,
            self.y,
            self.width,
            self.height,
            self.get_color(),
        );

        // 버튼 테두리
        crate::drivers::framebuffer::draw_rect(
            self.x,
            self.y,
            self.width,
            self.height,
            if self.enabled {
                Color::new(0, 80, 150)
            } else {
                Color::DARK_GRAY
            },
        );

        // 버튼 텍스트 (중앙 정렬)
        let text_width = self.text.len() * font::CHAR_WIDTH;
        let text_x = self.x + (self.width.saturating_sub(text_width)) / 2;
        let text_y = self.y + (self.height.saturating_sub(font::CHAR_HEIGHT)) / 2;

        font::draw_str(
            text_x,
            text_y,
            &self.text,
            if self.enabled {
                Color::WHITE
            } else {
                Color::LIGHT_GRAY
            },
        );
    }

    fn contains_point(&self, x: isize, y: isize) -> bool {
        x >= self.x as isize
            && x < (self.x + self.width) as isize
            && y >= self.y as isize
            && y < (self.y + self.height) as isize
    }
}

/// 텍스트 박스 위젯
pub struct TextBox {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub text: String,
    pub is_focused: bool,
    pub cursor_pos: usize,
}

impl TextBox {
    /// 새 텍스트 박스 생성
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        TextBox {
            x,
            y,
            width,
            height,
            text: String::new(),
            is_focused: false,
            cursor_pos: 0,
        }
    }

    /// 문자 입력
    pub fn insert_char(&mut self, ch: char) {
        if self.cursor_pos <= self.text.len() {
            self.text.insert(self.cursor_pos, ch);
            self.cursor_pos += 1;
        }
    }

    /// 백스페이스
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.text.remove(self.cursor_pos - 1);
            self.cursor_pos -= 1;
        }
    }

    /// 커서 이동
    pub fn move_cursor(&mut self, delta: isize) {
        let new_pos = (self.cursor_pos as isize + delta).max(0) as usize;
        self.cursor_pos = new_pos.min(self.text.len());
    }
}

impl Widget for TextBox {
    fn render(&self) {
        // 배경
        crate::drivers::framebuffer::fill_rect(
            self.x,
            self.y,
            self.width,
            self.height,
            Color::WHITE,
        );

        // 테두리
        crate::drivers::framebuffer::draw_rect(
            self.x,
            self.y,
            self.width,
            self.height,
            if self.is_focused {
                Color::new(0, 120, 215)
            } else {
                Color::GRAY
            },
        );

        // 텍스트
        let text_x = self.x + 4;
        let text_y = self.y + (self.height.saturating_sub(font::CHAR_HEIGHT)) / 2;
        font::draw_str(text_x, text_y, &self.text, Color::BLACK);

        // 커서 (포커스 있을 때만)
        if self.is_focused {
            let cursor_x = text_x + self.cursor_pos * font::CHAR_WIDTH;
            crate::drivers::framebuffer::draw_line(
                cursor_x as isize,
                text_y as isize,
                cursor_x as isize,
                (text_y + font::CHAR_HEIGHT) as isize,
                Color::BLACK,
            );
        }
    }

    fn contains_point(&self, x: isize, y: isize) -> bool {
        x >= self.x as isize
            && x < (self.x + self.width) as isize
            && y >= self.y as isize
            && y < (self.y + self.height) as isize
    }
}


