//! 데스크톱 환경
//!
//! 애플리케이션 런처, 태스크바, 데스크톱 관리 기능을 제공합니다.

use crate::drivers::framebuffer::{Color, get_width, get_height};
use crate::drivers::mouse::MouseEvent;
use crate::drivers::font;
use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;

/// 데스크톱 환경 전역 인스턴스
static DESKTOP: Mutex<Desktop> = Mutex::new(Desktop::new());

/// 애플리케이션 아이템
#[derive(Clone)]
pub struct AppItem {
    pub name: String,
    pub icon: AppIcon,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

/// 애플리케이션 아이콘 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppIcon {
    Calculator,
    TextEditor,
    FileManager,
    SystemMonitor,
    Terminal,
}

impl AppIcon {
    /// 아이콘 색상 가져오기
    pub fn color(&self) -> Color {
        match self {
            AppIcon::Calculator => Color::new(0, 150, 136),      // Teal
            AppIcon::TextEditor => Color::new(33, 150, 243),     // Blue
            AppIcon::FileManager => Color::new(255, 152, 0),     // Orange
            AppIcon::SystemMonitor => Color::new(76, 175, 80),   // Green
            AppIcon::Terminal => Color::new(96, 125, 139),       // Blue Gray
        }
    }

    /// 아이콘 심볼 가져오기
    pub fn symbol(&self) -> &'static str {
        match self {
            AppIcon::Calculator => "CALC",
            AppIcon::TextEditor => "EDIT",
            AppIcon::FileManager => "FILE",
            AppIcon::SystemMonitor => "SYSM",
            AppIcon::Terminal => "TERM",
        }
    }
}

/// 데스크톱 환경
pub struct Desktop {
    apps: Vec<AppItem>,
    taskbar_height: usize,
    show_launcher: bool,
    launcher_x: usize,
    launcher_y: usize,
    launcher_width: usize,
    launcher_height: usize,
}

impl Desktop {
    /// 새 데스크톱 생성
    const fn new() -> Self {
        Desktop {
            apps: Vec::new(),
            taskbar_height: 40,
            show_launcher: false,
            launcher_x: 0,
            launcher_y: 0,
            launcher_width: 0,
            launcher_height: 0,
        }
    }

    /// 데스크톱 초기화
    pub fn init(&mut self) {
        // 애플리케이션 목록 생성
        self.apps.clear();

        let grid_cols = 4;
        let grid_rows = 2;
        let icon_width = 100;
        let icon_height = 100;
        let spacing = 20;

        let start_x = 50;
        let start_y = 50;

        let apps_data = [
            ("Calculator", AppIcon::Calculator),
            ("Text Editor", AppIcon::TextEditor),
            ("File Manager", AppIcon::FileManager),
            ("System Monitor", AppIcon::SystemMonitor),
            ("Terminal", AppIcon::Terminal),
        ];

        for (i, (name, icon)) in apps_data.iter().enumerate() {
            let row = i / grid_cols;
            let col = i % grid_cols;

            let x = start_x + col * (icon_width + spacing);
            let y = start_y + row * (icon_height + spacing);

            self.apps.push(AppItem {
                name: String::from(*name),
                icon: *icon,
                x,
                y,
                width: icon_width,
                height: icon_height,
            });
        }

        // 런처 크기 계산
        self.launcher_width = grid_cols * (icon_width + spacing) + spacing + start_x * 2;
        self.launcher_height = grid_rows * (icon_height + spacing) + spacing + start_y * 2;
        
        let screen_width = get_width();
        let screen_height = get_height();
        
        self.launcher_x = (screen_width - self.launcher_width) / 2;
        self.launcher_y = (screen_height - self.launcher_height - self.taskbar_height) / 2;
    }

    /// 데스크톱 렌더링
    pub fn render(&self) {
        let screen_width = get_width();
        let screen_height = get_height();

        // 배경 그리기
        crate::drivers::framebuffer::fill_rect(
            0,
            0,
            screen_width,
            screen_height - self.taskbar_height,
            Color::new(30, 30, 30), // 어두운 회색 배경
        );

        // 런처가 열려 있으면 렌더링
        if self.show_launcher {
            self.render_launcher();
        }

        // 태스크바 렌더링
        self.render_taskbar();
    }

    /// 런처 렌더링
    fn render_launcher(&self) {
        // 반투명 배경 효과 시뮬레이션 (어두운 배경)
        crate::drivers::framebuffer::fill_rect(
            self.launcher_x,
            self.launcher_y,
            self.launcher_width,
            self.launcher_height,
            Color::new(50, 50, 50),
        );

        // 런처 테두리
        crate::drivers::framebuffer::draw_rect(
            self.launcher_x,
            self.launcher_y,
            self.launcher_width,
            self.launcher_height,
            Color::new(100, 100, 100),
        );

        // 타이틀
        font::draw_str(
            self.launcher_x + 20,
            self.launcher_y + 10,
            "Application Launcher",
            Color::WHITE,
        );

        // 애플리케이션 아이콘들 렌더링
        for app in &self.apps {
            self.render_app_icon(app);
        }
    }

