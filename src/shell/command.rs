//! 명령어 처리 모듈
//!
//! 이 모듈은 Shell에서 실행할 수 있는 명령어들을 정의하고 처리합니다.

use crate::drivers::vga;
use crate::drivers::timer;
use alloc::string::String;

// VGA 출력 매크로 사용
use crate::vga_println;

/// 명령어 타입
pub enum Command {
    Help,
    Clear,
    Echo,
    Uptime,
    Exit,
}

impl Command {
    /// 명령어 이름으로 파싱
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "help" | "h" => Some(Command::Help),
            "clear" | "cls" => Some(Command::Clear),
            "echo" => Some(Command::Echo),
            "uptime" => Some(Command::Uptime),
            "exit" | "quit" => Some(Command::Exit),
            _ => None,
        }
    }

    /// 명령어 실행
    pub fn execute(&self, args: &[&str]) -> Result<(), String> {
        match self {
            Command::Help => self.cmd_help(),
            Command::Clear => self.cmd_clear(),
            Command::Echo => self.cmd_echo(args),
            Command::Uptime => self.cmd_uptime(),
            Command::Exit => self.cmd_exit(),
        }
    }

    /// help 명령어: 도움말 출력
    fn cmd_help(&self) -> Result<(), String> {
        vga_println!("Available commands:");
        vga_println!("  help              - Show this help message");
        vga_println!("  clear, cls        - Clear the screen");
        vga_println!("  echo <text>       - Print text to the screen");
        vga_println!("  uptime            - Show system uptime");
        vga_println!("  exit, quit        - Exit the shell (reboot simulation)");
        Ok(())
    }

    /// clear 명령어: 화면 지우기
    fn cmd_clear(&self) -> Result<(), String> {
        vga::WRITER.lock().clear_screen();
        Ok(())
    }

    /// echo 명령어: 텍스트 출력
    fn cmd_echo(&self, args: &[&str]) -> Result<(), String> {
        if args.is_empty() {
            vga_println!();
        } else {
            // 인자들을 공백으로 연결하여 출력
            let text = args.join(" ");
            vga_println!("{}", text);
        }
        Ok(())
    }

    /// uptime 명령어: 시스템 업타임 표시
    fn cmd_uptime(&self) -> Result<(), String> {
        let ms = timer::get_milliseconds();
        let seconds = ms / 1000;
        let minutes = seconds / 60;
        let hours = minutes / 60;
        
        let remaining_ms = ms % 1000;
        let remaining_seconds = seconds % 60;
        let remaining_minutes = minutes % 60;
        
        if hours > 0 {
            vga_println!(
                "Uptime: {}h {}m {}s ({} ms)",
                hours, remaining_minutes, remaining_seconds, ms
            );
        } else if minutes > 0 {
            vga_println!(
                "Uptime: {}m {}s ({} ms)",
                remaining_minutes, remaining_seconds, ms
            );
        } else {
            vga_println!("Uptime: {}s ({} ms)", remaining_seconds, ms);
        }
        
        Ok(())
    }

    /// exit 명령어: Shell 종료 (재부팅 시뮬레이션)
    fn cmd_exit(&self) -> Result<(), String> {
        vga_println!("Exiting shell...");
        vga_println!("(In a real system, this would reboot or return to kernel)");
        // 실제로는 무한 루프로 계속 실행되므로, 여기서는 단순히 메시지만 출력
        // 향후 프로세스 시스템이 구현되면 실제로 종료할 수 있음
        Ok(())
    }
}

