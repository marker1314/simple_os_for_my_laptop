//! VESA Framebuffer Driver
//!
//! 이 모듈은 VESA 프레임버퍼를 통해 그래픽 출력을 제공합니다.
//! bootloader가 제공하는 프레임버퍼 정보를 사용합니다.

use bootloader_api::info::{FrameBuffer, PixelFormat};
use spin::Mutex;

/// 프레임버퍼 전역 인스턴스
static FRAMEBUFFER: Mutex<Option<FrameBufferWriter>> = Mutex::new(None);

/// 프레임버퍼 Writer 구조체
pub struct FrameBufferWriter {
    buffer: &'static mut [u8],
    info: FrameBufferInfo,
}

/// 프레임버퍼 정보
#[derive(Debug, Clone, Copy)]
pub struct FrameBufferInfo {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub pixel_format: PixelFormat,
    pub bytes_per_pixel: usize,
}

/// RGB 색상 구조체
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255 };
    pub const RED: Color = Color { r: 255, g: 0, b: 0 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255 };
    pub const YELLOW: Color = Color { r: 255, g: 255, b: 0 };
    pub const CYAN: Color = Color { r: 0, g: 255, b: 255 };
    pub const MAGENTA: Color = Color { r: 255, g: 0, b: 255 };
    pub const GRAY: Color = Color { r: 128, g: 128, b: 128 };
    pub const LIGHT_GRAY: Color = Color { r: 192, g: 192, b: 192 };
    pub const DARK_GRAY: Color = Color { r: 64, g: 64, b: 64 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }
}

impl FrameBufferWriter {
    /// 새 FrameBufferWriter 생성
    pub fn new(framebuffer: &'static mut FrameBuffer) -> Self {
        let info = FrameBufferInfo {
            width: framebuffer.info().width,
            height: framebuffer.info().height,
            stride: framebuffer.info().stride,
            pixel_format: framebuffer.info().pixel_format,
            bytes_per_pixel: framebuffer.info().bytes_per_pixel,
        };

        let buffer = framebuffer.buffer_mut();

        FrameBufferWriter { buffer, info }
    }

    /// 특정 픽셀에 색상 설정
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }

        let pixel_offset = y * self.info.stride + x;
        let byte_offset = pixel_offset * self.info.bytes_per_pixel;

        if byte_offset + self.info.bytes_per_pixel > self.buffer.len() {
            return;
        }

        match self.info.pixel_format {
            PixelFormat::Rgb => {
                self.buffer[byte_offset] = color.r;
                self.buffer[byte_offset + 1] = color.g;
                self.buffer[byte_offset + 2] = color.b;
            }
            PixelFormat::Bgr => {
                self.buffer[byte_offset] = color.b;
                self.buffer[byte_offset + 1] = color.g;
                self.buffer[byte_offset + 2] = color.r;
            }
            PixelFormat::U8 => {
                // 그레이스케일: 평균값 사용
                let gray = ((color.r as u16 + color.g as u16 + color.b as u16) / 3) as u8;
                self.buffer[byte_offset] = gray;
            }
            _ => {}
        }
    }

    /// 특정 픽셀의 색상 가져오기
    pub fn get_pixel(&self, x: usize, y: usize) -> Option<Color> {
        if x >= self.info.width || y >= self.info.height {
            return None;
        }

        let pixel_offset = y * self.info.stride + x;
        let byte_offset = pixel_offset * self.info.bytes_per_pixel;

        if byte_offset + self.info.bytes_per_pixel > self.buffer.len() {
            return None;
        }

        match self.info.pixel_format {
            PixelFormat::Rgb => Some(Color {
                r: self.buffer[byte_offset],
                g: self.buffer[byte_offset + 1],
                b: self.buffer[byte_offset + 2],
            }),
            PixelFormat::Bgr => Some(Color {
                r: self.buffer[byte_offset + 2],
                g: self.buffer[byte_offset + 1],
                b: self.buffer[byte_offset],
            }),
            PixelFormat::U8 => {
                let gray = self.buffer[byte_offset];
                Some(Color {
                    r: gray,
                    g: gray,
                    b: gray,
                })
            }
            _ => None,
        }
    }

    /// 화면 전체를 특정 색상으로 채우기
    pub fn clear(&mut self, color: Color) {
        for y in 0..self.info.height {
            for x in 0..self.info.width {
                self.set_pixel(x, y, color);
            }
        }
    }

    /// 사각형 그리기 (채워진)
    pub fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// 사각형 테두리 그리기
    pub fn draw_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        // 상단
        for dx in 0..width {
            self.set_pixel(x + dx, y, color);
        }
        // 하단
        for dx in 0..width {
            self.set_pixel(x + dx, y + height - 1, color);
        }
        // 좌측
        for dy in 0..height {
            self.set_pixel(x, y + dy, color);
        }
        // 우측
        for dy in 0..height {
            self.set_pixel(x + width - 1, y + dy, color);
        }
    }

    /// 선 그리기 (Bresenham 알고리즘)
    pub fn draw_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, color: Color) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && y >= 0 {
                self.set_pixel(x as usize, y as usize, color);
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// 원 그리기 (중점 원 알고리즘)
    pub fn draw_circle(&mut self, cx: isize, cy: isize, radius: isize, color: Color) {
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.set_pixel_signed(cx + x, cy + y, color);
            self.set_pixel_signed(cx + y, cy + x, color);
            self.set_pixel_signed(cx - y, cy + x, color);
            self.set_pixel_signed(cx - x, cy + y, color);
            self.set_pixel_signed(cx - x, cy - y, color);
            self.set_pixel_signed(cx - y, cy - x, color);
            self.set_pixel_signed(cx + y, cy - x, color);
            self.set_pixel_signed(cx + x, cy - y, color);

            if err <= 0 {
                y += 1;
                err += 2 * y + 1;
            }

            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }

    /// 채워진 원 그리기
    pub fn fill_circle(&mut self, cx: isize, cy: isize, radius: isize, color: Color) {
        for y in -radius..=radius {
            for x in -radius..=radius {
                if x * x + y * y <= radius * radius {
                    self.set_pixel_signed(cx + x, cy + y, color);
                }
            }
        }
    }

    /// signed 좌표로 픽셀 설정
    fn set_pixel_signed(&mut self, x: isize, y: isize, color: Color) {
        if x >= 0 && y >= 0 {
            self.set_pixel(x as usize, y as usize, color);
        }
    }

    /// 프레임버퍼 정보 가져오기
    pub fn info(&self) -> &FrameBufferInfo {
        &self.info
    }
}

