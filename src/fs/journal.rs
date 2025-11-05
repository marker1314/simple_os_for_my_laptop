//! 파일시스템 저널링 (Journaling)
//!
//! 안전한 파일시스템 쓰기를 위한 저널 레이어 구현

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use crate::drivers::ata::{BlockDevice, BlockDeviceError};

/// 저널 엔트리 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalEntryType {
    /// 데이터 블록 쓰기
    DataWrite,
    /// 메타데이터 블록 쓰기
    MetadataWrite,
}

/// 저널 엔트리
#[derive(Clone)]
pub struct JournalEntry {
    /// 블록 번호
    pub block_num: u64,
    /// 블록 데이터
    pub data: [u8; 512],
    /// 엔트리 타입
    pub entry_type: JournalEntryType,
    /// 순서 번호
    pub sequence: u64,
}

/// 저널 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalState {
    /// 초기 상태
    Idle,
    /// 트랜잭션 진행 중
    Transaction,
    /// 커밋 중
    Committing,
    /// 체크포인트 중
    Checkpointing,
}

/// 저널 관리자
pub struct Journal {
    /// 저널 엔트리들
    entries: BTreeMap<u64, JournalEntry>,
    /// 현재 상태
    state: JournalState,
    /// 시퀀스 번호
    sequence: u64,
    /// 최대 저널 엔트리 수
    max_entries: usize,
}

impl Journal {
    /// 새 저널 생성
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            state: JournalState::Idle,
            sequence: 0,
            max_entries: 64, // 최대 64개 엔트리
        }
    }
    
    /// 트랜잭션 시작
    pub fn begin_transaction(&mut self) {
        // 이미 트랜잭션이 진행 중이면 무시 (중첩 트랜잭션 지원)
        if self.state == JournalState::Idle {
            self.state = JournalState::Transaction;
            self.sequence += 1;
        }
    }
    
    /// 트랜잭션이 진행 중인지 확인
    pub fn in_transaction(&self) -> bool {
        matches!(self.state, JournalState::Transaction | JournalState::Committing | JournalState::Checkpointing)
    }
    
    /// 저널 엔트리 추가
    pub fn add_entry(&mut self, block_num: u64, data: &[u8], entry_type: JournalEntryType) -> Result<(), &'static str> {
        if self.state != JournalState::Transaction {
            return Err("Not in transaction");
        }
        
        if self.entries.len() >= self.max_entries {
            return Err("Journal full");
        }
        
        let mut entry_data = [0u8; 512];
        let copy_len = data.len().min(512);
        entry_data[..copy_len].copy_from_slice(&data[..copy_len]);
        
        let entry = JournalEntry {
            block_num,
            data: entry_data,
            entry_type,
            sequence: self.sequence,
        };
        
        self.entries.insert(block_num, entry);
        Ok(())
    }
    
    /// 트랜잭션 커밋 (저널에 쓰기)
    pub fn commit(&mut self) -> Result<(), &'static str> {
        if self.state != JournalState::Transaction {
            return Err("Not in transaction");
        }
        
        self.state = JournalState::Committing;
        // 실제로는 저널 영역에 쓰기 필요
        // 여기서는 간단히 상태만 변경
        self.state = JournalState::Checkpointing;
        Ok(())
    }
    
    /// 체크포인트 (실제 파일시스템에 쓰기)
    pub fn checkpoint(&mut self) -> Result<(), &'static str> {
        if self.state != JournalState::Checkpointing {
            return Err("Not checkpointing");
        }
        
        // 실제로는 저널 엔트리를 실제 파일시스템에 쓰기
        // 여기서는 엔트리 클리어만 수행
        self.entries.clear();
        self.state = JournalState::Idle;
        Ok(())
    }
    
    /// 트랜잭션 롤백
    pub fn rollback(&mut self) {
        self.entries.clear();
        self.state = JournalState::Idle;
    }
    
    /// 저널 복구 (재부트 후)
    /// 
    /// # Arguments
    /// * `device` - 블록 디바이스 (저널 읽기용)
    /// * `journal_start_block` - 저널 영역 시작 블록
    pub fn recover(&mut self, device: Option<&dyn BlockDevice>, journal_start_block: Option<u64>) -> Result<Vec<JournalEntry>, &'static str> {
        // 디스크에서 저널 읽기
        if let (Some(dev), Some(start_block)) = (device, journal_start_block) {
            match self.read_journal_from_disk(dev, start_block) {
                Ok(entries) => {
                    if !entries.is_empty() {
                        crate::log_info!("Journal recovery: found {} entries to replay", entries.len());
                        self.entries.clear();
                        self.state = JournalState::Idle;
                        return Ok(entries);
                    }
                }
                Err(e) => {
                    crate::log_warn!("Journal recovery failed: {:?}", e);
                    // 계속 진행 (저널이 없거나 손상된 경우)
                }
            }
        }
        
        // 메모리 엔트리 반환 (없으면 빈 벡터)
        let entries: Vec<JournalEntry> = self.entries.values().cloned().collect();
        self.entries.clear();
        self.state = JournalState::Idle;
        Ok(entries)
    }
    
    /// 디스크에서 저널 읽기
    fn read_journal_from_disk(&self, device: &dyn BlockDevice, start_block: u64) -> Result<Vec<JournalEntry>, &'static str> {
        let mut header = [0u8; 512];
        unsafe {
            device.read_block(start_block, &mut header).map_err(|_| "Failed to read journal header")?;
        }
        
        // 매직 넘버 확인
        if &header[0..4] != b"JRNL" {
            return Err("Invalid journal magic");
        }
        
        // 커밋 상태 확인
        if header[16] != 1 {
            // 커밋되지 않은 저널 (부분적으로만 쓰여진 상태)
            return Err("Journal not committed");
        }
        
        // 엔트리 수 읽기
        let entry_count = u32::from_le_bytes([
            header[12], header[13], header[14], header[15]
        ]) as usize;
        
        let mut entries = Vec::new();
        let mut block_offset = 1;
        
        // 엔트리 읽기
        for _ in 0..entry_count {
            let mut entry_header = [0u8; 512];
            unsafe {
                device.read_block(start_block + block_offset, &mut entry_header)
                    .map_err(|_| "Failed to read journal entry header")?;
                block_offset += 1;
            }
            
            // 블록 번호
            let block_num = u64::from_le_bytes([
                entry_header[0], entry_header[1], entry_header[2], entry_header[3],
                entry_header[4], entry_header[5], entry_header[6], entry_header[7],
            ]);
            
            // 타입
            let entry_type = match entry_header[8] {
                0 => JournalEntryType::DataWrite,
                1 => JournalEntryType::MetadataWrite,
                _ => return Err("Invalid entry type"),
            };
            
            // 시퀀스
            let sequence = u64::from_le_bytes([
                entry_header[9], entry_header[10], entry_header[11], entry_header[12],
                entry_header[13], entry_header[14], entry_header[15], entry_header[16],
            ]);
            
            // 데이터 읽기
            let mut data = [0u8; 512];
            unsafe {
                device.read_block(start_block + block_offset, &mut data)
                    .map_err(|_| "Failed to read journal entry data")?;
                block_offset += 1;
            }
            
            entries.push(JournalEntry {
                block_num,
                data,
                entry_type,
                sequence,
            });
        }
        
        // 커밋 마커 확인
        let mut commit_marker = [0u8; 512];
        unsafe {
            device.read_block(start_block + block_offset, &mut commit_marker)
                .map_err(|_| "Failed to read commit marker")?;
        }
        
        if &commit_marker[0..4] != b"COMM" {
            return Err("Journal commit marker not found");
        }
        
        Ok(entries)
    }
    
    /// 현재 상태
    pub fn state(&self) -> JournalState {
        self.state
    }
}

