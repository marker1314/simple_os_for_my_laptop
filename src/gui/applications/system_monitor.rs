//! 시스템 모니터 애플리케이션
//!
//! CPU 사용률, 메모리 사용량, 프로세스 정보를 표시하는 GUI 애플리케이션입니다.

use crate::drivers::framebuffer::Color;
use crate::drivers::font;
use crate::gui::Window;
use alloc::string::String;
use alloc::vec::Vec;

/// 프로세스 정보
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u64,
    pub name: String,
    pub state: String,
    pub memory: usize,
}

/// 시스템 모니터
pub struct SystemMonitor {
    window: Window,
    cpu_usage: f32,
    memory_used: usize,
    memory_total: usize,
    processes: Vec<ProcessInfo>,
    scroll_offset: usize,
    update_counter: usize,
}

impl SystemMonitor {
    /// 새 시스템 모니터 생성
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        let window = Window::new(x, y, width, height, "System Monitor");

        SystemMonitor {
            window,
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            processes: Vec::new(),
            scroll_offset: 0,
            update_counter: 0,
        }
    }

    /// 시스템 정보 업데이트
    pub fn update(&mut self) {
        self.update_counter += 1;

        // CPU 사용률 업데이트 (실제로는 스케줄러에서 가져와야 함)
        // 여기서는 데모용 값 사용
        self.cpu_usage = 15.0 + (self.update_counter % 20) as f32 * 2.0;

        // 메모리 정보 업데이트 (실제로는 메모리 관리자에서 가져와야 함)
        self.memory_total = 512 * 1024 * 1024; // 512 MB
        self.memory_used = 128 * 1024 * 1024 + (self.update_counter % 50) * 1024 * 1024;

        // 프로세스 목록 업데이트 (실제로는 스케줄러에서 가져와야 함)
        if self.processes.is_empty() {
            self.processes.push(ProcessInfo {
                pid: 0,
                name: String::from("kernel"),
                state: String::from("Running"),
                memory: 16 * 1024 * 1024,
            });

            self.processes.push(ProcessInfo {
                pid: 1,
                name: String::from("init"),
                state: String::from("Running"),
                memory: 4 * 1024 * 1024,
            });

            self.processes.push(ProcessInfo {
                pid: 2,
                name: String::from("shell"),
                state: String::from("Ready"),
                memory: 8 * 1024 * 1024,
            });

            self.processes.push(ProcessInfo {
                pid: 3,
                name: String::from("gui_compositor"),
                state: String::from("Running"),
                memory: 32 * 1024 * 1024,
            });
        }
    }

    /// 시스템 모니터 렌더링
    pub fn render(&self) {
        // 윈도우 렌더링
        self.window.render();

        const TITLE_BAR_HEIGHT: usize = 24;
        const PADDING: usize = 10;
        const SECTION_SPACING: usize = 10;

        let content_x = self.window.x + PADDING;
        let mut current_y = self.window.y + TITLE_BAR_HEIGHT + PADDING;

        // CPU 사용률 섹션
        font::draw_str(content_x, current_y, "CPU Usage:", Color::new(0, 100, 200));
        current_y += font::CHAR_HEIGHT + 5;

        // CPU 사용률 바
        let bar_width = self.window.width - PADDING * 2;
        let bar_height = 20;
        
        // 배경
        crate::drivers::framebuffer::fill_rect(
            content_x,
            current_y,
            bar_width,
            bar_height,
            Color::LIGHT_GRAY,
        );

        // 사용률 바
        let used_width = ((bar_width as f32) * (self.cpu_usage / 100.0)) as usize;
        let color = if self.cpu_usage < 50.0 {
            Color::new(0, 200, 100)
        } else if self.cpu_usage < 80.0 {
            Color::new(255, 200, 0)
        } else {
            Color::new(255, 100, 100)
        };
        
        crate::drivers::framebuffer::fill_rect(
            content_x,
            current_y,
            used_width,
            bar_height,
            color,
        );

        // 테두리
        crate::drivers::framebuffer::draw_rect(
            content_x,
            current_y,
            bar_width,
            bar_height,
            Color::BLACK,
        );

        // 퍼센트 표시
        let mut cpu_text = String::new();
        use core::fmt::Write;
        let _ = write!(&mut cpu_text, "{:.1}%", self.cpu_usage);
        font::draw_str(
            content_x + bar_width / 2 - 20,
            current_y + 6,
            &cpu_text,
            Color::BLACK,
        );

        current_y += bar_height + SECTION_SPACING;

        // 메모리 사용량 섹션
        font::draw_str(content_x, current_y, "Memory Usage:", Color::new(0, 100, 200));
        current_y += font::CHAR_HEIGHT + 5;

        // 메모리 사용률 바
        crate::drivers::framebuffer::fill_rect(
            content_x,
            current_y,
            bar_width,
            bar_height,
            Color::LIGHT_GRAY,
        );

        let memory_percent = (self.memory_used as f32 / self.memory_total as f32) * 100.0;
        let mem_used_width = ((bar_width as f32) * (self.memory_used as f32 / self.memory_total as f32)) as usize;
        
        let mem_color = if memory_percent < 50.0 {
            Color::new(0, 150, 255)
        } else if memory_percent < 80.0 {
            Color::new(255, 200, 0)
        } else {
            Color::new(255, 100, 100)
        };

        crate::drivers::framebuffer::fill_rect(
            content_x,
            current_y,
            mem_used_width,
            bar_height,
            mem_color,
        );

        crate::drivers::framebuffer::draw_rect(
            content_x,
            current_y,
            bar_width,
            bar_height,
            Color::BLACK,
        );

        // 메모리 정보 텍스트
        let mut mem_text = String::new();
        let _ = write!(
            &mut mem_text,
            "{} MB / {} MB",
            self.memory_used / (1024 * 1024),
            self.memory_total / (1024 * 1024)
        );
        font::draw_str(
            content_x + 5,
            current_y + 6,
            &mem_text,
            Color::BLACK,
        );

        current_y += bar_height + SECTION_SPACING;

        // 프로세스 목록 섹션
        font::draw_str(content_x, current_y, "Processes:", Color::new(0, 100, 200));
        current_y += font::CHAR_HEIGHT + 5;

        // 프로세스 테이블 헤더
        let header_y = current_y;
        crate::drivers::framebuffer::fill_rect(
            content_x,
            header_y,
            bar_width,
            font::CHAR_HEIGHT + 4,
            Color::new(200, 200, 200),
        );

        font::draw_str(content_x + 5, header_y + 2, "PID", Color::BLACK);
        font::draw_str(content_x + 50, header_y + 2, "Name", Color::BLACK);
        font::draw_str(content_x + 200, header_y + 2, "State", Color::BLACK);
        font::draw_str(content_x + 300, header_y + 2, "Memory", Color::BLACK);

        current_y += font::CHAR_HEIGHT + 6;

        // 프로세스 목록
        let item_height = font::CHAR_HEIGHT + 4;
        let available_height = self.window.height - (current_y - self.window.y) - PADDING;
        let visible_items = available_height / item_height;

        for (i, proc_index) in (self.scroll_offset..self.processes.len()).enumerate() {
            if i >= visible_items {
                break;
            }

            let proc = &self.processes[proc_index];
            let item_y = current_y + i * item_height;

            // 배경 (짝수/홀수 구분)
            if i % 2 == 0 {
                crate::drivers::framebuffer::fill_rect(
                    content_x,
                    item_y,
                    bar_width,
                    item_height,
                    Color::new(245, 245, 245),
                );
            }

            // PID
            let mut pid_text = String::new();
            let _ = write!(&mut pid_text, "{}", proc.pid);
            font::draw_str(content_x + 5, item_y + 2, &pid_text, Color::BLACK);

            // Name
            font::draw_str(content_x + 50, item_y + 2, &proc.name, Color::BLACK);

            // State
            let state_color = match proc.state.as_str() {
                "Running" => Color::new(0, 150, 0),
                "Ready" => Color::new(0, 100, 200),
                "Blocked" => Color::new(200, 100, 0),
                _ => Color::GRAY,
            };
            font::draw_str(content_x + 200, item_y + 2, &proc.state, state_color);

            // Memory
            let mut mem_text = String::new();
            let _ = write!(&mut mem_text, "{} MB", proc.memory / (1024 * 1024));
            font::draw_str(content_x + 300, item_y + 2, &mem_text, Color::BLACK);
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