/// 프레임버퍼 초기화
pub fn init(framebuffer: &'static mut FrameBuffer) {
    let writer = FrameBufferWriter::new(framebuffer);
    *FRAMEBUFFER.lock() = Some(writer);
}

/// 프레임버퍼가 초기화되었는지 확인
pub fn is_initialized() -> bool {
    FRAMEBUFFER.lock().is_some()
}

/// 프레임버퍼 정보 가져오기
pub fn info() -> Option<FrameBufferInfo> {
    FRAMEBUFFER.lock().as_ref().map(|fb| fb.info)
}

/// 화면 전체를 특정 색상으로 지우기
pub fn clear(color: Color) {
    if let Some(fb) = FRAMEBUFFER.lock().as_mut() {
        fb.clear(color);
    }
}

/// 픽셀 그리기
pub fn set_pixel(x: usize, y: usize, color: Color) {
    if let Some(fb) = FRAMEBUFFER.lock().as_mut() {
        fb.set_pixel(x, y, color);
    }
}

/// 채워진 사각형 그리기
pub fn fill_rect(x: usize, y: usize, width: usize, height: usize, color: Color) {
    if let Some(fb) = FRAMEBUFFER.lock().as_mut() {
        fb.fill_rect(x, y, width, height, color);
    }
}

/// 사각형 테두리 그리기
pub fn draw_rect(x: usize, y: usize, width: usize, height: usize, color: Color) {
    if let Some(fb) = FRAMEBUFFER.lock().as_mut() {
        fb.draw_rect(x, y, width, height, color);
    }
}

/// 선 그리기
pub fn draw_line(x0: isize, y0: isize, x1: isize, y1: isize, color: Color) {
    if let Some(fb) = FRAMEBUFFER.lock().as_mut() {
        fb.draw_line(x0, y0, x1, y1, color);
    }
}

/// 원 그리기
pub fn draw_circle(cx: isize, cy: isize, radius: isize, color: Color) {
    if let Some(fb) = FRAMEBUFFER.lock().as_mut() {
        fb.draw_circle(cx, cy, radius, color);
    }
}

/// 채워진 원 그리기
pub fn fill_circle(cx: isize, cy: isize, radius: isize, color: Color) {
    if let Some(fb) = FRAMEBUFFER.lock().as_mut() {
        fb.fill_circle(cx, cy, radius, color);
    }
}

/// 프레임버퍼에 직접 접근하여 작업 수행
pub fn with_framebuffer<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut FrameBufferWriter) -> R,
{
    FRAMEBUFFER.lock().as_mut().map(f)
}

/// 화면 너비 가져오기
pub fn get_width() -> usize {
    FRAMEBUFFER.lock().as_ref().map_or(800, |fb| fb.info.width)
}

/// 화면 높이 가져오기
pub fn get_height() -> usize {
    FRAMEBUFFER.lock().as_ref().map_or(600, |fb| fb.info.height)
}

