//! 윈도우 컴포지터
//!
//! 여러 윈도우를 관리하고 렌더링하는 컴포지터입니다.

use super::window::Window;
use crate::drivers::mouse::MouseEvent;
use alloc::vec::Vec;
use spin::Mutex;

/// 컴포지터 전역 인스턴스
static COMPOSITOR: Mutex<Compositor> = Mutex::new(Compositor::new());

/// 윈도우 컴포지터
pub struct Compositor {
    windows: Vec<Window>,
    dragging_window: Option<usize>,
    drag_offset_x: isize,
    drag_offset_y: isize,
}

impl Compositor {
    /// 새 컴포지터 생성
    const fn new() -> Self {
        Compositor {
            windows: Vec::new(),
            dragging_window: None,
            drag_offset_x: 0,
            drag_offset_y: 0,
        }
    }

    /// 윈도우 추가
    pub fn add_window(&mut self, window: Window) -> usize {
        self.windows.push(window);
        self.windows.len() - 1
    }

    /// 윈도우 제거
    pub fn remove_window(&mut self, index: usize) {
        if index < self.windows.len() {
            self.windows.remove(index);
        }
    }

    /// 모든 윈도우 렌더링
    pub fn render_all(&self) {
        for window in &self.windows {
            window.render();
        }
    }

    /// 마우스 이벤트 처리
    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::LeftButtonDown(x, y) => {
                // 클릭된 윈도우 찾기 (역순으로 검색, 위에 있는 윈도우 우선)
                for (i, window) in self.windows.iter_mut().enumerate().rev() {
                    if window.contains_point(x, y) {
                        // 포커스 설정
                        for (j, w) in self.windows.iter_mut().enumerate() {
                            w.set_focus(i == j);
                        }

                        // 타이틀 바 클릭 시 드래그 시작
                        if window.is_in_title_bar(x, y) {
                            self.dragging_window = Some(i);
                            self.drag_offset_x = x - window.x as isize;
                            self.drag_offset_y = y - window.y as isize;
                        }
                        break;
                    }
                }
            }
            MouseEvent::LeftButtonUp(_, _) => {
                self.dragging_window = None;
            }
            MouseEvent::Move(x, y) => {
                if let Some(window_index) = self.dragging_window {
                    if let Some(window) = self.windows.get_mut(window_index) {
                        let new_x = (x - self.drag_offset_x).max(0) as usize;
                        let new_y = (y - self.drag_offset_y).max(0) as usize;
                        window.move_to(new_x, new_y);
                    }
                }
            }
            _ => {}
        }
    }

    /// 윈도우 가져오기
    pub fn get_window(&self, index: usize) -> Option<&Window> {
        self.windows.get(index)
    }

    /// 윈도우 가져오기 (mutable)
    pub fn get_window_mut(&mut self, index: usize) -> Option<&mut Window> {
        self.windows.get_mut(index)
    }

    /// 윈도우 개수
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}

/// 전역 컴포지터에 윈도우 추가
pub fn add_window(window: Window) -> usize {
    COMPOSITOR.lock().add_window(window)
}

/// 전역 컴포지터에서 윈도우 제거
pub fn remove_window(index: usize) {
    COMPOSITOR.lock().remove_window(index)
}

/// 모든 윈도우 렌더링
pub fn render_all() {
    COMPOSITOR.lock().render_all()
}

/// 마우스 이벤트 처리
pub fn handle_mouse_event(event: MouseEvent) {
    COMPOSITOR.lock().handle_mouse_event(event)
}

/// 윈도우 가져오기
pub fn with_window<F, R>(index: usize, f: F) -> Option<R>
where
    F: FnOnce(&Window) -> R,
{
    let compositor = COMPOSITOR.lock();
    compositor.get_window(index).map(f)
}

/// 윈도우 가져오기 (mutable)
pub fn with_window_mut<F, R>(index: usize, f: F) -> Option<R>
where
    F: FnOnce(&mut Window) -> R,
{
    let mut compositor = COMPOSITOR.lock();
    compositor.get_window_mut(index).map(f)
}

