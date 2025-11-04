//! PS/2 마우스 드라이버
//!
//! 이 모듈은 PS/2 포트를 통한 마우스 입력을 처리합니다.

use spin::Mutex;
use x86_64::instructions::port::Port;

/// 마우스 데이터 포트
const MOUSE_DATA_PORT: u16 = 0x60;
/// 마우스 명령/상태 포트
const MOUSE_COMMAND_PORT: u16 = 0x64;

/// 마우스 상태
static MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState::new());

/// 마우스 이벤트 큐
static MOUSE_EVENTS: Mutex<MouseEventQueue> = Mutex::new(MouseEventQueue::new());

/// 마우스 상태 구조체
#[derive(Debug, Clone, Copy)]
pub struct MouseState {
    pub x: isize,
    pub y: isize,
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
    cycle: u8,
    data: [u8; 3],
}

impl MouseState {
    const fn new() -> Self {
        MouseState {
            x: 0,
            y: 0,
            left_button: false,
            right_button: false,
            middle_button: false,
            cycle: 0,
            data: [0; 3],
        }
    }

    /// 마우스 데이터 바이트 처리
    fn process_byte(&mut self, byte: u8, screen_width: isize, screen_height: isize) -> Option<MouseEvent> {
        self.data[self.cycle as usize] = byte;
        self.cycle += 1;

        if self.cycle == 3 {
            self.cycle = 0;

            // 패킷 파싱
            let flags = self.data[0];
            let dx = self.data[1] as i16 - ((flags as i16 & 0x10) << 4);
            let dy = self.data[2] as i16 - ((flags as i16 & 0x20) << 3);

            // 버튼 상태
            let left_button = (flags & 0x01) != 0;
            let right_button = (flags & 0x02) != 0;
            let middle_button = (flags & 0x04) != 0;

            // 위치 업데이트 (Y축 반전)
            self.x += dx as isize;
            self.y -= dy as isize; // Y축은 화면 좌표계와 반대

            // 화면 경계 체크
            if self.x < 0 {
                self.x = 0;
            }
            if self.x >= screen_width {
                self.x = screen_width - 1;
            }
            if self.y < 0 {
                self.y = 0;
            }
            if self.y >= screen_height {
                self.y = screen_height - 1;
            }

            // 버튼 이벤트 생성
            let mut event = None;

            if left_button != self.left_button {
                event = Some(if left_button {
                    MouseEvent::LeftButtonDown(self.x, self.y)
                } else {
                    MouseEvent::LeftButtonUp(self.x, self.y)
                });
                self.left_button = left_button;
            } else if right_button != self.right_button {
                event = Some(if right_button {
                    MouseEvent::RightButtonDown(self.x, self.y)
                } else {
                    MouseEvent::RightButtonUp(self.x, self.y)
                });
                self.right_button = right_button;
            } else if middle_button != self.middle_button {
                event = Some(if middle_button {
                    MouseEvent::MiddleButtonDown(self.x, self.y)
                } else {
                    MouseEvent::MiddleButtonUp(self.x, self.y)
                });
                self.middle_button = middle_button;
            } else if dx != 0 || dy != 0 {
                event = Some(MouseEvent::Move(self.x, self.y));
            }

            event
        } else {
            None
        }
    }
}

/// 마우스 이벤트
#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Move(isize, isize),
    LeftButtonDown(isize, isize),
    LeftButtonUp(isize, isize),
    RightButtonDown(isize, isize),
    RightButtonUp(isize, isize),
    MiddleButtonDown(isize, isize),
    MiddleButtonUp(isize, isize),
}

/// 마우스 이벤트 큐
struct MouseEventQueue {
    events: [Option<MouseEvent>; 32],
    head: usize,
    tail: usize,
}

impl MouseEventQueue {
    const fn new() -> Self {
        MouseEventQueue {
            events: [None; 32],
            head: 0,
            tail: 0,
        }
    }

