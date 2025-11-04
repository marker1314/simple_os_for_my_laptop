//! ELAN 트랙패드 드라이버
//!
//! ELAN I2C-HID 트랙패드를 지원합니다.
//! 특히 ELAN708:00 04F3:30A0 모델을 타겟으로 합니다.

use spin::Mutex;
use crate::drivers::i2c_hid::{I2cHidDevice, I2cHidError};
use crate::drivers::mouse::MouseEvent;

/// ELAN 트랙패드 제조사 ID
pub const ELAN_VENDOR_ID: u16 = 0x04F3;

/// ELAN708 제품 ID
pub const ELAN708_PRODUCT_ID: u16 = 0x30A0;

/// 트랙패드 에러
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchpadError {
    /// I2C-HID 에러
    I2cHidError(I2cHidError),
    /// 지원하지 않는 장치
    UnsupportedDevice,
    /// 초기화되지 않음
    NotInitialized,
    /// 잘못된 리포트
    InvalidReport,
}

impl From<I2cHidError> for TouchpadError {
    fn from(err: I2cHidError) -> Self {
        TouchpadError::I2cHidError(err)
    }
}

/// 터치 포인트 정보
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    /// X 좌표 (절대 좌표)
    pub x: u16,
    /// Y 좌표 (절대 좌표)
    pub y: u16,
    /// 터치 여부
    pub touching: bool,
    /// 압력
    pub pressure: u8,
}

/// ELAN 트랙패드 드라이버
pub struct ElanTouchpad {
    /// I2C-HID 장치
    device: I2cHidDevice,
    /// 트랙패드 최대 X 좌표
    max_x: u16,
    /// 트랙패드 최대 Y 좌표
    max_y: u16,
    /// 화면 너비
    screen_width: isize,
    /// 화면 높이
    screen_height: isize,
    /// 현재 마우스 X 위치
    cursor_x: isize,
    /// 현재 마우스 Y 위치
    cursor_y: isize,
    /// 이전 터치 상태
    prev_touch: Option<TouchPoint>,
    /// 왼쪽 버튼 상태 (터치 = 클릭)
    left_button: bool,
    /// 초기화 여부
    initialized: bool,
}

impl ElanTouchpad {
    /// 새 ELAN 트랙패드 생성
    ///
    /// # Arguments
    /// * `slave_addr` - I2C 슬레이브 주소
    pub const fn new(slave_addr: u8) -> Self {
        Self {
            device: I2cHidDevice::new(slave_addr),
            max_x: 3200, // ELAN708 기본값
            max_y: 2200, // ELAN708 기본값
            screen_width: 800,
            screen_height: 600,
            cursor_x: 400,
            cursor_y: 300,
            prev_touch: None,
            left_button: false,
            initialized: false,
        }
    }

    /// 트랙패드 초기화
    ///
    /// # Safety
    /// I2C 컨트롤러가 초기화된 후에 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), TouchpadError> {
        // I2C-HID 장치 초기화
        self.device.init()?;

        // Descriptor 검증
        if let Some(desc) = self.device.get_descriptor() {
            let vendor_id = desc.vendor_id;
            let product_id = desc.product_id;
            
            if vendor_id != ELAN_VENDOR_ID {
                crate::log_warn!("Non-ELAN touchpad detected: 0x{:04X}", vendor_id);
                // 계속 진행 (호환 가능할 수 있음)
            }

            crate::log_info!("ELAN touchpad initialized: Product=0x{:04X}", product_id);
        } else {
            return Err(TouchpadError::UnsupportedDevice);
        }

        // 화면 크기 가져오기
        if let Some(info) = crate::drivers::framebuffer::info() {
            self.screen_width = info.width as isize;
            self.screen_height = info.height as isize;
        }

        // 커서를 화면 중앙에 위치
        self.cursor_x = self.screen_width / 2;
        self.cursor_y = self.screen_height / 2;

