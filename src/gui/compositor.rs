//! 윈도우 컴포지터
//!
//! 여러 윈도우를 관리하고 렌더링하는 컴포지터입니다.

use super::window::Window;
use crate::drivers::mouse::MouseEvent;
use alloc::vec::Vec;
use alloc::vec;
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

/// 컴포지터 전역 인스턴스
static COMPOSITOR: Mutex<Compositor> = Mutex::new(Compositor::new());
static NEEDS_REDRAW: AtomicBool = AtomicBool::new(true);

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

    /// 모든 윈도우 렌더링 (이벤트 기반)
    /// NEEDS_REDRAW가 true일 때만 렌더링 (busy loop 방지)
    /// Occluded/minimized 윈도우는 full repaint 스킵
    pub fn render_all(&mut self) {
        // Redraw가 필요할 때만 렌더링 (이벤트 기반)
        if NEEDS_REDRAW.load(Ordering::Acquire) {
            // Occlusion 계산: 각 윈도우가 다른 윈도우에 가려졌는지 확인
            let mut occluded_windows = vec![false; self.windows.len()];
            
            // Z-order 역순으로 occlusion 계산 (위에 있는 윈도우가 아래를 가림)
            for i in (0..self.windows.len()).rev() {
                if self.windows[i].is_minimized || !self.windows[i].is_visible {
                    occluded_windows[i] = true;
                    continue;
                }
                
                // 이 윈도우보다 위에 있는 윈도우가 이 윈도우를 가리는지 확인
                for j in (i+1..self.windows.len()).rev() {
                    if self.windows[j].is_minimized || !self.windows[j].is_visible {
                        continue;
                    }
                    
                    // 간단한 overlap 체크
                    let w1 = &self.windows[i];
                    let w2 = &self.windows[j];
                    if !(w2.x + w2.width <= w1.x || w1.x + w1.width <= w2.x ||
                         w2.y + w2.height <= w1.y || w1.y + w1.height <= w2.y) {
                        // Overlap 발견 - i번 윈도우가 가려짐
                        occluded_windows[i] = true;
                        break;
                    }
                }
            }
            
            // Occlusion 상태 업데이트 및 렌더링
            for (i, window) in self.windows.iter_mut().enumerate() {
                // Occlusion 상태 업데이트
                window.set_occluded(occluded_windows[i]);
                
                if !window.is_visible || window.is_minimized || occluded_windows[i] {
                    continue; // 렌더링 스킵
                }
                
                // 각 윈도우가 needs_redraw 플래그를 가지고 있으면 그것도 확인
                if !window.needs_redraw && !NEEDS_REDRAW.load(Ordering::Acquire) {
                    continue; // 이 윈도우는 redraw 불필요
                }
                
                window.render();
                window.set_needs_redraw(false); // 렌더링 완료
            }
            
            NEEDS_REDRAW.store(false, Ordering::Release);
        }
    }
    
    /// 강제로 모든 윈도우 렌더링 (NEEDS_REDRAW 체크 없이)
    pub fn force_render_all(&self) {
        for window in &self.windows {
            window.render();
        }
        NEEDS_REDRAW.store(false, Ordering::Release);
    }

    /// 마우스 이벤트 처리
    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::LeftButtonDown(x, y) => {
                // 클릭된 윈도우 찾기 (역순으로 검색, 위에 있는 윈도우 우선)
                for i in (0..self.windows.len()).rev() {
                    // 불변/가변 분리를 위해 split_at_mut 사용
                    let (left, right) = self.windows.split_at_mut(i);
                    let window = &mut right[0];
                    if window.contains_point(x, y) {
                        // 포커스 설정: 왼쪽은 불변 순회, 선택 창만 가변 접근
                        for (j, w) in left.iter_mut().enumerate() {
                            w.set_focus(false);
                        }
                        window.set_focus(true);

                        // 타이틀 바 클릭 시 드래그 시작
                        if window.is_in_title_bar(x, y) {
                            self.dragging_window = Some(i);
                            self.drag_offset_x = x - window.x as isize;
                            self.drag_offset_y = y - window.y as isize;
                        }
                        break;
                    }
                }
                NEEDS_REDRAW.store(true, Ordering::Release);
            }
            MouseEvent::LeftButtonUp(_, _) => {
                self.dragging_window = None;
                NEEDS_REDRAW.store(true, Ordering::Release);
            }
            MouseEvent::Move(x, y) => {
                if let Some(window_index) = self.dragging_window {
                    if let Some(window) = self.windows.get_mut(window_index) {
                        let new_x = (x - self.drag_offset_x).max(0) as usize;
                        let new_y = (y - self.drag_offset_y).max(0) as usize;
                        window.move_to(new_x, new_y);
                        NEEDS_REDRAW.store(true, Ordering::Release);
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

/// 외부에서 리드로우 요청
pub fn request_redraw() { NEEDS_REDRAW.store(true, Ordering::Release); }

/// 리드로우 필요 여부
pub fn needs_redraw() -> bool { NEEDS_REDRAW.load(Ordering::Acquire) }

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