    fn push(&mut self, event: MouseEvent) {
        let next_tail = (self.tail + 1) % self.events.len();
        if next_tail != self.head {
            self.events[self.tail] = Some(event);
            self.tail = next_tail;
        }
    }

    fn pop(&mut self) -> Option<MouseEvent> {
        if self.head == self.tail {
            None
        } else {
            let event = self.events[self.head];
            self.head = (self.head + 1) % self.events.len();
            event
        }
    }
}

/// PS/2 마우스 초기화
pub unsafe fn init() {
    let mut command_port = Port::<u8>::new(MOUSE_COMMAND_PORT);
    let mut data_port = Port::<u8>::new(MOUSE_DATA_PORT);

    // 보조 디바이스 활성화
    command_port.write(0xA8);

    // 컨트롤러 설정 읽기
    command_port.write(0x20);
    wait_for_read();
    let mut status = data_port.read();

    // 인터럽트 활성화 및 클럭 활성화
    status |= 0x02; // 보조 디바이스 인터럽트 활성화
    status &= !0x20; // 보조 디바이스 클럭 활성화

    // 설정 쓰기
    command_port.write(0x60);
    wait_for_write();
    data_port.write(status);

    // 마우스 기본값 설정
    write_mouse(0xF6);
    read_mouse();

    // 마우스 데이터 보고 활성화
    write_mouse(0xF4);
    read_mouse();
}

/// 마우스 인터럽트 핸들러
pub fn handle_interrupt() {
    let mut data_port = Port::<u8>::new(MOUSE_DATA_PORT);
    
    unsafe {
        let byte = data_port.read();
        
        // 화면 크기 가져오기 (기본값: 800x600)
        let (screen_width, screen_height) = if let Some(info) = crate::drivers::framebuffer::info() {
            (info.width as isize, info.height as isize)
        } else {
            (800, 600)
        };

        // 인터럽트 컨텍스트: 잠금 경합 시 즉시 드롭하여 지연/데드락 방지
        if let Some(mut state) = MOUSE_STATE.try_lock() {
            if let Some(event) = state.process_byte(byte, screen_width, screen_height) {
                if let Some(mut q) = MOUSE_EVENTS.try_lock() {
                    q.push(event);
                }
            }
        }
    }
}

/// 마우스 이벤트 가져오기
pub fn get_event() -> Option<MouseEvent> {
    MOUSE_EVENTS.lock().pop()
}

/// 현재 마우스 위치 가져오기
pub fn get_position() -> (isize, isize) {
    let state = MOUSE_STATE.lock();
    (state.x, state.y)
}

/// 버튼 상태 가져오기
pub fn get_buttons() -> (bool, bool, bool) {
    let state = MOUSE_STATE.lock();
    (state.left_button, state.right_button, state.middle_button)
}

/// 마우스 위치 설정
pub fn set_position(x: isize, y: isize) {
    let mut state = MOUSE_STATE.lock();
    state.x = x;
    state.y = y;
}

/// 마우스에 명령 쓰기
unsafe fn write_mouse(value: u8) {
    let mut command_port = Port::<u8>::new(MOUSE_COMMAND_PORT);
    let mut data_port = Port::<u8>::new(MOUSE_DATA_PORT);

    wait_for_write();
    command_port.write(0xD4);

    wait_for_write();
    data_port.write(value);
}

/// 마우스에서 데이터 읽기
unsafe fn read_mouse() -> u8 {
    let mut data_port = Port::<u8>::new(MOUSE_DATA_PORT);
    wait_for_read();
    data_port.read()
}

/// 쓰기 대기
unsafe fn wait_for_write() {
    let mut command_port = Port::<u8>::new(MOUSE_COMMAND_PORT);
    for _ in 0..1000 {
        if (command_port.read() & 0x02) == 0 {
            return;
        }
    }
}

/// 읽기 대기
unsafe fn wait_for_read() {
    let mut command_port = Port::<u8>::new(MOUSE_COMMAND_PORT);
    for _ in 0..1000 {
        if (command_port.read() & 0x01) != 0 {
            return;
        }
    }
}

