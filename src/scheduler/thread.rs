//! 스레드 구조 및 컨텍스트 관리
//!
//! 이 모듈은 스레드의 상태와 CPU 컨텍스트를 관리합니다.

/// 스레드 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// 실행 가능 (Ready)
    Ready,
    /// 실행 중 (Running)
    Running,
    /// 대기 중 (Waiting/Blocked)
    Blocked,
    /// 종료됨 (Terminated)
    Terminated,
}

/// CPU 컨텍스트 (레지스터 상태)
///
/// x86_64의 주요 레지스터를 저장합니다.
/// 컨텍스트 스위칭 시 이 구조체를 사용하여 레지스터를 저장/복원합니다.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct ThreadContext {
    /// R15 레지스터
    pub r15: u64,
    /// R14 레지스터
    pub r14: u64,
    /// R13 레지스터
    pub r13: u64,
    /// R12 레지스터
    pub r12: u64,
    /// R11 레지스터
    pub r11: u64,
    /// R10 레지스터
    pub r10: u64,
    /// R9 레지스터
    pub r9: u64,
    /// R8 레지스터
    pub r8: u64,
    /// RSI 레지스터
    pub rsi: u64,
    /// RDI 레지스터
    pub rdi: u64,
    /// RBP 레지스터 (베이스 포인터)
    pub rbp: u64,
    /// RDX 레지스터
    pub rdx: u64,
    /// RCX 레지스터
    pub rcx: u64,
    /// RBX 레지스터
    pub rbx: u64,
    /// RAX 레지스터
    pub rax: u64,
    /// RIP 레지스터 (명령 포인터)
    pub rip: u64,
    /// RSP 레지스터 (스택 포인터)
    pub rsp: u64,
    /// RFLAGS 레지스터
    pub rflags: u64,
}

impl ThreadContext {
    /// 새로운 빈 컨텍스트 생성
    pub fn new() -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            rip: 0,
            rsp: 0,
            rflags: 0x202, // 기본 RFLAGS 값 (IF 플래그 설정)
        }
    }

    /// 스택 포인터와 명령 포인터를 설정하여 초기 컨텍스트 생성
    ///
    /// # Arguments
    /// * `entry_point` - 스레드 진입점 주소
    /// * `stack_pointer` - 스레드 스택 포인터
    pub fn new_with_stack(entry_point: u64, stack_pointer: u64) -> Self {
        let mut ctx = Self::new();
        ctx.rip = entry_point;
        ctx.rsp = stack_pointer;
        ctx
    }
}

impl Default for ThreadContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 스레드 구조체
///
/// 각 스레드는 고유한 ID, 상태, 컨텍스트를 가집니다.
pub struct Thread {
    /// 스레드 ID
    pub id: u64,
    /// 스레드 상태
    pub state: ThreadState,
    /// CPU 컨텍스트
    pub context: ThreadContext,
    /// 스레드 이름 (디버깅용)
    pub name: &'static str,
}

impl Thread {
    /// 새로운 스레드 생성
    ///
    /// # Arguments
    /// * `id` - 스레드 ID
    /// * `name` - 스레드 이름
    /// * `entry_point` - 스레드 진입점 주소
    /// * `stack_pointer` - 스레드 스택 포인터
    pub fn new(id: u64, name: &'static str, entry_point: u64, stack_pointer: u64) -> Self {
        Self {
            id,
            state: ThreadState::Ready,
            context: ThreadContext::new_with_stack(entry_point, stack_pointer),
            name,
        }
    }

    /// 스레드 상태를 Ready로 변경
    pub fn set_ready(&mut self) {
        self.state = ThreadState::Ready;
    }

    /// 스레드 상태를 Running으로 변경
    pub fn set_running(&mut self) {
        self.state = ThreadState::Running;
    }

    /// 스레드 상태를 Blocked로 변경
    pub fn set_blocked(&mut self) {
        self.state = ThreadState::Blocked;
    }

    /// 스레드 상태를 Terminated로 변경
    pub fn set_terminated(&mut self) {
        self.state = ThreadState::Terminated;
    }
}

