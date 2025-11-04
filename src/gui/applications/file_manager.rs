//! 파일 관리자 애플리케이션
//!
//! 파일 시스템을 탐색하고 파일을 관리하는 GUI 애플리케이션입니다.

use crate::drivers::framebuffer::Color;
use crate::drivers::font;
use crate::drivers::mouse::MouseEvent;
use crate::gui::widget::{Button, Widget};
use crate::gui::Window;
use alloc::string::String;
use alloc::vec::Vec;

/// 파일 항목 타입
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileEntryType {
    Directory,
    File,
}

/// 파일 항목
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub entry_type: FileEntryType,
    pub size: usize,
}

/// 파일 관리자
pub struct FileManager {
    window: Window,
    current_path: String,
    entries: Vec<FileEntry>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    buttons: Vec<Button>,
}

impl FileManager {
    /// 새 파일 관리자 생성
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        let window = Window::new(x, y, width, height, "File Manager");
        let mut buttons = Vec::new();

        // 버튼 생성
        let button_y = y + 28;
        buttons.push(Button::new(x + 10, button_y, 60, 30, "Up"));
        buttons.push(Button::new(x + 80, button_y, 80, 30, "Refresh"));
        buttons.push(Button::new(x + 170, button_y, 80, 30, "New Dir"));

