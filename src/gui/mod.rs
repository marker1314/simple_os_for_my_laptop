//! GUI 시스템 모듈
//!
//! 이 모듈은 그래픽 사용자 인터페이스 기능을 제공합니다.

pub mod window;
pub mod widget;
pub mod compositor;
pub mod applications;
pub mod desktop;
pub mod desktop_manager;

pub use window::Window;
pub use widget::{Widget, Button, TextBox};
pub use compositor::Compositor;
pub use applications::*;
pub use desktop::{AppIcon, LauncherAction};

use crate::drivers::framebuffer::Color;

/// GUI 시스템 초기화
pub fn init() -> Result<(), &'static str> {
    if !crate::drivers::framebuffer::is_initialized() {
        return Err("Framebuffer not initialized");
    }
    
    // 화면 지우기
    crate::drivers::framebuffer::clear(Color::new(30, 30, 30));
    
    // 데스크톱 환경 초기화
    desktop_manager::init();
    
    Ok(())
}

