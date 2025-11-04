//! 시스템 콜 번호 정의
//!
//! 각 시스템 콜은 고유한 번호를 가집니다.
//! 이 번호는 RAX 레지스터를 통해 전달됩니다.

/// 시스템 콜 번호
///
/// 각 시스템 콜은 고유한 번호를 가집니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    /// 프로세스 종료
    /// 파라미터: exit_code (u64)
    Exit = 0,
    
    /// 데이터 쓰기
    /// 파라미터: fd (u64), buf (const u8*), count (u64)
    /// 반환값: 쓰여진 바이트 수
    Write = 1,
    
    /// 데이터 읽기
    /// 파라미터: fd (u64), buf (u8*), count (u64)
    /// 반환값: 읽은 바이트 수
    Read = 2,
    
    /// CPU 양보 (다른 스레드에게 실행 권한 양보)
    /// 파라미터: 없음
    /// 반환값: 항상 0 (성공)
    Yield = 3,
    
    /// 대기 (밀리초 단위)
    /// 파라미터: milliseconds (u64)
    /// 반환값: 남은 밀리초 (일반적으로 0)
    Sleep = 4,
    
    /// 현재 시간 얻기 (밀리초)
    /// 파라미터: 없음
    /// 반환값: 부팅 이후 경과한 밀리초
    GetTime = 5,
    
    /// 프로세스 ID 얻기
    /// 파라미터: 없음
    /// 반환값: 현재 프로세스/스레드 ID
    GetPid = 6,
}

impl SyscallNumber {
    /// u64에서 시스템 콜 번호로 변환
    pub fn from_u64(value: u64) -> Option<Self> {
        match value {
            0 => Some(SyscallNumber::Exit),
            1 => Some(SyscallNumber::Write),
            2 => Some(SyscallNumber::Read),
            3 => Some(SyscallNumber::Yield),
            4 => Some(SyscallNumber::Sleep),
            5 => Some(SyscallNumber::GetTime),
            6 => Some(SyscallNumber::GetPid),
            _ => None,
        }
    }
    
    /// 시스템 콜 번호를 u64로 변환
    pub fn as_u64(self) -> u64 {
        self as u64
    }
}

/// 시스템 콜 최대 번호
pub const MAX_SYSCALL_NUMBER: u64 = 6;

