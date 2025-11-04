//! PIC (Programmable Interrupt Controller) 리매핑
//!
//! 이 모듈은 PIC를 리매핑하여 하드웨어 인터럽트가 시스템 예외와 충돌하지 않도록 합니다.

use x86_64::instructions::port::Port;
use x86_64::instructions::interrupts;

/// PIC 제어 포트
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

/// ICW1 (Initialization Command Word 1) 플래그
const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;

/// ICW4 플래그
const ICW4_8086: u8 = 0x01;

/// PIC 리매핑된 인터럽트 벡터 오프셋
pub const PIC1_OFFSET: u8 = 32; // 0x20 (시스템 예외 0-31 이후)
pub const PIC2_OFFSET: u8 = 40; // 0x28

/// PIC 초기화 및 리매핑
///
/// PIC를 리매핑하여 하드웨어 인터럽트가 시스템 예외와 충돌하지 않도록 합니다.
/// PIC1은 IRQ 0-7을 인터럽트 32-39로 매핑하고,
/// PIC2는 IRQ 8-15를 인터럽트 40-47로 매핑합니다.
pub unsafe fn init() {
    let mut pic1_command = Port::new(PIC1_COMMAND);
    let mut pic1_data = Port::new(PIC1_DATA);
    let mut pic2_command = Port::new(PIC2_COMMAND);
    let mut pic2_data = Port::new(PIC2_DATA);

    // 인터럽트 비활성화 (초기화 중)
    let mut wait_port: Port<u8> = Port::new(0x80);
    let mut wait = || wait_port.write(0);

    // ICW1: 초기화 시작
    pic1_command.write(ICW1_INIT | ICW1_ICW4);
    wait();
    pic2_command.write(ICW1_INIT | ICW1_ICW4);
    wait();

    // ICW2: 벡터 오프셋 설정
    pic1_data.write(PIC1_OFFSET);
    wait();
    pic2_data.write(PIC2_OFFSET);
    wait();

    // ICW3: 마스터/슬레이브 연결
    pic1_data.write(4); // PIC2가 IRQ2에 연결됨
    wait();
    pic2_data.write(2); // PIC2의 슬레이브 ID
    wait();

    // ICW4: 8086 모드
    pic1_data.write(ICW4_8086);
    wait();
    pic2_data.write(ICW4_8086);
    wait();

    // 모든 인터럽트 마스크 (나중에 개별적으로 활성화)
    pic1_data.write(0xFF);
    pic2_data.write(0xFF);
}

/// PIC에서 인터럽트 마스크 설정
pub unsafe fn set_mask(irq: u8, enabled: bool) {
    interrupts::without_interrupts(|| {
        let mut port = if irq < 8 {
            Port::new(PIC1_DATA)
        } else {
            Port::new(PIC2_DATA)
        };

        let mut current_mask = port.read() as u8;
        let bit = 1 << (irq % 8);

        if enabled {
            current_mask &= !bit; // 비트 클리어 = 인터럽트 활성화
        } else {
            current_mask |= bit; // 비트 설정 = 인터럽트 비활성화
        }

        port.write(current_mask);
    });
}

/// PIC 인터럽트 종료 신호 전송
pub unsafe fn end_of_interrupt(irq: u8) {
    if irq >= 8 {
        // 슬레이브 PIC에도 EOI 전송
        Port::new(PIC2_COMMAND).write(0x20_u8);
    }
    // 마스터 PIC에 EOI 전송
    Port::new(PIC1_COMMAND).write(0x20_u8);
}

