//! PCM (Pulse Code Modulation) 오디오 스트림
//!
//! PCM 오디오 데이터를 처리합니다.

/// PCM 포맷
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmFormat {
    /// 샘플링 레이트 (Hz)
    pub sample_rate: u32,
    /// 채널 수 (1 = 모노, 2 = 스테레오)
    pub channels: u8,
    /// 샘플 크기 (비트)
    pub sample_size: u8,
    /// 부호 있는/없는 (true = signed)
    pub signed: bool,
}

impl PcmFormat {
    /// 새 PCM 포맷 생성
    pub fn new(sample_rate: u32, channels: u8, sample_size: u8, signed: bool) -> Self {
        Self {
            sample_rate,
            channels,
            sample_size,
            signed,
        }
    }
    
    /// 기본 포맷 (44.1kHz, 스테레오, 16-bit, signed)
    pub fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            sample_size: 16,
            signed: true,
        }
    }
    
    /// 바이트당 샘플 크기 계산
    pub fn bytes_per_sample(&self) -> usize {
        (self.sample_size as usize + 7) / 8
    }
    
    /// 초당 바이트 수 계산
    pub fn bytes_per_second(&self) -> usize {
        self.sample_rate as usize * self.channels as usize * self.bytes_per_sample()
    }
}

/// PCM 스트림
pub struct PcmStream {
    /// PCM 포맷
    format: PcmFormat,
    /// 버퍼 크기 (바이트)
    buffer_size: usize,
    /// 활성화 여부
    active: bool,
}

impl PcmStream {
    /// 새 PCM 스트림 생성
    pub fn new(format: PcmFormat, buffer_size: usize) -> Self {
        Self {
            format,
            buffer_size,
            active: false,
        }
    }
    
    /// 스트림 시작
    pub fn start(&mut self) -> Result<(), PcmError> {
        if self.active {
            return Err(PcmError::AlreadyActive);
        }
        
        // TODO: HDA 컨트롤러에 스트림 설정
        crate::log_debug!("PCM stream started: {}Hz, {}ch, {}bit",
                         self.format.sample_rate,
                         self.format.channels,
                         self.format.sample_size);
        
        self.active = true;
        Ok(())
    }
    
    /// 스트림 중지
    pub fn stop(&mut self) -> Result<(), PcmError> {
        if !self.active {
            return Err(PcmError::NotActive);
        }
        
        // TODO: HDA 컨트롤러에 스트림 중지
        self.active = false;
        Ok(())
    }
    
    /// PCM 데이터 쓰기
    pub fn write(&mut self, data: &[u8]) -> Result<usize, PcmError> {
        if !self.active {
            return Err(PcmError::NotActive);
        }
        
        // TODO: HDA 컨트롤러에 데이터 전송
        crate::log_debug!("PCM write: {} bytes", data.len());
        
        Ok(data.len())
    }
    
    /// 포맷 가져오기
    pub fn format(&self) -> PcmFormat {
        self.format
    }
    
    /// 활성화 여부 확인
    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// PCM 에러
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcmError {
    NotInitialized,
    InvalidFormat,
    BufferTooSmall,
    AlreadyActive,
    NotActive,
    IoError,
}

impl core::fmt::Display for PcmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PcmError::NotInitialized => write!(f, "PCM stream not initialized"),
            PcmError::InvalidFormat => write!(f, "Invalid PCM format"),
            PcmError::BufferTooSmall => write!(f, "PCM buffer too small"),
            PcmError::AlreadyActive => write!(f, "PCM stream already active"),
            PcmError::NotActive => write!(f, "PCM stream not active"),
            PcmError::IoError => write!(f, "PCM I/O error"),
        }
    }
}

