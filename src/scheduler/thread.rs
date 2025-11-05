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
    /// 스택 시작 주소 (낮은 주소)
    pub stack_start: Option<u64>,
    /// 스택 크기 (바이트)
    pub stack_size: usize,
    /// 동적으로 할당된 프레임 목록 (해제 시 사용)
    allocated_frames: alloc::vec::Vec<x86_64::structures::paging::PhysFrame<x86_64::structures::paging::Size4KiB>>,
    /// 동적 스택 할당 여부
    dynamic_stack: bool,
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
        // 스택 크기 계산 (기본 8KB)
        const DEFAULT_STACK_SIZE: usize = 8 * 1024;
        let stack_start = stack_pointer.saturating_sub(DEFAULT_STACK_SIZE as u64);
        
        Self {
            id,
            state: ThreadState::Ready,
            context: ThreadContext::new_with_stack(entry_point, stack_pointer),
            name,
            stack_start: Some(stack_start),
            stack_size: DEFAULT_STACK_SIZE,
            allocated_frames: alloc::vec::Vec::new(),
            dynamic_stack: false, // 정적 스택
        }
    }
    
    /// 스택 정보로 스레드 생성
    pub fn new_with_stack(id: u64, name: &'static str, entry_point: u64, stack_start: u64, stack_size: usize) -> Self {
        let stack_pointer = stack_start + stack_size as u64;
        
        // Guard page 자동 생성
        unsafe {
            if let Err(e) = crate::memory::guard::auto_create_stack_guard(stack_start, stack_size) {
                crate::log_warn!("Failed to create guard pages for thread {}: {}", id, e);
            }
        }
        
        Self {
            id,
            state: ThreadState::Ready,
            context: ThreadContext::new_with_stack(entry_point, stack_pointer),
            name,
            stack_start: Some(stack_start),
            stack_size,
            allocated_frames: alloc::vec::Vec::new(),
            dynamic_stack: false, // 기본값: 정적 스택
        }
    }
    
    /// 동적 스택으로 스레드 생성 (프레임 할당)
    ///
    /// # Safety
    /// 메모리 관리가 초기화된 후에 호출되어야 합니다.
    pub unsafe fn new_with_dynamic_stack(id: u64, name: &'static str, entry_point: u64, stack_size: usize) -> Option<Self> {
        use x86_64::VirtAddr;
        use x86_64::structures::paging::{Page, Size4KiB};
        use crate::memory::frame::allocate_frame;
        use crate::memory::paging::map_zero_page_at;
        
        // 필요한 페이지 수 계산
        let pages_needed = (stack_size + Size4KiB::SIZE as usize - 1) / Size4KiB::SIZE as usize;
        
        // 스택 영역 할당 (낮은 주소에서 시작)
        let stack_start_virt = VirtAddr::new(0x7000_0000_0000); // 사용자 스택 영역
        
        let mut allocated_frames = alloc::vec::Vec::new();
        
        // 각 페이지에 대해 프레임 할당 및 매핑
        for i in 0..pages_needed {
            let frame = allocate_frame()?;
            allocated_frames.push(frame);
            
            let page_addr = stack_start_virt + (i * Size4KiB::SIZE) as u64;
            let page = Page::<Size4KiB>::containing_address(page_addr);
            
            // 페이지 매핑 (실제로는 map_zero_page_at 사용)
            if let Err(e) = map_zero_page_at(page.start_address()) {
                crate::log_error!("Failed to map stack page for thread {}: {:?}", id, e);
                // 실패 시 이미 할당한 프레임 해제
                for frame in allocated_frames {
                    crate::memory::frame::deallocate_frame(frame);
                }
                return None;
            }
        }
        
        let stack_start = stack_start_virt.as_u64();
        let stack_pointer = stack_start + stack_size as u64;
        
        // Guard page 생성
        if let Err(e) = crate::memory::guard::auto_create_stack_guard(stack_start, stack_size) {
            crate::log_warn!("Failed to create guard pages for thread {}: {}", id, e);
        }
        
        Some(Self {
            id,
            state: ThreadState::Ready,
            context: ThreadContext::new_with_stack(entry_point, stack_pointer),
            name,
            stack_start: Some(stack_start),
            stack_size,
            allocated_frames,
            dynamic_stack: true,
        })
    }
    
    /// 프레임 할당 기록 (동적 할당 시)
    pub fn track_frame(&mut self, frame: x86_64::structures::paging::PhysFrame<x86_64::structures::paging::Size4KiB>) {
        self.allocated_frames.push(frame);
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
    
    /// 스레드 리소스 정리
    /// 스레드 종료 시 호출되어 메모리 및 리소스를 해제합니다
    pub fn cleanup(&mut self) {
        use x86_64::VirtAddr;
        use x86_64::structures::paging::{Page, Size4KiB, Mapper};
        use crate::memory::paging::init_mapper;
        use crate::memory::paging::get_physical_memory_offset;
        use crate::boot::get_boot_info;
        
        // 동적 스택 해제
        if self.dynamic_stack {
            if let Some(stack_start) = self.stack_start {
                unsafe {
                    // 부트 정보에서 물리 메모리 오프셋 가져오기
                    if let Some(boot_info) = get_boot_info() {
                        let phys_offset = get_physical_memory_offset(boot_info);
                        let mut mapper = init_mapper(phys_offset);
                        
                        // 스택 페이지 언맵
                        let pages_needed = (self.stack_size + Size4KiB::SIZE as usize - 1) / Size4KiB::SIZE as usize;
                        for i in 0..pages_needed {
                            let page_addr = VirtAddr::new(stack_start + (i * Size4KiB::SIZE) as u64);
                            let page = Page::<Size4KiB>::containing_address(page_addr);
                            
                            // 페이지 언맵 시도
                            if let Ok((frame, _)) = mapper.unmap(page) {
                                frame.flush();
                                // 프레임 해제
                                crate::memory::frame::deallocate_frame(frame);
                            }
                        }
                        
                        crate::log_info!("Thread {} ({}) dynamic stack freed", self.id, self.name);
                    }
                }
            }
        }
        
        // 할당된 프레임 해제
        for frame in self.allocated_frames.drain(..) {
            crate::memory::frame::deallocate_frame(frame);
        }
        
        // 파일 디스크립터 닫기는 향후 구현
        // 현재는 파일 시스템이 단순하므로 파일 디스크립터 추적 없음
        
        // 잠금 해제 확인
        // 스레드가 보유한 잠금이 있다면 해제해야 함
        // 현재는 단순한 구조이므로 특별한 정리 불필요
        
        crate::log_info!("Thread {} ({}) resources cleaned up", self.id, self.name);
        
        // 상태를 Terminated로 설정
        self.set_terminated();
    }
}