/// 전역 저널 인스턴스
static GLOBAL_JOURNAL: Mutex<Journal> = Mutex::new(Journal {
    entries: BTreeMap::new(),
    state: JournalState::Idle,
    sequence: 0,
    max_entries: 64,
});

/// 저널 초기화
pub fn init() {
    crate::log_info!("Journal initialized");
}

/// 트랜잭션 시작
pub fn begin_transaction() {
    GLOBAL_JOURNAL.lock().begin_transaction();
}

/// 트랜잭션이 진행 중인지 확인
pub fn in_transaction() -> bool {
    GLOBAL_JOURNAL.lock().in_transaction()
}

/// 저널 엔트리 추가
pub fn add_entry(block_num: u64, data: &[u8], entry_type: JournalEntryType) -> Result<(), &'static str> {
    GLOBAL_JOURNAL.lock().add_entry(block_num, data, entry_type)
}

/// 트랜잭션 커밋
/// 
/// # Arguments
/// * `device` - 블록 디바이스 (저널 영역에 쓰기용)
/// * `journal_start_block` - 저널 영역 시작 블록
pub fn commit(device: Option<&dyn BlockDevice>, journal_start_block: Option<u64>) -> Result<(), &'static str> {
    GLOBAL_JOURNAL.lock().commit(device, journal_start_block)
}

/// 체크포인트
/// 
/// # Arguments
/// * `device` - 블록 디바이스 (파일시스템에 쓰기용)
pub fn checkpoint(device: Option<&dyn BlockDevice>) -> Result<(), &'static str> {
    GLOBAL_JOURNAL.lock().checkpoint(device)
}

/// 트랜잭션 롤백
pub fn rollback() {
    GLOBAL_JOURNAL.lock().rollback();
}

/// 저널 복구
/// 
/// # Arguments
/// * `device` - 블록 디바이스 (저널 읽기용)
/// * `journal_start_block` - 저널 영역 시작 블록
pub fn recover(device: Option<&dyn BlockDevice>, journal_start_block: Option<u64>) -> Result<Vec<JournalEntry>, &'static str> {
    GLOBAL_JOURNAL.lock().recover(device, journal_start_block)
}

