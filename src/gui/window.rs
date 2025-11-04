//! 윈도우 관리
//!
//! GUI 윈도우의 기본 구조와 렌더링을 제공합니다.

use crate::drivers::framebuffer::Color;
use crate::drivers::font;
use alloc::string::String;
use alloc::vec::Vec;

/// 윈도우 구조체
pub struct Window {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub title: String,
    pub background_color: Color,
    pub border_color: Color,
    pub title_bar_color: Color,
    pub is_focused: bool,
    pub is_visible: bool,
}

impl Window {
    /// 새 윈도우 생성
    pub fn new(x: usize, y: usize, width: usize, height: usize, title: &str) -> Self {
        Window {
            x,
            y,
            width,
            height,
            title: String::from(title),
            background_color: Color::new(240, 240, 240),
            border_color: Color::new(100, 100, 100),
            title_bar_color: Color::new(0, 120, 215),
            is_focused: false,
            is_visible: true,
        }
    }

    /// 윈도우 렌더링
    pub fn render(&self) {
        if !self.is_visible {
            return;
        }

        const TITLE_BAR_HEIGHT: usize = 24;

        // 타이틀 바 그리기
        crate::drivers::framebuffer::fill_rect(
            self.x,
            self.y,
            self.width,
            TITLE_BAR_HEIGHT,
            if self.is_focused {
                self.title_bar_color
            } else {
                Color::GRAY
            },
        );

        // 타이틀 텍스트
        font::draw_str(self.x + 8, self.y + 8, &self.title, Color::WHITE);

        // 윈도우 내용 영역
        crate::drivers::framebuffer::fill_rect(
            self.x,
            self.y + TITLE_BAR_HEIGHT,
            self.width,
            self.height - TITLE_BAR_HEIGHT,
            self.background_color,
        );

        // 테두리
        crate::drivers::framebuffer::draw_rect(
            self.x,
            self.y,
            self.width,
            self.height,
            self.border_color,
        );
    }

    /// 윈도우 이동
    pub fn move_to(&mut self, x: usize, y: usize) {
        self.x = x;
        self.y = y;
    }

    /// 윈도우 크기 변경
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width.max(100);
        self.height = height.max(50);
    }

    /// 포커스 설정
    pub fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    /// 가시성 설정
    pub fn set_visible(&mut self, visible: bool) {
        self.is_visible = visible;
    }

    /// 마우스 좌표가 윈도우 내부에 있는지 확인
    pub fn contains_point(&self, x: isize, y: isize) -> bool {
        if !self.is_visible {
            return false;
        }

        x >= self.x as isize
            && x < (self.x + self.width) as isize
            && y >= self.y as isize
            && y < (self.y + self.height) as isize
    }

    /// 마우스 좌표가 타이틀 바에 있는지 확인
    pub fn is_in_title_bar(&self, x: isize, y: isize) -> bool {
        const TITLE_BAR_HEIGHT: usize = 24;

        if !self.is_visible {
            return false;
        }

        x >= self.x as isize
            && x < (self.x + self.width) as isize
            && y >= self.y as isize
            && y < (self.y + TITLE_BAR_HEIGHT) as isize
    }
}

