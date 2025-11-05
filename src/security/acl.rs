//! 접근 제어 리스트 (ACL)
//!
//! 이 모듈은 파일 및 리소스에 대한 세밀한 접근 제어를 제공합니다.

use crate::security::user::{UserId, GroupId};
use alloc::vec::Vec;
use alloc::collections::BTreeSet;

/// ACL 엔트리
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AclEntry {
    /// 사용자 ID (None이면 그룹)
    pub uid: Option<UserId>,
    /// 그룹 ID (None이면 사용자)
    pub gid: Option<GroupId>,
    /// 읽기 권한
    pub read: bool,
    /// 쓰기 권한
    pub write: bool,
    /// 실행 권한
    pub execute: bool,
}

impl AclEntry {
    /// 새 ACL 엔트리 생성 (사용자)
    pub fn new_user(uid: UserId, read: bool, write: bool, execute: bool) -> Self {
        Self {
            uid: Some(uid),
            gid: None,
            read,
            write,
            execute,
        }
    }
    
    /// 새 ACL 엔트리 생성 (그룹)
    pub fn new_group(gid: GroupId, read: bool, write: bool, execute: bool) -> Self {
        Self {
            uid: None,
            gid: Some(gid),
            read,
            write,
            execute,
        }
    }
}

/// 접근 제어 리스트
#[derive(Debug, Clone)]
pub struct AccessControlList {
    /// ACL 엔트리 목록
    entries: Vec<AclEntry>,
}

impl AccessControlList {
    /// 새 ACL 생성
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    
    /// ACL 엔트리 추가
    pub fn add_entry(&mut self, entry: AclEntry) {
        self.entries.push(entry);
    }
    
    /// 사용자 접근 권한 확인
    pub fn check_access(&self, uid: UserId, gid: GroupId, permission: crate::security::permissions::AccessPermission) -> crate::security::permissions::AccessResult {
        use crate::security::permissions::AccessPermission;
        
        // 엔트리 검색
        for entry in &self.entries {
            // 사용자 매칭
            if let Some(entry_uid) = entry.uid {
                if entry_uid == uid {
                    return match permission {
                        AccessPermission::Read => {
                            if entry.read {
                                crate::security::permissions::AccessResult::Allowed
                            } else {
                                crate::security::permissions::AccessResult::Denied
                            }
                        }
                        AccessPermission::Write => {
                            if entry.write {
                                crate::security::permissions::AccessResult::Allowed
                            } else {
                                crate::security::permissions::AccessResult::Denied
                            }
                        }
                        AccessPermission::Execute => {
                            if entry.execute {
                                crate::security::permissions::AccessResult::Allowed
                            } else {
                                crate::security::permissions::AccessResult::Denied
                            }
                        }
                    };
                }
            }
            
            // 그룹 매칭
            if let Some(entry_gid) = entry.gid {
                if entry_gid == gid {
                    return match permission {
                        AccessPermission::Read => {
                            if entry.read {
                                crate::security::permissions::AccessResult::Allowed
                            } else {
                                crate::security::permissions::AccessResult::Denied
                            }
                        }
                        AccessPermission::Write => {
                            if entry.write {
                                crate::security::permissions::AccessResult::Allowed
                            } else {
                                crate::security::permissions::AccessResult::Denied
                            }
                        }
                        AccessPermission::Execute => {
                            if entry.execute {
                                crate::security::permissions::AccessResult::Allowed
                            } else {
                                crate::security::permissions::AccessResult::Denied
                            }
                        }
                    };
                }
            }
        }
        
        // 기본 권한 사용 (ACL에 없으면 거부)
        crate::security::permissions::AccessResult::Denied
    }
}

impl Default for AccessControlList {
    fn default() -> Self {
        Self::new()
    }
}

