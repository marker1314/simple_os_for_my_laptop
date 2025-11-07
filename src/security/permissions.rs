//! 파일 권한 및 접근 제어
//!
//! 이 모듈은 파일 권한 검사 및 접근 제어를 제공합니다.

use crate::fs::vfs::{FileMode, FileMetadata};
use crate::security::user::{UserId, GroupId, get_current_uid, get_current_gid, is_user_in_group};

/// 파일 접근 권한
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPermission {
    Read,
    Write,
    Execute,
}

/// 파일 접근 검사 결과
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessResult {
    /// 접근 허용
    Allowed,
    /// 접근 거부
    Denied,
}

/// 파일 권한 검사
/// 
/// Unix 스타일 권한 검사 (owner, group, other)
pub struct PermissionChecker;

impl PermissionChecker {
    /// 파일 접근 권한 검사
    /// 
    /// # Arguments
    /// * `metadata` - 파일 메타데이터
    /// * `permission` - 요청한 권한 (읽기/쓰기/실행)
    /// 
    /// # Returns
    /// 접근 허용 여부
    pub fn check_access(metadata: &FileMetadata, permission: AccessPermission) -> AccessResult {
        let current_uid = get_current_uid();
        let current_gid = get_current_gid();
        
        // 파일 소유자 및 그룹 가져오기
        // TODO: FileMetadata에 uid, gid 필드 추가 필요
        let file_uid = 0; // 임시: 파일 시스템에서 가져와야 함
        let file_gid = 0; // 임시: 파일 시스템에서 가져와야 함
        
        // Root 사용자는 항상 접근 허용
        if current_uid == 0 {
            return AccessResult::Allowed;
        }
        
        // 소유자 권한 검사
        if current_uid == file_uid {
            return Self::check_owner_permission(&metadata.mode, permission);
        }
        
        // 그룹 권한 검사
        if is_user_in_group(current_uid, file_gid) {
            return Self::check_group_permission(&metadata.mode, permission);
        }
        
        // 기타 사용자 권한 검사
        Self::check_other_permission(&metadata.mode, permission)
    }
    
    /// 소유자 권한 검사
    fn check_owner_permission(mode: &FileMode, permission: AccessPermission) -> AccessResult {
        match permission {
            AccessPermission::Read => {
                if mode.can_read() {
                    AccessResult::Allowed
                } else {
                    AccessResult::Denied
                }
            }
            AccessPermission::Write => {
                if mode.can_write() {
                    AccessResult::Allowed
                } else {
                    AccessResult::Denied
                }
            }
            AccessPermission::Execute => {
                if mode.can_execute() {
                    AccessResult::Allowed
                } else {
                    AccessResult::Denied
                }
            }
        }
    }
    
    /// 그룹 권한 검사
    fn check_group_permission(mode: &FileMode, permission: AccessPermission) -> AccessResult {
        // Unix 권한: 그룹 비트는 owner 비트의 다음 3비트
        // 현재 FileMode는 단순화되어 있으므로, 기본 권한 사용
        // TODO: Unix 스타일 권한 (rwxrwxrwx) 구현 필요
        Self::check_owner_permission(mode, permission)
    }
    
    /// 기타 사용자 권한 검사
    fn check_other_permission(mode: &FileMode, permission: AccessPermission) -> AccessResult {
        // Unix 권한: other 비트는 group 비트의 다음 3비트
        // 현재 FileMode는 단순화되어 있으므로, 기본 권한 사용
        // TODO: Unix 스타일 권한 (rwxrwxrwx) 구현 필요
        AccessResult::Denied // 기본적으로 거부
    }
    
    /// 파일 읽기 권한 검사
    pub fn check_read(metadata: &FileMetadata) -> AccessResult {
        Self::check_access(metadata, AccessPermission::Read)
    }
    
    /// 파일 쓰기 권한 검사
    pub fn check_write(metadata: &FileMetadata) -> AccessResult {
        Self::check_access(metadata, AccessPermission::Write)
    }
    
    /// 파일 실행 권한 검사
    pub fn check_execute(metadata: &FileMetadata) -> AccessResult {
        Self::check_access(metadata, AccessPermission::Execute)
    }
}

/// Unix 스타일 파일 권한 (rwxrwxrwx)
/// 
/// owner, group, other 각각에 대해 읽기/쓰기/실행 권한을 나타냅니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnixPermissions {
    /// Owner 권한 (rwx)
    owner: u8,
    /// Group 권한 (rwx)
    group: u8,
    /// Other 권한 (rwx)
    other: u8,
}

impl UnixPermissions {
    /// 새 권한 생성
    /// 
    /// # Arguments
    /// * `owner` - 소유자 권한 (rwx 비트: 0b111 = 7 = rwx)
    /// * `group` - 그룹 권한
    /// * `other` - 기타 사용자 권한
    pub fn new(owner: u8, group: u8, other: u8) -> Self {
        Self { owner, group, other }
    }
    
    /// 8진수 권한 문자열에서 생성 (예: 0o755)
    pub fn from_octal(mode: u32) -> Self {
        let owner = ((mode >> 6) & 0o7) as u8;
        let group = ((mode >> 3) & 0o7) as u8;
        let other = (mode & 0o7) as u8;
        Self { owner, group, other }
    }
    
    /// 8진수 권한으로 변환
    pub fn to_octal(&self) -> u32 {
        ((self.owner as u32) << 6) | ((self.group as u32) << 3) | (self.other as u32)
    }
    
    /// 소유자 읽기 권한
    pub fn owner_read(&self) -> bool {
        (self.owner & 0o4) != 0
    }
    
    /// 소유자 쓰기 권한
    pub fn owner_write(&self) -> bool {
        (self.owner & 0o2) != 0
    }
    
    /// 소유자 실행 권한
    pub fn owner_execute(&self) -> bool {
        (self.owner & 0o1) != 0
    }
    
    /// 그룹 읽기 권한
    pub fn group_read(&self) -> bool {
        (self.group & 0o4) != 0
    }
    
    /// 그룹 쓰기 권한
    pub fn group_write(&self) -> bool {
        (self.group & 0o2) != 0
    }
    
    /// 그룹 실행 권한
    pub fn group_execute(&self) -> bool {
        (self.group & 0o1) != 0
    }
    
    /// 기타 사용자 읽기 권한
    pub fn other_read(&self) -> bool {
        (self.other & 0o4) != 0
    }
    
    /// 기타 사용자 쓰기 권한
    pub fn other_write(&self) -> bool {
        (self.other & 0o2) != 0
    }
    
    /// 기타 사용자 실행 권한
    pub fn other_execute(&self) -> bool {
        (self.other & 0o1) != 0
    }
}

impl Default for UnixPermissions {
    fn default() -> Self {
        // 기본 권한: 0o644 (rw-r--r--)
        Self::from_octal(0o644)
    }
}

