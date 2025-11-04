//! Shell 모듈
//!
//! 이 모듈은 커널의 기본 사용자 인터페이스를 제공합니다.
//! 명령어를 파싱하고 실행하는 간단한 Shell을 구현합니다.

mod command;

use crate::drivers::keyboard;
use crate::drivers::vga;
use alloc::vec::Vec;
use alloc::string::String;

// VGA 출력 매크로 사용
use crate::vga_print;
use crate::vga_println;

pub use command::Command;

/// Shell 구조체
pub struct Shell {
    /// 명령어 히스토리 (향후 구현)
    history: Vec<String>,
    /// 현재 입력 버퍼
    input_buffer: String,
}

impl Shell {
    /// 새로운 Shell 생성
    pub fn new() -> Self {
        Shell {
            history: Vec::new(),
            input_buffer: String::new(),
        }
    }

    /// Shell 실행 (메인 루프)
    ///
    /// 이 함수는 무한 루프로 실행되며 사용자 입력을 받아 명령어를 실행합니다.
    pub fn run(&mut self) -> ! {
        // 시작 메시지 출력
        vga::WRITER.lock().clear_screen();
        vga_println!("Simple OS Shell");
        vga_println!("===============");
        vga_println!("Type 'help' for available commands.");
        vga_println!();

        loop {
            // 프롬프트 출력
            self.print_prompt();

            // 명령어 입력 받기
            let command = self.read_command();

            // 명령어 실행
            if !command.is_empty() {
                self.execute_command(&command);
            }
        }
    }

    /// 프롬프트 출력
    fn print_prompt(&self) {
        use core::fmt::Write;
        let mut writer = vga::WRITER.lock();
        writer.set_color(vga::Color::LightGreen, vga::Color::Black);
        let _ = writer.write_str("user@simple-os");
        writer.set_color(vga::Color::LightGray, vga::Color::Black);
        let _ = writer.write_str("$ ");
    }

    /// 명령어 읽기
    ///
    /// 사용자가 Enter를 누를 때까지 입력을 받습니다.
    fn read_command(&mut self) -> String {
        self.input_buffer.clear();

        loop {
            // CPU 대기 (전력 절약)
            x86_64::instructions::hlt();

            // 키보드 입력 확인
            if let Some(ch) = keyboard::read_char() {
                match ch {
                    '\n' => {
                        // Enter 키: 명령어 완료
                        vga_println!();
                        let command = self.input_buffer.clone();
                        self.input_buffer.clear();
                        return command;
                    }
                    '\x08' => {
                        // Backspace: 마지막 문자 삭제
                        if !self.input_buffer.is_empty() {
                            self.input_buffer.pop();
                            // VGA에서 마지막 문자 지우기
                            let mut writer = vga::WRITER.lock();
                            writer.write_byte(b'\x08'); // 커서를 뒤로 이동
                            writer.write_byte(b' ');    // 공백으로 덮어쓰기
                            writer.write_byte(b'\x08'); // 다시 커서를 뒤로 이동
                        }
                    }
                    '\t' => {
                        // Tab: 공백 4개로 변환 (간단한 구현)
                        for _ in 0..4 {
                            self.input_buffer.push(' ');
                            vga_print!(" ");
                        }
                    }
                    _ => {
                        // 일반 문자: 버퍼에 추가하고 화면에 출력
                        self.input_buffer.push(ch);
                        vga_print!("{}", ch);
                    }
                }
            }
        }
    }

    /// 명령어 실행
    fn execute_command(&mut self, input: &str) {
        // 명령어 파싱
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        
        if parts.is_empty() {
            return;
        }

        let command_name = parts[0];
        let args = &parts[1..];

        // 명령어 실행
        match Command::parse(command_name) {
            Some(cmd) => {
                match cmd.execute(args) {
                    Ok(_) => {}
                    Err(e) => {
                        vga_println!("Error: {}", e);
                    }
                }
            }
            None => {
                vga_println!("Unknown command: '{}'", command_name);
                vga_println!("Type 'help' for available commands.");
            }
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

