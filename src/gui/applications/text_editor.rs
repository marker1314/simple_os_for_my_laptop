//! 텍스트 에디터 애플리케이션
//!
//! 간단한 텍스트 편집 기능을 제공하는 GUI 애플리케이션입니다.

use crate::drivers::framebuffer::Color;
use crate::drivers::font;
use crate::gui::Window;
use alloc::string::String;
use alloc::vec::Vec;

/// 텍스트 에디터
pub struct TextEditor {
    window: Window,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_offset: usize,
    filename: String,
    modified: bool,
}

impl TextEditor {
    /// 새 텍스트 에디터 생성
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        let window = Window::new(x, y, width, height, "Text Editor - Untitled");
        let mut lines = Vec::new();
        lines.push(String::new());

        TextEditor {
            window,
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            filename: String::from("Untitled"),
            modified: false,
        }
    }

    /// 파일 로드
    pub fn load_file(&mut self, filename: &str, content: &str) {
        self.filename = String::from(filename);
        self.lines.clear();
        
        for line in content.lines() {
            self.lines.push(String::from(line));
        }
        
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.modified = false;
        
        self.update_title();
    }

    /// 텍스트 내용 가져오기
    pub fn get_content(&self) -> String {
        let mut content = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            content.push_str(line);
            if i < self.lines.len() - 1 {
                content.push('\n');
            }
        }
        content
    }

    /// 에디터 렌더링
    pub fn render(&self) {
        // 윈도우 렌더링
        self.window.render();

        const TITLE_BAR_HEIGHT: usize = 24;
        const PADDING: usize = 8;
        
        let text_area_x = self.window.x + PADDING;
        let text_area_y = self.window.y + TITLE_BAR_HEIGHT + PADDING;
        let text_area_width = self.window.width - PADDING * 2;
        let text_area_height = self.window.height - TITLE_BAR_HEIGHT - PADDING * 2;

        // 표시 가능한 줄 수 계산
        let visible_rows = text_area_height / font::CHAR_HEIGHT;

        // 텍스트 렌더링
        for (i, line_index) in (self.scroll_offset..self.lines.len()).enumerate() {
            if i >= visible_rows {
                break;
            }

            let y = text_area_y + i * font::CHAR_HEIGHT;
            let line = &self.lines[line_index];

            // 최대 표시 문자 수
            let max_chars = text_area_width / font::CHAR_WIDTH;
            let display_text = if line.len() > max_chars {
                &line[..max_chars]
            } else {
                line
            };

            font::draw_str(text_area_x, y, display_text, Color::BLACK);

            // 커서 렌더링 (현재 줄에만)
            if line_index == self.cursor_row && self.window.is_focused {
                let cursor_x = text_area_x + self.cursor_col * font::CHAR_WIDTH;
                crate::drivers::framebuffer::draw_line(
                    cursor_x as isize,
                    y as isize,
                    cursor_x as isize,
                    (y + font::CHAR_HEIGHT) as isize,
                    Color::BLACK,
                );
            }
        }

        // 상태 표시줄
        let status_y = self.window.y + self.window.height - 20;
        crate::drivers::framebuffer::fill_rect(
            self.window.x,
            status_y,
            self.window.width,
            20,
            Color::LIGHT_GRAY,
        );

        // 상태 정보
        let mut status = String::new();
        use core::fmt::Write;
        let _ = write!(&mut status, "Ln {}, Col {} ", self.cursor_row + 1, self.cursor_col + 1);
        if self.modified {
            status.push_str("[Modified]");
        }

        font::draw_str(
            self.window.x + 8,
            status_y + 6,
            &status,
            Color::BLACK,
        );
    }

    /// 키 입력 처리 (스캔 코드 기반)
    pub fn handle_key(&mut self, scan_code: u8) {
        match scan_code {
            0x1C => { // Enter
                // Enter: 새 줄 추가
                let current_line = &self.lines[self.cursor_row];
                let rest_of_line = String::from(&current_line[self.cursor_col..]);
                self.lines[self.cursor_row].truncate(self.cursor_col);
                
                self.cursor_row += 1;
                self.lines.insert(self.cursor_row, rest_of_line);
                self.cursor_col = 0;
                
                self.modified = true;
                self.update_scroll();
            }
            0x0E => { // Backspace
                // Backspace
                if self.cursor_col > 0 {
                    self.lines[self.cursor_row].remove(self.cursor_col - 1);
                    self.cursor_col -= 1;
                    self.modified = true;
                } else if self.cursor_row > 0 {
                    // 이전 줄과 병합
                    let current_line = self.lines.remove(self.cursor_row);
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                    self.lines[self.cursor_row].push_str(&current_line);
                    self.modified = true;
                    self.update_scroll();
                }
            }
            0x4B => { // Left arrow (extended)
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                    self.update_scroll();
                }
            }
            0x4D => { // Right arrow (extended)
                if self.cursor_col < self.lines[self.cursor_row].len() {
                    self.cursor_col += 1;
                } else if self.cursor_row < self.lines.len() - 1 {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                    self.update_scroll();
                }
            }
            0x48 => { // Up arrow (extended)
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                    self.update_scroll();
                }
            }
            0x50 => { // Down arrow (extended)
                if self.cursor_row < self.lines.len() - 1 {
                    self.cursor_row += 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                    self.update_scroll();
                }
            }
            _ => {}
        }
        
        self.update_title();
    }

    /// 문자 입력
    pub fn insert_char(&mut self, ch: char) {
        if ch.is_ascii_graphic() || ch == ' ' {
            self.lines[self.cursor_row].insert(self.cursor_col, ch);
            self.cursor_col += 1;
            self.modified = true;
            self.update_title();
        }
    }

    /// 스크롤 업데이트
    fn update_scroll(&mut self) {
        const TITLE_BAR_HEIGHT: usize = 24;
        const PADDING: usize = 8;
        let text_area_height = self.window.height - TITLE_BAR_HEIGHT - PADDING * 2 - 20; // 상태바 제외
        let visible_rows = text_area_height / font::CHAR_HEIGHT;

        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        } else if self.cursor_row >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.cursor_row - visible_rows + 1;
        }
    }

    /// 타이틀 업데이트
    fn update_title(&mut self) {
        let mut title = String::from("Text Editor - ");
        title.push_str(&self.filename);
        if self.modified {
            title.push_str(" *");
        }
        self.window.title = title;
    }

    /// 윈도우가 마우스 좌표를 포함하는지 확인
    pub fn contains_point(&self, x: isize, y: isize) -> bool {
        self.window.contains_point(x, y)
    }

    /// 포커스 설정
    pub fn set_focus(&mut self, focused: bool) {
        self.window.set_focus(focused);
    }

    /// 윈도우 이동
    pub fn move_to(&mut self, x: usize, y: usize) {
        self.window.move_to(x, y);
    }

    /// 타이틀 바 클릭 확인
    pub fn is_in_title_bar(&self, x: isize, y: isize) -> bool {
        self.window.is_in_title_bar(x, y)
    }

    /// 수정 여부
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// 파일명 가져오기
    pub fn filename(&self) -> &str {
        &self.filename
    }
}