        FileManager {
            window,
            current_path: String::from("/"),
            entries: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            buttons,
        }
    }

    /// 디렉토리 로드 (VFS를 통해)
    pub fn load_directory(&mut self, path: &str) {
        self.current_path = String::from(path);
        self.entries.clear();
        self.selected_index = None;
        self.scroll_offset = 0;

        // 실제로는 VFS를 통해 디렉토리 내용을 로드해야 함
        // 여기서는 데모 데이터 사용
        self.entries.push(FileEntry {
            name: String::from(".."),
            entry_type: FileEntryType::Directory,
            size: 0,
        });

        self.entries.push(FileEntry {
            name: String::from("documents"),
            entry_type: FileEntryType::Directory,
            size: 0,
        });

        self.entries.push(FileEntry {
            name: String::from("pictures"),
            entry_type: FileEntryType::Directory,
            size: 0,
        });

        self.entries.push(FileEntry {
            name: String::from("readme.txt"),
            entry_type: FileEntryType::File,
            size: 1024,
        });

        self.entries.push(FileEntry {
            name: String::from("system.log"),
            entry_type: FileEntryType::File,
            size: 4096,
        });

        self.update_title();
    }

    /// 파일 관리자 렌더링
    pub fn render(&self) {
        // 윈도우 렌더링
        self.window.render();

        const TITLE_BAR_HEIGHT: usize = 24;
        const TOOLBAR_HEIGHT: usize = 40;
        const PADDING: usize = 10;

        // 툴바 배경
        crate::drivers::framebuffer::fill_rect(
            self.window.x,
            self.window.y + TITLE_BAR_HEIGHT,
            self.window.width,
            TOOLBAR_HEIGHT,
            Color::new(220, 220, 220),
        );

        // 버튼 렌더링
        for button in &self.buttons {
            button.render();
        }

        // 파일 목록 영역
        let list_y = self.window.y + TITLE_BAR_HEIGHT + TOOLBAR_HEIGHT;
        let list_height = self.window.height - TITLE_BAR_HEIGHT - TOOLBAR_HEIGHT - 30; // 상태바 제외

        // 현재 경로 표시
        let path_y = list_y + 5;
        font::draw_str(
            self.window.x + PADDING,
            path_y,
            &self.current_path,
            Color::BLACK,
        );

        // 파일 목록
        let item_height = font::CHAR_HEIGHT + 4;
        let visible_items = (list_height - 30) / item_height;

        for (i, entry_index) in (self.scroll_offset..self.entries.len()).enumerate() {
            if i >= visible_items {
                break;
            }

            let entry = &self.entries[entry_index];
            let item_y = list_y + 25 + i * item_height;

            // 선택된 항목 강조
            if Some(entry_index) == self.selected_index {
                crate::drivers::framebuffer::fill_rect(
                    self.window.x + PADDING,
                    item_y - 2,
                    self.window.width - PADDING * 2,
                    item_height,
                    Color::new(200, 220, 255),
                );
            }

            // 아이콘 (디렉토리 또는 파일)
            let icon = if entry.entry_type == FileEntryType::Directory {
                "[D]"
            } else {
                "[F]"
            };

            font::draw_str(
                self.window.x + PADDING,
                item_y,
                icon,
                if entry.entry_type == FileEntryType::Directory {
                    Color::new(0, 100, 200)
                } else {
                    Color::BLACK
                },
            );

            // 파일명
            let name_x = self.window.x + PADDING + 30;
            font::draw_str(name_x, item_y, &entry.name, Color::BLACK);

            // 크기 (파일만)
            if entry.entry_type == FileEntryType::File {
                let mut size_str = String::new();
                use core::fmt::Write;
                
                if entry.size < 1024 {
                    let _ = write!(&mut size_str, "{} B", entry.size);
                } else if entry.size < 1024 * 1024 {
                    let _ = write!(&mut size_str, "{} KB", entry.size / 1024);
                } else {
                    let _ = write!(&mut size_str, "{} MB", entry.size / (1024 * 1024));
                }

                let size_x = self.window.x + self.window.width - 100;
                font::draw_str(size_x, item_y, &size_str, Color::GRAY);
            }
        }

        // 상태 표시줄
        let status_y = self.window.y + self.window.height - 25;
        crate::drivers::framebuffer::fill_rect(
            self.window.x,
            status_y,
            self.window.width,
            25,
            Color::LIGHT_GRAY,
        );

        // 상태 정보
        let mut status = String::new();
        use core::fmt::Write;
        let _ = write!(&mut status, "{} items", self.entries.len());
        if let Some(index) = self.selected_index {
            if index < self.entries.len() {
                let _ = write!(&mut status, " | Selected: {}", self.entries[index].name);
            }
        }

        font::draw_str(
            self.window.x + 8,
            status_y + 8,
            &status,
            Color::BLACK,
        );
    }

    /// 마우스 이벤트 처리
    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> bool {
        match event {
            MouseEvent::LeftButtonDown(x, y) => {
                // 버튼 클릭 확인
                for (i, button) in self.buttons.iter_mut().enumerate() {
                    if button.contains_point(x, y) {
                        button.is_pressed = true;
                        self.handle_button_click(i);
                        return true;
                    }
                }

                // 파일 목록 클릭 확인
                const TITLE_BAR_HEIGHT: usize = 24;
                const TOOLBAR_HEIGHT: usize = 40;
                const PADDING: usize = 10;
                
                let list_y = self.window.y + TITLE_BAR_HEIGHT + TOOLBAR_HEIGHT + 25;
                let item_height = font::CHAR_HEIGHT + 4;

                if x >= (self.window.x + PADDING) as isize
                    && x < (self.window.x + self.window.width - PADDING) as isize
                    && y >= list_y as isize
                {
                    let relative_y = (y as usize).saturating_sub(list_y);
                    let clicked_index = relative_y / item_height + self.scroll_offset;

                    if clicked_index < self.entries.len() {
                        self.selected_index = Some(clicked_index);
                        return true;
                    }
                }
            }
            MouseEvent::LeftButtonUp(_, _) => {
                for button in &mut self.buttons {
                    button.is_pressed = false;
                }
            }
            // Note: 더블클릭은 현재 MouseEvent에 없으므로 생략
            // 향후 마우스 드라이버 업데이트 시 추가 가능
            _ => {}
        }
        false
    }

    /// 버튼 클릭 처리
    fn handle_button_click(&mut self, button_index: usize) {
        match button_index {
            0 => {
                // Up: 상위 디렉토리
                self.navigate_to("..");
            }
            1 => {
                // Refresh
                let path = self.current_path.clone();
                self.load_directory(&path);
            }
            2 => {
                // New Dir (구현 예정)
            }
            _ => {}
        }
    }

    /// 디렉토리 이동
    fn navigate_to(&mut self, name: &str) {
        if name == ".." {
            // 상위 디렉토리로 이동
            if self.current_path != "/" {
                // 경로에서 마지막 부분 제거
                if let Some(pos) = self.current_path.rfind('/') {
                    if pos == 0 {
                        self.current_path = String::from("/");
                    } else {
                        self.current_path.truncate(pos);
                    }
                }
            }
        } else {
            // 하위 디렉토리로 이동
            if !self.current_path.ends_with('/') {
                self.current_path.push('/');
            }
            self.current_path.push_str(name);
        }

        self.load_directory(&self.current_path.clone());
    }

    /// 타이틀 업데이트
    fn update_title(&mut self) {
        let mut title = String::from("File Manager - ");
        title.push_str(&self.current_path);
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
        let dx = x as isize - self.window.x as isize;
        let dy = y as isize - self.window.y as isize;

        self.window.move_to(x, y);

        // 버튼 위치도 함께 이동
        for button in &mut self.buttons {
            button.x = (button.x as isize + dx) as usize;
            button.y = (button.y as isize + dy) as usize;
        }
    }

    /// 타이틀 바 클릭 확인
    pub fn is_in_title_bar(&self, x: isize, y: isize) -> bool {
        self.window.is_in_title_bar(x, y)
    }

    /// 선택된 파일 항목 가져오기
    pub fn get_selected_entry(&self) -> Option<&FileEntry> {
        self.selected_index.and_then(|idx| self.entries.get(idx))
    }
}

