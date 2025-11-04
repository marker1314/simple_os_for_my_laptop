//! 명령어 처리 모듈
//!
//! 이 모듈은 Shell에서 실행할 수 있는 명령어들을 정의하고 처리합니다.

use crate::drivers::vga;
use crate::drivers::timer;
use crate::drivers::ata::BlockDevice;
use alloc::string::String;
use alloc::format;

// VGA 출력 매크로 사용
use crate::vga_println;

/// 명령어 타입
pub enum Command {
    Help,
    Clear,
    Echo,
    Uptime,
    Exit,
    Disk,     // 디스크 정보 표시
    Read,     // 섹터 읽기 (테스트용)
    Write,    // 섹터 쓰기 (테스트용)
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
            "disk" => Some(Command::Disk),
            "read" => Some(Command::Read),
            "write" => Some(Command::Write),
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
            Command::Disk => self.cmd_disk(),
            Command::Read => self.cmd_read(args),
            Command::Write => self.cmd_write(args),
        }
    }

    /// help 명령어: 도움말 출력
    fn cmd_help(&self) -> Result<(), String> {
        vga_println!("Available commands:");
        vga_println!("  help              - Show this help message");
        vga_println!("  clear, cls        - Clear the screen");
        vga_println!("  echo <text>       - Print text to the screen");
        vga_println!("  uptime            - Show system uptime");
        vga_println!("  disk              - Show disk information");
        vga_println!("  read <sector>     - Read a sector from disk (test)");
        vga_println!("  write <sector>    - Write test data to sector (test)");
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

    /// disk 명령어: 디스크 정보 표시
    fn cmd_disk(&self) -> Result<(), String> {
        use crate::drivers::ata;
        
        let disk = ata::PRIMARY_MASTER.lock();
        
        if let Some(ref driver) = *disk {
            let num_sectors = driver.num_blocks();
            let size_mb = (num_sectors * 512) / (1024 * 1024);
            let size_gb = size_mb / 1024;
            
            vga_println!("Primary Master Disk:");
            vga_println!("  Total Sectors: {}", num_sectors);
            vga_println!("  Size: {} MB ({} GB)", size_mb, size_gb);
            vga_println!("  Sector Size: {} bytes", driver.block_size());
        } else {
            vga_println!("No disk found or not initialized");
        }
        
        Ok(())
    }

    /// read 명령어: 섹터 읽기
    fn cmd_read(&self, args: &[&str]) -> Result<(), String> {
        use crate::drivers::ata::{PRIMARY_MASTER, BlockDevice};
        
        if args.is_empty() {
            return Err(String::from("Usage: read <sector_number>"));
        }
        
        let sector: u64 = args[0].parse()
            .map_err(|_| String::from("Invalid sector number"))?;
        
        let mut disk = PRIMARY_MASTER.lock();
        
        if let Some(ref mut driver) = *disk {
            let mut buffer = [0u8; 512];
            
            match driver.read_block(sector, &mut buffer) {
                Ok(_) => {
                    vga_println!("Read sector {} successfully:", sector);
                    vga_println!();
                    
                    // 처음 256바이트만 16진수로 출력 (16바이트씩 16줄)
                    for line in 0..16 {
                        let offset = line * 16;
                        crate::vga_print!("{:04x}: ", offset);
                        
                        // 16진수 출력
                        for i in 0..16 {
                            crate::vga_print!("{:02x} ", buffer[offset + i]);
                        }
                        
                        crate::vga_print!(" ");
                        
                        // ASCII 출력
                        for i in 0..16 {
                            let byte = buffer[offset + i];
                            if byte >= 32 && byte <= 126 {
                                crate::vga_print!("{}", byte as char);
                            } else {
                                crate::vga_print!(".");
                            }
                        }
                        
                        vga_println!();
                    }
                    
                    Ok(())
                }
                Err(e) => Err(format!("Failed to read sector: {:?}", e)),
            }
        } else {
            Err(String::from("No disk available"))
        }
    }

    /// write 명령어: 섹터 쓰기 (테스트용)
    fn cmd_write(&self, args: &[&str]) -> Result<(), String> {
        use crate::drivers::ata::{PRIMARY_MASTER, BlockDevice};
        
        if args.is_empty() {
            return Err(String::from("Usage: write <sector_number>"));
        }
        
        let sector: u64 = args[0].parse()
            .map_err(|_| String::from("Invalid sector number"))?;
        
        let mut disk = PRIMARY_MASTER.lock();
        
        if let Some(ref mut driver) = *disk {
            // 테스트 데이터 생성
            let mut buffer = [0u8; 512];
            
            // 섹터 번호와 패턴으로 채우기
            let pattern = format!("Test sector {} - Simple OS\n", sector);
            let pattern_bytes = pattern.as_bytes();
            
            for i in 0..buffer.len() {
                if i < pattern_bytes.len() {
                    buffer[i] = pattern_bytes[i];
                } else {
                    buffer[i] = (i % 256) as u8;
                }
            }
            
            match driver.write_block(sector, &buffer) {
                Ok(_) => {
                    vga_println!("Wrote test data to sector {} successfully", sector);
                    vga_println!("Warning: This overwrites existing data!");
                    Ok(())
                }
                Err(e) => Err(format!("Failed to write sector: {:?}", e)),
            }
        } else {
            Err(String::from("No disk available"))
        }
    }
}
