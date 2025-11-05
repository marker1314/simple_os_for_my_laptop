//! PIT (Programmable Interval Timer) 드라이버
//!
//! 이 모듈은 x86 PIT를 사용하여 시스템 타이머를 구현합니다.
//! PIT는 1.193182 MHz의 고정 클럭을 사용하며, 분주기를 통해 원하는 주파수로 설정할 수 있습니다.

use x86_64::instructions::port::Port;
use spin::Mutex;
use crate::interrupts::pic;

/// PIT I/O 포트 주소
const PIT_CHANNEL0_DATA: u16 = 0x40;
const PIT_CHANNEL1_DATA: u16 = 0x41;
const PIT_CHANNEL2_DATA: u16 = 0x42;
const PIT_COMMAND: u16 = 0x43;

/// PIT 기본 클럭 주파수 (Hz)
const PIT_BASE_FREQUENCY: u32 = 1_193_182;

/// 타이머 틱 카운터
static TICK_COUNT: Mutex<u64> = Mutex::new(0);

/// 밀리초당 틱 수 (1000Hz = 1ms마다 인터럽트)
const TICKS_PER_SECOND: u32 = 1000;

/// Idle 상태에서 동적 tick 조정
/// Idle 상태일 때는 더 긴 간격으로 tick (타겟 ≥ 10ms)
static IDLE_TICK_MODE: Mutex<bool> = Mutex::new(false);
static SKIP_TICKS: Mutex<u32> = Mutex::new(0);
static TICK_SKIP_COUNTER: Mutex<u32> = Mutex::new(0);

/// 밀리초 가져오기
pub fn get_milliseconds() -> u64 {
    *TICK_COUNT.lock()
}

/// 초 가져오기
pub fn get_seconds() -> u64 {
    get_milliseconds() / 1000
}

/// PIT 초기화
///
/// 주어진 주파수로 타이머를 설정합니다.
/// 기본값은 1000Hz (1ms마다 인터럽트)입니다.
///
/// # Safety
/// 이 함수는 한 번만 호출되어야 하며, 인터럽트가 비활성화된 상태에서 호출되어야 합니다.
pub unsafe fn init() {
    // 분주기 계산: PIT_BASE_FREQUENCY / 원하는_주파수
    let divisor = (PIT_BASE_FREQUENCY / TICKS_PER_SECOND) as u16;
    
    // Command Register에 명령 전송
    // Bits 7-6: Channel 0 선택 (00)
    // Bits 5-4: Access mode - Low/High byte (11 = lobyte/hibyte)
    // Bits 3-1: Operating mode - Mode 3 (Square Wave Generator) (011)
    // Bit 0: BCD mode - Binary (0)
    // 값: 0b00110110 = 0x36
    let mut command_port: Port<u8> = Port::new(PIT_COMMAND);
    command_port.write(0x36); // Channel 0, lobyte/hibyte, Mode 3, Binary
    
    // 분주기 값 전송 (먼저 하위 바이트, 그 다음 상위 바이트)
    let mut data_port: Port<u8> = Port::new(PIT_CHANNEL0_DATA);
    data_port.write((divisor & 0xFF) as u8);      // 하위 바이트
    data_port.write(((divisor >> 8) & 0xFF) as u8); // 상위 바이트
    
    crate::log_info!("PIT initialized: {} Hz (divisor: {})", TICKS_PER_SECOND, divisor);
}

/// 타이머 인터럽트 핸들러
///
/// 타이머 틱이 발생할 때마다 호출됩니다.
/// 이 함수는 인터럽트 컨텍스트에서 실행되므로 빠르게 처리해야 합니다.
pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: x86_64::structures::idt::InterruptStackFrame) {
    // Idle tick coalescing: 일부 tick을 스킵하여 wakeup 감소
    let mut skip_counter = TICK_SKIP_COUNTER.lock();
    let skip_ticks = *SKIP_TICKS.lock();
    let idle_mode = *IDLE_TICK_MODE.lock();
    
    if idle_mode && skip_ticks > 0 {
        *skip_counter += 1;
        if *skip_counter < skip_ticks {
            // Tick 스킵 - 타이머만 증가, 나머지는 처리하지 않음
            *TICK_COUNT.lock() += 1;
            unsafe {
                pic::end_of_interrupt(0);
            }
            return;
        }
        *skip_counter = 0;
    }
    
    // 타이머 틱 증가
    *TICK_COUNT.lock() += 1;
    
    // 스케줄러 틱 처리
    // TODO: 컨텍스트 스위칭이 필요하면 여기서 처리
    crate::scheduler::tick();
    
    // Watchdog 체크
    crate::kernel::watchdog::check();
    
    // PIC에 인터럽트 종료 신호 전송 (IRQ 0)
    unsafe {
        pic::end_of_interrupt(0);
    }
}

/// Idle 상태에서 tick coalescing 활성화
/// skip_ticks: 몇 개의 tick을 스킵할지 (예: 10 = 10ms마다 tick)
pub fn set_idle_tick_coalescing(enabled: bool, skip_ticks: u32) {
    *IDLE_TICK_MODE.lock() = enabled;
    *SKIP_TICKS.lock() = skip_ticks;
    *TICK_SKIP_COUNTER.lock() = 0;
}

/// 지정된 밀리초 동안 대기
///
/// # Arguments
/// * `ms` - 대기할 밀리초
pub fn sleep_ms(ms: u64) {
    let start = get_milliseconds();
    while get_milliseconds() - start < ms {
        x86_64::instructions::hlt();
    }
}

