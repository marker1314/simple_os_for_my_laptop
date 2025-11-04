//! 터미널 에뮬레이터 애플리케이션
//!
//! GUI 환경에서 실행되는 터미널 에뮬레이터입니다.

use crate::drivers::framebuffer::Color;
use crate::drivers::font;
use crate::gui::Window;
use alloc::string::String;
use alloc::vec::Vec;

/// 터미널 에뮬레이터
pub struct Terminal {
    window: Window,
    lines: Vec<String>,
    input_buffer: String,
    cursor_pos: usize,
    scroll_offset: usize,
    prompt: String,
}

impl Terminal {
    /// 새 터미널 생성
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        let window = Window::new(x, y, width, height, "Terminal");
        let mut lines = Vec::new();
        
        // 환영 메시지
        lines.push(String::from("Simple OS Terminal v0.1"));
        lines.push(String::from("Type 'help' for available commands"));
        lines.push(String::from(""));

        Terminal {
            window,
            lines,
            input_buffer: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            prompt: String::from("$ "),
        }
    }

    /// 터미널 렌더링
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

        // 히스토리 렌더링
        let total_lines = self.lines.len();
        let start_line = if total_lines > visible_rows - 1 {
            self.scroll_offset
        } else {
            0
        };

        for (i, line_index) in (start_line..self.lines.len()).enumerate() {
            if i >= visible_rows - 1 {
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

            font::draw_str(text_area_x, y, display_text, Color::new(200, 200, 200));
        }

        // 현재 입력 줄 렌더링
        let input_line_y = text_area_y + (visible_rows - 1) * font::CHAR_HEIGHT;
        
        // 프롬프트
        font::draw_str(text_area_x, input_line_y, &self.prompt, Color::new(0, 255, 0));

        // 입력 버퍼
        let input_x = text_area_x + self.prompt.len() * font::CHAR_WIDTH;
        font::draw_str(input_x, input_line_y, &self.input_buffer, Color::new(200, 200, 200));

        // 커서 (포커스 있을 때만)
        if self.window.is_focused {
            let cursor_x = input_x + self.cursor_pos * font::CHAR_WIDTH;
            crate::drivers::framebuffer::fill_rect(
                cursor_x,
                input_line_y,
                font::CHAR_WIDTH,
                font::CHAR_HEIGHT,
                Color::new(200, 200, 200),
            );

            // 커서 위치의 문자를 반전 색상으로 표시
            if self.cursor_pos < self.input_buffer.len() {
                if let Some(ch) = self.input_buffer.chars().nth(self.cursor_pos) {
                    let ch_str = alloc::format!("{}", ch);
                    font::draw_str(cursor_x, input_line_y, &ch_str, Color::BLACK);
                }
            }
        }
    }

    /// 키 입력 처리 (스캔 코드 기반)
    pub fn handle_key(&mut self, scan_code: u8) {
        match scan_code {
            0x1C => { // Enter
                // Enter: 명령 실행
                let command = self.input_buffer.clone();
                
                // 입력한 명령을 히스토리에 추가
                let mut prompt_line = self.prompt.clone();
                prompt_line.push_str(&command);
                self.lines.push(prompt_line);

                // 명령 실행
                self.execute_command(&command);

                // 입력 버퍼 초기화
                self.input_buffer.clear();
                self.cursor_pos = 0;

                // 스크롤을 맨 아래로
                self.update_scroll();
            }
            0x0E => { // Backspace
                if self.cursor_pos > 0 {
                    self.input_buffer.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            }
            0x4B => { // Left arrow (extended)
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            0x4D => { // Right arrow (extended)
                if self.cursor_pos < self.input_buffer.len() {
                    self.cursor_pos += 1;
                }
            }
            0x47 => { // Home (extended)
                self.cursor_pos = 0;
            }
            0x4F => { // End (extended)
                self.cursor_pos = self.input_buffer.len();
            }
            _ => {}
        }
    }

    /// 문자 입력
    pub fn insert_char(&mut self, ch: char) {
        if ch.is_ascii_graphic() || ch == ' ' {
            self.input_buffer.insert(self.cursor_pos, ch);
            self.cursor_pos += 1;
        }
    }

    /// 명령 실행
    fn execute_command(&mut self, command: &str) {
        let command = command.trim();

        if command.is_empty() {
            return;
        }

        match command {
            "help" => {
                self.lines.push(String::from("Available commands:"));
                self.lines.push(String::from("  help     - Show this help message"));
                self.lines.push(String::from("  clear    - Clear the terminal"));
                self.lines.push(String::from("  echo     - Echo text"));
                self.lines.push(String::from("  uptime   - Show system uptime"));
                self.lines.push(String::from("  mem      - Show memory information"));
                self.lines.push(String::from("  version  - Show OS version"));
            }
            "clear" => {
                self.lines.clear();
            }
            "version" => {
                self.lines.push(String::from("Simple OS v0.1.0"));
                self.lines.push(String::from("Rust-based laptop operating system"));
            }
            "uptime" => {
                // 실제로는 타이머 드라이버에서 시간 가져와야 함
                self.lines.push(String::from("System uptime: 0d 0h 5m"));
            }
            "mem" => {
                self.lines.push(String::from("Memory Information:"));
                self.lines.push(String::from("  Total: 512 MB"));
                self.lines.push(String::from("  Used:  128 MB"));
                self.lines.push(String::from("  Free:  384 MB"));
            }
            cmd if cmd.starts_with("echo ") => {
                let text = &cmd[5..];
                self.lines.push(String::from(text));
            }
            _ => {
                let mut error = String::from("Unknown command: ");
                error.push_str(command);
                self.lines.push(error);
                self.lines.push(String::from("Type 'help' for available commands"));
            }
        }
    }

    /// 출력 추가
    pub fn print(&mut self, text: &str) {
        for line in text.lines() {
            self.lines.push(String::from(line));
        }
        self.update_scroll();
    }

    /// 스크롤 업데이트
    fn update_scroll(&mut self) {
        const TITLE_BAR_HEIGHT: usize = 24;
        const PADDING: usize = 8;
        let text_area_height = self.window.height - TITLE_BAR_HEIGHT - PADDING * 2;
        let visible_rows = text_area_height / font::CHAR_HEIGHT;

        // 스크롤을 맨 아래로
        if self.lines.len() > visible_rows - 1 {
            self.scroll_offset = self.lines.len() - (visible_rows - 1);
        } else {
            self.scroll_offset = 0;
        }
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
}