    /// 애플리케이션 아이콘 렌더링
    fn render_app_icon(&self, app: &AppItem) {
        let abs_x = self.launcher_x + app.x;
        let abs_y = self.launcher_y + app.y;

        // 아이콘 배경
        crate::drivers::framebuffer::fill_rect(
            abs_x,
            abs_y,
            app.width,
            app.height,
            app.icon.color(),
        );

        // 아이콘 테두리
        crate::drivers::framebuffer::draw_rect(
            abs_x,
            abs_y,
            app.width,
            app.height,
            Color::new(200, 200, 200),
        );

        // 아이콘 심볼 (중앙에)
        let symbol = app.icon.symbol();
        let symbol_x = abs_x + (app.width - symbol.len() * 8) / 2;
        let symbol_y = abs_y + app.height / 2 - 16;
        font::draw_str(symbol_x, symbol_y, symbol, Color::WHITE);

        // 애플리케이션 이름 (아래에)
        let name_x = abs_x + (app.width - app.name.len() * 8) / 2;
        let name_y = abs_y + app.height / 2 + 8;
        font::draw_str(name_x, name_y, &app.name, Color::WHITE);
    }

    /// 태스크바 렌더링
    fn render_taskbar(&self) {
        let screen_width = get_width();
        let screen_height = get_height();

        // 태스크바 배경
        crate::drivers::framebuffer::fill_rect(
            0,
            screen_height - self.taskbar_height,
            screen_width,
            self.taskbar_height,
            Color::new(40, 40, 40),
        );

        // 태스크바 상단 테두리
        crate::drivers::framebuffer::draw_line(
            0,
            (screen_height - self.taskbar_height) as isize,
            screen_width as isize,
            (screen_height - self.taskbar_height) as isize,
            Color::new(80, 80, 80),
        );

        // 시작 버튼
        let start_btn_width = 120;
        let start_btn_height = 30;
        let start_btn_x = 10;
        let start_btn_y = screen_height - self.taskbar_height + 5;

        crate::drivers::framebuffer::fill_rect(
            start_btn_x,
            start_btn_y,
            start_btn_width,
            start_btn_height,
            if self.show_launcher {
                Color::new(0, 120, 215)
            } else {
                Color::new(60, 60, 60)
            },
        );

        crate::drivers::framebuffer::draw_rect(
            start_btn_x,
            start_btn_y,
            start_btn_width,
            start_btn_height,
            Color::new(100, 100, 100),
        );

        font::draw_str(
            start_btn_x + 10,
            start_btn_y + 11,
            "Applications",
            Color::WHITE,
        );

        // 시스템 트레이 (우측)
        let time_x = screen_width - 100;
        let time_y = screen_height - self.taskbar_height + 12;
        
        // 간단한 시간 표시 대신 시스템 정보
        let uptime = crate::drivers::timer::get_milliseconds() / 1000;
        let uptime_str = alloc::format!("{}s", uptime);
        font::draw_str(time_x, time_y, &uptime_str, Color::WHITE);
    }

    /// 런처 토글
    pub fn toggle_launcher(&mut self) {
        self.show_launcher = !self.show_launcher;
    }

    /// 마우스 이벤트 처리
    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<LauncherAction> {
        match event {
            MouseEvent::LeftButtonDown(x, y) => {
                let screen_height = get_height();

                // 태스크바의 시작 버튼 클릭 확인
                let start_btn_x = 10;
                let start_btn_y = screen_height - self.taskbar_height + 5;
                let start_btn_width = 120;
                let start_btn_height = 30;

                if x >= start_btn_x as isize
                    && x < (start_btn_x + start_btn_width) as isize
                    && y >= start_btn_y as isize
                    && y < (start_btn_y + start_btn_height) as isize
                {
                    self.toggle_launcher();
                    return Some(LauncherAction::ToggleLauncher);
                }

                // 런처가 열려 있을 때 앱 클릭 확인
                if self.show_launcher {
                    for app in &self.apps {
                        let abs_x = self.launcher_x + app.x;
                        let abs_y = self.launcher_y + app.y;

                        if x >= abs_x as isize
                            && x < (abs_x + app.width) as isize
                            && y >= abs_y as isize
                            && y < (abs_y + app.height) as isize
                        {
                            self.show_launcher = false;
                            return Some(LauncherAction::LaunchApp(app.icon));
                        }
                    }

                    // 런처 외부 클릭 시 닫기
                    if x < self.launcher_x as isize
                        || x >= (self.launcher_x + self.launcher_width) as isize
                        || y < self.launcher_y as isize
                        || y >= (self.launcher_y + self.launcher_height) as isize
                    {
                        self.show_launcher = false;
                        return Some(LauncherAction::ToggleLauncher);
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// 런처가 표시 중인지 확인
    pub fn is_launcher_visible(&self) -> bool {
        self.show_launcher
    }
}

/// 런처 액션
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherAction {
    ToggleLauncher,
    LaunchApp(AppIcon),
}

/// 데스크톱 초기화
pub fn init() {
    DESKTOP.lock().init();
}

/// 데스크톱 렌더링
pub fn render() {
    DESKTOP.lock().render();
}

/// 마우스 이벤트 처리
pub fn handle_mouse_event(event: MouseEvent) -> Option<LauncherAction> {
    DESKTOP.lock().handle_mouse_event(event)
}

/// 런처가 표시 중인지 확인
pub fn is_launcher_visible() -> bool {
    DESKTOP.lock().is_launcher_visible()
}

