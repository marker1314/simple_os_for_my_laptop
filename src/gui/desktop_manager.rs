//! 데스크톱 관리자
//!
//! 데스크톱 환경과 애플리케이션 윈도우를 통합 관리합니다.

use super::desktop::{self, LauncherAction, AppIcon};
use super::applications::*;
use crate::drivers::mouse::MouseEvent;
use spin::Mutex;
use alloc::vec::Vec;

/// 데스크톱 관리자 전역 인스턴스
static DESKTOP_MANAGER: Mutex<DesktopManager> = Mutex::new(DesktopManager::new());

/// 실행 중인 애플리케이션
/// 각 애플리케이션은 자체 Window를 내부에서 관리합니다.
enum RunningApp {
    Calculator(Calculator),
    TextEditor(TextEditor),
    FileManager(FileManager),
    SystemMonitor(SystemMonitor),
    Terminal(Terminal),
}

/// 데스크톱 관리자
pub struct DesktopManager {
    running_apps: Vec<RunningApp>,
    show_desktop: bool,
    next_app_offset: usize,
}

impl DesktopManager {
    /// 새 데스크톱 관리자 생성
    const fn new() -> Self {
        DesktopManager {
            running_apps: Vec::new(),
            show_desktop: true,
            next_app_offset: 0,
        }
    }

    /// 데스크톱 관리자 초기화
    pub fn init(&mut self) {
        desktop::init();
    }

    /// 애플리케이션 실행
    pub fn launch_app(&mut self, icon: AppIcon) {
        let offset = self.next_app_offset * 20;
        self.next_app_offset = (self.next_app_offset + 1) % 10;

        let app = match icon {
            AppIcon::Calculator => {
                RunningApp::Calculator(Calculator::new(100 + offset, 100 + offset))
            }
            AppIcon::TextEditor => {
                RunningApp::TextEditor(TextEditor::new(120 + offset, 120 + offset, 600, 400))
            }
            AppIcon::FileManager => {
                RunningApp::FileManager(FileManager::new(140 + offset, 140 + offset, 700, 500))
            }
            AppIcon::SystemMonitor => {
                RunningApp::SystemMonitor(SystemMonitor::new(160 + offset, 160 + offset, 500, 400))
            }
            AppIcon::Terminal => {
                RunningApp::Terminal(Terminal::new(180 + offset, 180 + offset, 600, 400))
            }
        };

        self.running_apps.push(app);
    }

    /// 마우스 이벤트 처리
    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        // 먼저 데스크톱(런처/태스크바) 이벤트 처리
        if let Some(action) = desktop::handle_mouse_event(event) {
            match action {
                LauncherAction::LaunchApp(icon) => {
                    self.launch_app(icon);
                }
                LauncherAction::ToggleLauncher => {
                    // 런처 토글은 이미 desktop 모듈에서 처리됨
                }
            }
        } else {
            // 런처가 보이지 않을 때만 애플리케이션으로 이벤트 전달
            if !desktop::is_launcher_visible() {
                for app in &mut self.running_apps {
                    match app {
                        RunningApp::Calculator(calc) => calc.handle_mouse_event(event),
                        RunningApp::FileManager(fm) => fm.handle_mouse_event(event),
                        // 다른 앱들은 아직 handle_mouse_event를 구현하지 않음
                        _ => {}
                    }
                }
            }
        }
    }

    /// 전체 화면 렌더링
    pub fn render(&self) {
        // 1. 배경과 데스크톱 요소 (태스크바, 런처)
        desktop::render();

        // 2. 런처가 숨겨져 있을 때만 애플리케이션 렌더링
        if !desktop::is_launcher_visible() {
            for app in &self.running_apps {
                match app {
                    RunningApp::Calculator(calc) => calc.render(),
                    RunningApp::TextEditor(editor) => editor.render(),
                    RunningApp::FileManager(fm) => fm.render(),
                    RunningApp::SystemMonitor(sm) => sm.render(),
                    RunningApp::Terminal(term) => term.render(),
                }
            }
        }
    }

    /// 데스크톱 표시 여부 설정
    pub fn set_show_desktop(&mut self, show: bool) {
        self.show_desktop = show;
    }
}

/// 데스크톱 관리자 초기화
pub fn init() {
    DESKTOP_MANAGER.lock().init();
}

/// 애플리케이션 실행
pub fn launch_app(icon: AppIcon) {
    DESKTOP_MANAGER.lock().launch_app(icon);
}

/// 마우스 이벤트 처리
pub fn handle_mouse_event(event: MouseEvent) {
    DESKTOP_MANAGER.lock().handle_mouse_event(event);
}

/// 전체 화면 렌더링
pub fn render() {
    DESKTOP_MANAGER.lock().render();
}

/// 데스크톱 표시 여부 설정
pub fn set_show_desktop(show: bool) {
    DESKTOP_MANAGER.lock().set_show_desktop(show);
}

