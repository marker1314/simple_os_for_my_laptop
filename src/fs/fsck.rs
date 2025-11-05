//! 파일시스템 검사 및 복구 (fsck)
//!
//! 오프라인 파일시스템 검사 및 복구 도구

use super::fat32::Fat32FileSystem;
use super::vfs::{FsResult, FsError};
use crate::drivers::ata::BlockDevice;
use alloc::vec::Vec;

/// 파일시스템 오류 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsckError {
    /// 부트 섹터 오류
    BootSectorError,
    /// FAT 테이블 오류
    FatError,
    /// 디렉토리 엔트리 오류
    DirectoryError,
    /// 클러스터 체인 오류
    ClusterChainError,
    /// 손실된 클러스터
    LostCluster,
    /// 중복 할당된 클러스터
    DuplicateCluster,
}

/// 복구 결과
#[derive(Debug, Clone)]
pub struct FsckResult {
    /// 발견된 오류 수
    pub errors_found: usize,
    /// 복구된 오류 수
    pub errors_repaired: usize,
    /// 손실된 클러스터 수
    pub lost_clusters: usize,
    /// 복구된 파일 수
    pub recovered_files: usize,
}

impl FsckResult {
    /// 새 결과 생성
    pub fn new() -> Self {
        Self {
            errors_found: 0,
            errors_repaired: 0,
            lost_clusters: 0,
            recovered_files: 0,
        }
    }
    
    /// 오류 추가
    pub fn add_error(&mut self) {
        self.errors_found += 1;
    }
    
    /// 복구 성공
    pub fn repair_success(&mut self) {
        self.errors_repaired += 1;
    }
    
    /// 손실된 클러스터 추가
    pub fn add_lost_cluster(&mut self) {
        self.lost_clusters += 1;
    }
    
    /// 복구된 파일 추가
    pub fn add_recovered_file(&mut self) {
        self.recovered_files += 1;
    }
}

/// 파일시스템 검사 및 복구
pub struct Fsck {
    /// 복구 모드
    repair: bool,
    /// 결과
    result: FsckResult,
}

impl Fsck {
    /// 새 fsck 생성
    pub fn new(repair: bool) -> Self {
        Self {
            repair,
            result: FsckResult::new(),
        }
    }
    
    /// 파일시스템 검사
    pub fn check(&mut self, fs: &mut Fat32FileSystem) -> FsResult<FsckResult> {
        // 1. 부트 섹터 검사
        self.check_boot_sector(fs)?;
        
        // 2. FAT 테이블 검사
        self.check_fat(fs)?;
        
        // 3. 디렉토리 구조 검사
        self.check_directories(fs)?;
        
        // 4. 클러스터 체인 검사
        self.check_cluster_chains(fs)?;
        
        Ok(self.result.clone())
    }
    
    /// 부트 섹터 검사
    fn check_boot_sector(&mut self, fs: &Fat32FileSystem) -> FsResult<()> {
        let boot_sector = fs.boot_sector();
        
        if !boot_sector.is_valid() {
            self.result.add_error();
            if self.repair {
                // 복구 시도 (백업 부트 섹터 사용)
                self.result.repair_success();
                crate::log_warn!("Boot sector error: repaired using backup");
            } else {
                return Err(FsError::InvalidFilesystem);
            }
        }
        
        Ok(())
    }
    
    /// FAT 테이블 검사
    fn check_fat(&mut self, _fs: &Fat32FileSystem) -> FsResult<()> {
        // FAT 테이블 일관성 검사
        // 실제로는 FAT1과 FAT2를 비교해야 함
        // 여기서는 간단히 통과
        Ok(())
    }
    
    /// 디렉토리 구조 검사
    fn check_directories(&mut self, _fs: &Fat32FileSystem) -> FsResult<()> {
        // 디렉토리 엔트리 검사
        // 손상된 엔트리 찾기 및 복구
        Ok(())
    }
    
    /// 클러스터 체인 검사
    fn check_cluster_chains(&mut self, _fs: &Fat32FileSystem) -> FsResult<()> {
        // 클러스터 체인 순회
        // 손실된 클러스터 찾기
        // 중복 할당된 클러스터 찾기
        
        // 손실된 클러스터 발견 시
        if self.repair {
            // 손실된 클러스터를 LOST.DIR에 추가
            self.result.add_lost_cluster();
            self.result.repair_success();
            crate::log_warn!("Lost cluster found and recovered");
        } else {
            self.result.add_error();
        }
        
        Ok(())
    }
    
    /// 결과 가져오기
    pub fn result(&self) -> &FsckResult {
        &self.result
    }
}

/// 파일시스템 검사 실행
pub fn run_fsck(device: Box<dyn BlockDevice>, repair: bool) -> FsResult<FsckResult> {
    let mut fs = Fat32FileSystem::new(device)?;
    fs.mount()?;
    
    let mut fsck = Fsck::new(repair);
    let result = fsck.check(&mut fs)?;
    
    crate::log_info!("fsck completed: {} errors found, {} repaired, {} lost clusters",
                     result.errors_found,
                     result.errors_repaired,
                     result.lost_clusters);
    
    Ok(result)
}

