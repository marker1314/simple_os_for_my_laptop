//! Round-Robin 스케줄러 구현
//!
//! 이 모듈은 간단한 Round-Robin 스케줄링 알고리즘을 구현합니다.
//! 각 스레드는 시간 할당량(time quantum) 동안 실행되고, 그 후 다음 스레드로 전환됩니다.

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use crate::scheduler::thread::{Thread, ThreadState, ThreadPriority};

/// Round-Robin 스케줄러 (우선순위 지원)
///
/// 준비 큐(ready queue)를 사용하여 스레드를 순환적으로 스케줄링합니다.
/// 우선순위가 높은 스레드가 먼저 실행됩니다.
pub struct RoundRobinScheduler {
    /// 준비 큐 (우선순위별로 분리)
    ready_queues: [VecDeque<Arc<Mutex<Thread>>>; 4], // Priority 0-3
    /// 현재 실행 중인 스레드
    current_thread: Option<Arc<Mutex<Thread>>>,
    /// 다음 스레드 ID
    next_thread_id: u64,
    /// 시간 할당량 (타이머 틱 수, 우선순위별)
    time_quantum: [u32; 4], // Priority별 시간 할당량
    /// 현재 실행 시간 (타이머 틱 수)
    current_time: u32,
}

impl RoundRobinScheduler {
    /// 새로운 Round-Robin 스케줄러 생성
    ///
    /// # Arguments
    /// * `time_quantum` - 기본 시간 할당량 (타이머 틱 수)
    pub fn new(time_quantum: u32) -> Self {
        // 우선순위별 시간 할당량 설정
        // 높은 우선순위일수록 더 많은 시간 할당
        let time_quantums = [
            time_quantum / 2,      // Low: 기본의 절반
            time_quantum,           // Normal: 기본
            time_quantum * 2,       // High: 기본의 2배
            time_quantum * 4,       // Realtime: 기본의 4배
        ];
        
        Self {
            ready_queues: [
                VecDeque::new(),  // Low
                VecDeque::new(),  // Normal
                VecDeque::new(),  // High
                VecDeque::new(),  // Realtime
            ],
            current_thread: None,
            next_thread_id: 1,
            time_quantum: time_quantums,
            current_time: 0,
        }
    }

    /// 스레드를 스케줄러에 추가
    ///
    /// # Arguments
    /// * `thread` - 추가할 스레드
    pub fn add_thread(&mut self, thread: Arc<Mutex<Thread>>) {
        let priority = {
            let mut t = thread.lock();
            t.set_ready();
            
            // OOM Killer 통계 업데이트
            let total_memory = t.stack_size + (t.allocated_frames_len() * 4096);
            crate::memory::oom_killer::update_thread_memory(
                t.id,
                0, // 힙 사용량은 별도로 추적 필요
                t.stack_size,
                t.allocated_frames_len(),
            );
            
            t.priority
        };
        
        // 우선순위별 큐에 추가
        let priority_index = priority.to_u8() as usize;
        if priority_index < 4 {
            self.ready_queues[priority_index].push_back(thread);
        } else {
            // 잘못된 우선순위는 Normal로 처리
            self.ready_queues[ThreadPriority::Normal.to_u8() as usize].push_back(thread);
        }
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
        
        // 현재 스레드의 우선순위에 따른 시간 할당량 확인
        let current_quantum = if let Some(current) = &self.current_thread {
            let thread = current.lock();
            let priority_index = thread.priority.to_u8() as usize;
            if priority_index < 4 {
                self.time_quantum[priority_index]
            } else {
                self.time_quantum[ThreadPriority::Normal.to_u8() as usize]
            }
        } else {
            self.time_quantum[ThreadPriority::Normal.to_u8() as usize]
        };

        // 시간 할당량이 만료되었는지 확인
        if self.current_time >= current_quantum {
            self.current_time = 0;
            return self.switch_to_next();
        }
        
        // 우선순위가 높은 스레드가 준비되었는지 확인 (선점)
        if let Some(current) = &self.current_thread {
            let current_priority = {
                let thread = current.lock();
                thread.priority
            };
            
            // 더 높은 우선순위의 스레드가 있는지 확인
            for priority in (current_priority.to_u8() as usize + 1)..4 {
                if !self.ready_queues[priority].is_empty() {
                    // 선점 필요
                    self.current_time = 0;
                    return self.switch_to_next();
                }
            }
        }

        false
    }

