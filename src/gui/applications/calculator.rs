//! 계산기 애플리케이션
//!
//! 기본적인 사칙연산을 수행하는 GUI 계산기입니다.

use crate::drivers::framebuffer::Color;
use crate::drivers::font;
use crate::drivers::mouse::MouseEvent;
use crate::gui::widget::{Button, Widget};
use crate::gui::Window;
use alloc::string::String;
use alloc::vec::Vec;

/// 계산기 상태
#[derive(Debug, Clone, Copy, PartialEq)]
enum CalculatorState {
    EnteringFirstNumber,
    EnteringSecondNumber,
    ShowingResult,
}

/// 연산자
#[derive(Debug, Clone, Copy, PartialEq)]
enum Operator {
    None,
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// 계산기 애플리케이션
pub struct Calculator {
    window: Window,
    buttons: Vec<Button>,
    display: String,
    first_number: f64,
    second_number: f64,
    operator: Operator,
    state: CalculatorState,
}

impl Calculator {
    /// 새 계산기 생성
    pub fn new(x: usize, y: usize) -> Self {
        let window = Window::new(x, y, 280, 400, "Calculator");
        let mut buttons = Vec::new();

        // 버튼 레이아웃 정의
        let button_width = 60;
        let button_height = 50;
        let spacing = 10;
        let start_x = x + 10;
        let start_y = y + 80; // 타이틀 바(24) + 디스플레이(56)

        // 숫자 버튼 (0-9)
        let button_labels = [
            ["7", "8", "9", "/"],
            ["4", "5", "6", "*"],
            ["1", "2", "3", "-"],
            ["C", "0", "=", "+"],
        ];

        for (row, labels) in button_labels.iter().enumerate() {
            for (col, label) in labels.iter().enumerate() {
                let btn_x = start_x + col * (button_width + spacing);
                let btn_y = start_y + row * (button_height + spacing);
                buttons.push(Button::new(btn_x, btn_y, button_width, button_height, label));
            }
        }

        Calculator {
            window,
            buttons,
            display: String::from("0"),
            first_number: 0.0,
            second_number: 0.0,
            operator: Operator::None,
            state: CalculatorState::EnteringFirstNumber,
        }
    }

    /// 계산기 렌더링
    pub fn render(&self) {
        // 윈도우 렌더링
        self.window.render();

        // 디스플레이 영역
        let display_x = self.window.x + 10;
        let display_y = self.window.y + 34; // 타이틀 바 아래
        let display_width = self.window.width - 20;
        let display_height = 40;

        // 디스플레이 배경
        crate::drivers::framebuffer::fill_rect(
            display_x,
            display_y,
            display_width,
            display_height,
            Color::WHITE,
        );

        // 디스플레이 테두리
        crate::drivers::framebuffer::draw_rect(
            display_x,
            display_y,
            display_width,
            display_height,
            Color::BLACK,
        );

        // 디스플레이 텍스트 (오른쪽 정렬)
        let text_width = self.display.len() * font::CHAR_WIDTH;
        let text_x = display_x + display_width - text_width - 10;
        let text_y = display_y + (display_height - font::CHAR_HEIGHT) / 2;
        font::draw_str(text_x, text_y, &self.display, Color::BLACK);

        // 버튼 렌더링
        for button in &self.buttons {
            button.render();
        }
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
            }
            MouseEvent::LeftButtonUp(_, _) => {
                // 모든 버튼 해제
                for button in &mut self.buttons {
                    button.is_pressed = false;
                }
            }
            _ => {}
        }
        false
    }

    /// 버튼 클릭 처리
    fn handle_button_click(&mut self, button_index: usize) {
        let button_labels = ["7", "8", "9", "/", "4", "5", "6", "*", "1", "2", "3", "-", "C", "0", "=", "+"];
        
        if button_index >= button_labels.len() {
            return;
        }

        let label = button_labels[button_index];

        match label {
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                self.handle_number(label);
            }
            "+" => self.handle_operator(Operator::Add),
            "-" => self.handle_operator(Operator::Subtract),
            "*" => self.handle_operator(Operator::Multiply),
            "/" => self.handle_operator(Operator::Divide),
            "=" => self.calculate(),
            "C" => self.clear(),
            _ => {}
        }
    }

    /// 숫자 입력 처리
    fn handle_number(&mut self, digit: &str) {
        match self.state {
            CalculatorState::EnteringFirstNumber => {
                if self.display == "0" || self.display == "Error" {
                    self.display = String::from(digit);
                } else if self.display.len() < 15 {
                    self.display.push_str(digit);
                }
            }
            CalculatorState::EnteringSecondNumber => {
                if self.display.len() < 15 {
                    self.display.push_str(digit);
                }
            }
            CalculatorState::ShowingResult => {
                self.display = String::from(digit);
                self.state = CalculatorState::EnteringFirstNumber;
                self.operator = Operator::None;
            }
        }
    }

    /// 연산자 입력 처리
    fn handle_operator(&mut self, op: Operator) {
        match self.state {
            CalculatorState::EnteringFirstNumber => {
                if let Ok(num) = self.display.parse::<f64>() {
                    self.first_number = num;
                    self.operator = op;
                    self.display.clear();
                    self.state = CalculatorState::EnteringSecondNumber;
                }
            }
            CalculatorState::EnteringSecondNumber => {
                // 연속 계산: 먼저 현재 결과를 계산
                self.calculate();
                self.operator = op;
                self.display.clear();
                self.state = CalculatorState::EnteringSecondNumber;
            }
            CalculatorState::ShowingResult => {
                self.operator = op;
                self.display.clear();
                self.state = CalculatorState::EnteringSecondNumber;
            }
        }
    }

    /// 계산 수행
    fn calculate(&mut self) {
        if self.state != CalculatorState::EnteringSecondNumber {
            return;
        }

        if let Ok(num) = self.display.parse::<f64>() {
            self.second_number = num;

            let result = match self.operator {
                Operator::Add => self.first_number + self.second_number,
                Operator::Subtract => self.first_number - self.second_number,
                Operator::Multiply => self.first_number * self.second_number,
                Operator::Divide => {
                    if self.second_number != 0.0 {
                        self.first_number / self.second_number
                    } else {
                        self.display = String::from("Error");
                        self.state = CalculatorState::ShowingResult;
                        return;
                    }
                }
                Operator::None => self.second_number,
            };

            // 결과 포맷팅 (소수점 처리)
            // fract()가 no_std에서 사용 불가하므로 간단한 검사 사용
            let is_integer = (result - (result as i64 as f64)).abs() < 0.0000001;
            if is_integer && result.abs() < 1e10 {
                // 정수 결과
                use core::fmt::Write;
                self.display.clear();
                let _ = write!(&mut self.display, "{}", result as i64);
            } else {
                // 부동소수점 결과 (간단한 표현)
                use core::fmt::Write;
                self.display.clear();
                let _ = write!(&mut self.display, "{}", result);
            }

            // 결과를 첫 번째 숫자로 설정 (연속 계산용)
            self.first_number = result;
            self.state = CalculatorState::ShowingResult;
        }
    }

    /// 초기화
    fn clear(&mut self) {
        self.display = String::from("0");
        self.first_number = 0.0;
        self.second_number = 0.0;
        self.operator = Operator::None;
        self.state = CalculatorState::EnteringFirstNumber;
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
}

