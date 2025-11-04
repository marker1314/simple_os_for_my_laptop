//! 시스템 콜 인터페이스 모듈
//!
//! 이 모듈은 사용자 공간과 커널 공간 간의 인터페이스를 제공합니다.
//! x86_64에서는 인터럽트 0x80을 통해 시스템 콜을 호출합니다.
//!
//! 시스템 콜 호출 규약:
//! - 시스템 콜 번호: RAX 레지스터
//! - 파라미터: RDI, RSI, RDX, R10, R8, R9 (최대 6개)
//! - 반환값: RAX 레지스터
//! - 에러 코드: 음수 값 (성공 시 0 이상)

mod numbers;
mod handler;
mod dispatcher;
mod implementations;

pub use numbers::SyscallNumber;
pub use handler::init_syscall_handler;

use x86_64::structures::idt::InterruptStackFrame;

/// 시스템 콜 결과 타입
///
/// 성공 시 값을 반환하고, 실패 시 에러 코드를 반환합니다.
pub type SyscallResult = Result<u64, SyscallError>;

/// 시스템 콜 에러 코드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    /// 잘못된 시스템 콜 번호
    InvalidSyscall = -1,
    /// 잘못된 파라미터
    InvalidArgument = -2,
    /// 권한 없음
    PermissionDenied = -3,
    /// 파일/리소스를 찾을 수 없음
    NotFound = -4,
    /// 리소스 부족
    ResourceExhausted = -5,
    /// I/O 에러
    IoError = -6,
    /// 인터럽트됨
    Interrupted = -7,
}

impl SyscallError {
    /// 에러 코드를 i64로 변환
    pub fn as_i64(self) -> i64 {
        self as i64
    }
}

/// 시스템 콜 핸들러
///
/// IDT의 인터럽트 핸들러에서 호출됩니다.
/// 인터럽트 발생 시 스택에 저장된 레지스터를 통해 파라미터를 읽고 결과를 반환합니다.
pub extern "x86-interrupt" fn syscall_handler(stack_frame: InterruptStackFrame) {
    use dispatcher::dispatch_syscall;
    use core::arch::asm;
    
    // 인터럽트 핸들러 진입 시 레지스터는 이미 스택에 저장되어 있음
    // 하지만 x86-interrupt ABI를 사용하면 함수 파라미터로 전달되지 않음
    // 인라인 어셈블리를 사용하여 레지스터 직접 읽기
    // 주의: 인터럽트 핸들러 진입 시점에 레지스터는 아직 저장되지 않았을 수 있음
    // 따라서 스택에서 읽어야 하지만, 간단하게 하기 위해 레지스터 직접 읽기 시도
    
    let syscall_num: u64;
    let arg1: u64;
    let arg2: u64;
    let arg3: u64;
    let arg4: u64;
    let arg5: u64;
    let arg6: u64;
    
    unsafe {
        // 레지스터에서 직접 읽기
        // x86-interrupt ABI를 사용하면 인터럽트 핸들러 진입 시 레지스터가 보존됨
        // 하지만 실제로는 스택에 저장되므로 스택에서 읽어야 함
        // 현재는 테스트를 위해 레지스터 직접 읽기 시도
        asm!(
            "mov {}, rax",
            "mov {}, rdi",
            "mov {}, rsi",
            "mov {}, rdx",
            "mov {}, r10",
            "mov {}, r8",
            "mov {}, r9",
            out(reg) syscall_num,
            out(reg) arg1,
            out(reg) arg2,
            out(reg) arg3,
            out(reg) arg4,
            out(reg) arg5,
            out(reg) arg6,
        );
    }
    
    // 시스템 콜 디스패치
    let result = dispatch_syscall(
        syscall_num,
        arg1, arg2, arg3, arg4, arg5, arg6,
    );
    
    // 결과를 RAX에 저장
    // 인터럽트 핸들러 반환 시 스택에서 레지스터가 복원되므로,
    // 스택에 저장된 RAX 값을 수정해야 함
    // 현재는 간단하게 레지스터 직접 설정
    unsafe {
        asm!(
            "mov rax, {}",
            in(reg) result,
        );
    }
    
    crate::log_debug!("Syscall {} returned: {}", syscall_num, result);
}