    /// 다음 스레드로 전환
    ///
    /// 현재 스레드를 준비 큐의 뒤로 이동하고, 다음 스레드를 실행합니다.
    /// 우선순위가 높은 스레드부터 선택합니다.
    ///
    /// # Returns
    /// 컨텍스트 스위칭이 필요한 경우 `true`, 그렇지 않으면 `false`
    pub fn switch_to_next(&mut self) -> bool {
        // 현재 스레드가 있다면 준비 큐로 이동
        if let Some(current) = self.current_thread.take() {
            let priority_index = {
                let mut thread = current.lock();
                if thread.state == ThreadState::Running {
                    // 스택 카나리 검증 (오버플로우 감지)
                    if let Err(e) = thread.verify_canary() {
                        crate::log_error!("Stack canary corruption detected in thread {} ({}): {}", 
                                        thread.id, thread.name, e);
                        crate::crash::record_exception(thread.context.rip, 0x0E); // Page Fault로 기록
                        thread.set_terminated();
                        drop(thread);
                        // 손상된 스레드는 큐에 추가하지 않음
                        // 다음 스레드 선택으로 계속
                    } else {
                        thread.set_ready();
                        let priority = thread.priority;
                        drop(thread);
                        
                        // 우선순위별 큐에 다시 추가
                        let idx = priority.to_u8() as usize;
                        if idx < 4 {
                            self.ready_queues[idx].push_back(current);
                            return self.select_next_thread();
                        }
                    }
                }
                // 우선순위 인덱스 (실제로는 사용되지 않음)
                0
            };
        }

        self.select_next_thread()
    }
    
    /// 다음 스레드 선택 (우선순위 기반)
    fn select_next_thread(&mut self) -> bool {
        // 우선순위가 높은 큐부터 확인
        for priority_idx in (0..4).rev() {
            if let Some(next) = self.ready_queues[priority_idx].pop_front() {
                next.lock().set_running();
                self.current_thread = Some(next);
                // 컨텍스트 스위칭 메트릭 기록
                crate::monitoring::record_context_switch();
                return true;
            }
        }
        
        false
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
        let priority = {
            let mut t = thread.lock();
            if t.state == ThreadState::Blocked {
                t.set_ready();
                t.priority
            } else {
                drop(t);
                return;
            }
        };
        
        // 우선순위별 큐에 추가
        let priority_index = priority.to_u8() as usize;
        if priority_index < 4 {
            self.ready_queues[priority_index].push_back(thread);
        }
    }

    /// 스레드 종료
    ///
    /// 스레드를 종료하고 다음 스레드로 전환합니다.
    ///
    /// # Arguments
    /// * `thread_id` - 종료할 스레드 ID
    pub fn terminate_thread(&mut self, thread_id: u64) {
        // 현재 실행 중인 스레드 확인
        if let Some(current) = &self.current_thread {
            let mut thread = current.lock();
            if thread.id == thread_id {
                // 리소스 정리
                thread.cleanup();
                drop(thread);
                self.current_thread = None;
                self.current_time = 0;
                self.switch_to_next();
                return;
            }
        }
        
        // 준비 큐에서 찾기 (모든 우선순위 큐 검색)
        let mut found = false;
        for queue in &mut self.ready_queues {
            queue.retain(|t| {
                let mut thread = t.lock();
                if thread.id == thread_id {
                    thread.cleanup();
                    found = true;
                    false // 제거
                } else {
                    true // 유지
                }
            });
            if found {
                break;
            }
        }
        
        if found {
            crate::log_info!("Thread {} terminated and removed from ready queue", thread_id);
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
        self.ready_queues.iter().map(|q| q.len()).sum()
    }
    
    /// 우선순위 설정
    ///
    /// # Arguments
    /// * `thread_id` - 스레드 ID
    /// * `priority` - 새로운 우선순위
    pub fn set_thread_priority(&mut self, thread_id: u64, priority: ThreadPriority) -> bool {
        // 현재 스레드 확인
        if let Some(current) = &self.current_thread {
            let mut thread = current.lock();
            if thread.id == thread_id {
                thread.priority = priority;
                return true;
            }
        }
        
        // 준비 큐에서 찾기
        for queue in &mut self.ready_queues {
            for thread_arc in queue.iter() {
                let mut thread = thread_arc.lock();
                if thread.id == thread_id {
                    thread.priority = priority;
                    return true;
                }
            }
        }
        
        false
    }

    /// 현재 실행 중인 스레드가 있는지 확인
    pub fn has_current_thread(&self) -> bool {
        self.current_thread.is_some()
    }
}

