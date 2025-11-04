//! 프로세스 및 스레드 스케줄러 모듈
//!
//! 이 모듈은 프로세스와 스레드의 스케줄링을 담당합니다.

pub mod thread;
pub mod round_robin;
pub mod context_switch;

use alloc::sync::Arc;
use spin::Mutex;
use thread::Thread;
use round_robin::RoundRobinScheduler;

/// 전역 스케줄러 인스턴스
static SCHEDULER: Mutex<Option<RoundRobinScheduler>> = Mutex::new(None);

/// 스케줄러 초기화
///
/// # Arguments
/// * `time_quantum` - 각 스레드의 시간 할당량 (타이머 틱 수, 기본값: 10)
pub fn init(time_quantum: u32) {
    let mut scheduler = SCHEDULER.lock();
    *scheduler = Some(RoundRobinScheduler::new(time_quantum));
    crate::log_info!("Scheduler initialized with time quantum: {} ticks", time_quantum);
}

/// 스레드를 스케줄러에 추가
///
/// # Arguments
/// * `thread` - 추가할 스레드
pub fn add_thread(thread: Arc<Mutex<Thread>>) {
    let mut scheduler = SCHEDULER.lock();
    if let Some(ref mut sched) = *scheduler {
        sched.add_thread(thread);
    }
}

/// 타이머 틱 처리
///
/// 시간 할당량이 만료되면 다음 스레드로 전환합니다.
///
/// # Returns
/// 컨텍스트 스위칭이 필요한 경우 `true`, 그렇지 않으면 `false`
pub fn tick() -> bool {
    let mut scheduler = SCHEDULER.lock();
    if let Some(ref mut sched) = *scheduler {
        sched.tick()
    } else {
        false
    }
}

/// 스레드 블로킹
///
/// # Arguments
/// * `thread_id` - 블로킹할 스레드 ID
pub fn block_thread(thread_id: u64) {
    let mut scheduler = SCHEDULER.lock();
    if let Some(ref mut sched) = *scheduler {
        sched.block_thread(thread_id);
    }
}

/// 스레드 언블로킹
///
/// # Arguments
/// * `thread` - 언블로킹할 스레드
pub fn unblock_thread(thread: Arc<Mutex<Thread>>) {
    let mut scheduler = SCHEDULER.lock();
    if let Some(ref mut sched) = *scheduler {
        sched.unblock_thread(thread);
    }
}

/// 스레드 종료
///
/// # Arguments
/// * `thread_id` - 종료할 스레드 ID
pub fn terminate_thread(thread_id: u64) {
    let mut scheduler = SCHEDULER.lock();
    if let Some(ref mut sched) = *scheduler {
        sched.terminate_thread(thread_id);
    }
}

/// 다음 스레드 ID 할당
pub fn allocate_thread_id() -> u64 {
    let mut scheduler = SCHEDULER.lock();
    if let Some(ref mut sched) = *scheduler {
        sched.allocate_thread_id()
    } else {
        0
    }
}

/// 현재 실행 중인 스레드 가져오기
pub fn current_thread() -> Option<Arc<Mutex<Thread>>> {
    let scheduler = SCHEDULER.lock();
    if let Some(ref sched) = *scheduler {
        sched.current_thread()
    } else {
        None
    }
}

/// 준비 큐의 스레드 수 반환
pub fn ready_count() -> usize {
    let scheduler = SCHEDULER.lock();
    if let Some(ref sched) = *scheduler {
        sched.ready_count()
    } else {
        0
    }
}