        self.initialized = true;
        Ok(())
    }

    /// Input Report 처리
    ///
    /// # Returns
    /// 생성된 마우스 이벤트 (있는 경우)
    pub fn process_input(&mut self) -> Result<Option<MouseEvent>, TouchpadError> {
        if !self.initialized {
            return Err(TouchpadError::NotInitialized);
        }

        let mut buffer = [0u8; 64];
        let len = match self.device.read_input_report(&mut buffer) {
            Ok(len) => len,
            Err(I2cHidError::I2cError(crate::drivers::i2c::I2cError::Nack)) => {
                // NACK는 데이터가 없다는 의미 (정상)
                return Ok(None);
            }
            Err(e) => {
                return Err(TouchpadError::I2cHidError(e));
            }
        };

        if len < 6 {
            // 유효한 리포트가 아님
            return Ok(None);
        }

        // ELAN 트랙패드 리포트 파싱
        // 일반적인 형식:
        // [0]: Report ID
        // [1]: 버튼 상태
        // [2-3]: X 좌표 (Little-endian)
        // [4-5]: Y 좌표 (Little-endian)
        // [6]: 압력 (선택적)

        let _report_id = buffer[0];
        let button_state = buffer[1];
        let touch_x = u16::from_le_bytes([buffer[2], buffer[3]]);
        let touch_y = u16::from_le_bytes([buffer[4], buffer[5]]);
        let pressure = if len > 6 { buffer[6] } else { 0 };

        // 터치 여부 확인 (버튼 비트 또는 압력으로 판단)
        let touching = (button_state & 0x01) != 0 || pressure > 10;

        let current_touch = if touching {
            Some(TouchPoint {
                x: touch_x,
                y: touch_y,
                touching: true,
                pressure,
            })
        } else {
            None
        };

        // 이벤트 생성
        let event = self.create_mouse_event(current_touch);

        // 상태 업데이트
        self.prev_touch = current_touch;

        Ok(event)
    }

    /// 터치 정보를 마우스 이벤트로 변환
    fn create_mouse_event(&mut self, current_touch: Option<TouchPoint>) -> Option<MouseEvent> {
        match (self.prev_touch, current_touch) {
            // 터치 시작 (버튼 다운)
            (None, Some(touch)) => {
                // 절대 좌표를 화면 좌표로 변환
                self.cursor_x = (touch.x as isize * self.screen_width) / self.max_x as isize;
                self.cursor_y = (touch.y as isize * self.screen_height) / self.max_y as isize;
                
                // 경계 체크
                self.clamp_cursor();
                
                self.left_button = true;
                Some(MouseEvent::LeftButtonDown(self.cursor_x, self.cursor_y))
            }

            // 터치 이동
            (Some(prev), Some(current)) => {
                // 이전 위치와 현재 위치의 차이 계산
                let dx = (current.x as i32 - prev.x as i32) as isize;
                let dy = (current.y as i32 - prev.y as i32) as isize;

                // 상대 이동으로 커서 업데이트
                self.cursor_x += (dx * self.screen_width) / self.max_x as isize;
                self.cursor_y += (dy * self.screen_height) / self.max_y as isize;

                // 경계 체크
                self.clamp_cursor();

                if dx != 0 || dy != 0 {
                    Some(MouseEvent::Move(self.cursor_x, self.cursor_y))
                } else {
                    None
                }
            }

            // 터치 종료 (버튼 업)
            (Some(_), None) => {
                self.left_button = false;
                Some(MouseEvent::LeftButtonUp(self.cursor_x, self.cursor_y))
            }

            // 터치 없음
            (None, None) => None,
        }
    }

    /// 커서 위치를 화면 경계 내로 제한
    fn clamp_cursor(&mut self) {
        if self.cursor_x < 0 {
            self.cursor_x = 0;
        }
        if self.cursor_x >= self.screen_width {
            self.cursor_x = self.screen_width - 1;
        }
        if self.cursor_y < 0 {
            self.cursor_y = 0;
        }
        if self.cursor_y >= self.screen_height {
            self.cursor_y = self.screen_height - 1;
        }
    }

    /// 현재 커서 위치 가져오기
    pub fn get_position(&self) -> (isize, isize) {
        (self.cursor_x, self.cursor_y)
    }

    /// 버튼 상태 가져오기
    pub fn get_buttons(&self) -> (bool, bool, bool) {
        (self.left_button, false, false)
    }

    /// 초기화 여부 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// 전역 트랙패드 인스턴스
static TOUCHPAD: Mutex<Option<ElanTouchpad>> = Mutex::new(None);

/// 트랙패드 초기화
///
/// # Arguments
/// * `slave_addr` - I2C 슬레이브 주소 (일반적으로 0x15)
///
/// # Safety
/// I2C 컨트롤러가 초기화된 후에 호출되어야 합니다.
pub unsafe fn init(slave_addr: u8) -> Result<(), TouchpadError> {
    let mut touchpad = ElanTouchpad::new(slave_addr);
    touchpad.init()?;
    
    *TOUCHPAD.lock() = Some(touchpad);
    
    crate::log_info!("ELAN touchpad driver initialized");
    Ok(())
}

/// 트랙패드 이벤트 폴링
///
/// GUI 루프에서 주기적으로 호출되어야 합니다.
pub fn poll_event() -> Option<MouseEvent> {
    if let Some(ref mut touchpad) = *TOUCHPAD.lock() {
        match touchpad.process_input() {
            Ok(event) => event,
            Err(_e) => {
                // 에러는 무시하고 계속 진행
                None
            }
        }
    } else {
        None
    }
}

/// 현재 커서 위치 가져오기
pub fn get_position() -> Option<(isize, isize)> {
    TOUCHPAD.lock().as_ref().map(|tp| tp.get_position())
}

/// 버튼 상태 가져오기
pub fn get_buttons() -> Option<(bool, bool, bool)> {
    TOUCHPAD.lock().as_ref().map(|tp| tp.get_buttons())
}

/// 초기화 여부 확인
pub fn is_initialized() -> bool {
    TOUCHPAD.lock().as_ref().map(|tp| tp.is_initialized()).unwrap_or(false)
}

