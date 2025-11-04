//! Round-Robin 스케줄러 구현
//!
//! 이 모듈은 간단한 Round-Robin 스케줄링 알고리즘을 구현합니다.
//! 각 스레드는 시간 할당량(time quantum) 동안 실행되고, 그 후 다음 스레드로 전환됩니다.

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use crate::scheduler::thread::{Thread, ThreadState};

/// Round-Robin 스케줄러
///
/// 준비 큐(ready queue)를 사용하여 스레드를 순환적으로 스케줄링합니다.
pub struct RoundRobinScheduler {
    /// 준비 큐 (Ready Queue)
    ready_queue: VecDeque<Arc<Mutex<Thread>>>,
    /// 현재 실행 중인 스레드
    current_thread: Option<Arc<Mutex<Thread>>>,
    /// 다음 스레드 ID
    next_thread_id: u64,
    /// 시간 할당량 (타이머 틱 수)
    time_quantum: u32,
    /// 현재 실행 시간 (타이머 틱 수)
    current_time: u32,
}

impl RoundRobinScheduler {
    /// 새로운 Round-Robin 스케줄러 생성
    ///
    /// # Arguments
    /// * `time_quantum` - 각 스레드의 시간 할당량 (타이머 틱 수)
    pub fn new(time_quantum: u32) -> Self {
        Self {
            ready_queue: VecDeque::new(),
            current_thread: None,
            next_thread_id: 1,
            time_quantum,
            current_time: 0,
        }
    }

    /// 스레드를 스케줄러에 추가
    ///
    /// # Arguments
    /// * `thread` - 추가할 스레드
    pub fn add_thread(&mut self, thread: Arc<Mutex<Thread>>) {
        thread.lock().set_ready();
        self.ready_queue.push_back(thread);
    }

    /// 현재 실행 중인 스레드 가져오기
    pub fn current_thread(&self) -> Option<Arc<Mutex<Thread>>> {
        self.current_thread.as_ref().map(|t| Arc::clone(t))
    }

    /// 타이머 틱 처리
    ///
    /// 시간 할당량이 만료되면 다음 스레드로 전환합니다.
    ///
    /// # Returns
    /// 컨텍스트 스위칭이 필요한 경우 `true`, 그렇지 않으면 `false`
    pub fn tick(&mut self) -> bool {
        self.current_time += 1;

        // 시간 할당량이 만료되었는지 확인
        if self.current_time >= self.time_quantum {
            self.current_time = 0;
            return self.switch_to_next();
        }

        false
    }

    /// 다음 스레드로 전환
    ///
    /// 현재 스레드를 준비 큐의 뒤로 이동하고, 다음 스레드를 실행합니다.
    ///
    /// # Returns
    /// 컨텍스트 스위칭이 필요한 경우 `true`, 그렇지 않으면 `false`
    pub fn switch_to_next(&mut self) -> bool {
        // 현재 스레드가 있다면 준비 큐로 이동
        if let Some(current) = self.current_thread.take() {
            let mut thread = current.lock();
            if thread.state == ThreadState::Running {
                thread.set_ready();
                drop(thread);
                self.ready_queue.push_back(current);
            }
        }

        // 다음 스레드 가져오기
        if let Some(next) = self.ready_queue.pop_front() {
            next.lock().set_running();
            self.current_thread = Some(next);
            true
        } else {
            false
        }
    }

    /// 스레드 블로킹
    ///
    /// 현재 실행 중인 스레드를 블로킹하고 다음 스레드로 전환합니다.
    ///
    /// # Arguments
    /// * `thread_id` - 블로킹할 스레드 ID
    pub fn block_thread(&mut self, thread_id: u64) {
        if let Some(current) = &self.current_thread {
            let mut thread = current.lock();
            if thread.id == thread_id {
                thread.set_blocked();
                drop(thread);
                self.current_thread = None;
                self.current_time = 0;
                self.switch_to_next();
            }
        }
    }

    /// 스레드 언블로킹
    ///
    /// 블로킹된 스레드를 준비 큐에 다시 추가합니다.
    ///
    /// # Arguments
    /// * `thread` - 언블로킹할 스레드
    pub fn unblock_thread(&mut self, thread: Arc<Mutex<Thread>>) {
        let mut t = thread.lock();
        if t.state == ThreadState::Blocked {
            t.set_ready();
            drop(t);
            self.ready_queue.push_back(thread);
        }
    }

    /// 스레드 종료
    ///
    /// 스레드를 종료하고 다음 스레드로 전환합니다.
    ///
    /// # Arguments
    /// * `thread_id` - 종료할 스레드 ID
    pub fn terminate_thread(&mut self, thread_id: u64) {
        if let Some(current) = &self.current_thread {
            let mut thread = current.lock();
            if thread.id == thread_id {
                thread.set_terminated();
                drop(thread);
                self.current_thread = None;
                self.current_time = 0;
                self.switch_to_next();
            }
        }
    }

    /// 다음 스레드 ID 생성
    pub fn allocate_thread_id(&mut self) -> u64 {
        let id = self.next_thread_id;
        self.next_thread_id += 1;
        id
    }

    /// 준비 큐의 스레드 수 반환
    pub fn ready_count(&self) -> usize {
        self.ready_queue.len()
    }

    /// 현재 실행 중인 스레드가 있는지 확인
    pub fn has_current_thread(&self) -> bool {
        self.current_thread.is_some()
    }
}

