//! 사용자 및 그룹 관리
//!
//! 이 모듈은 사용자 ID, 그룹 ID, 그리고 권한 관리를 제공합니다.

use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// 사용자 ID 타입
pub type UserId = u32;
/// 그룹 ID 타입
pub type GroupId = u32;

/// 사용자 정보
#[derive(Debug, Clone)]
pub struct User {
    /// 사용자 ID
    pub uid: UserId,
    /// 그룹 ID (기본 그룹)
    pub gid: GroupId,
    /// 사용자 이름
    pub name: String,
    /// 홈 디렉토리 경로
    pub home: String,
    /// 쉘 경로
    pub shell: String,
    /// 추가 그룹 목록
    pub groups: Vec<GroupId>,
}

/// 그룹 정보
#[derive(Debug, Clone)]
pub struct Group {
    /// 그룹 ID
    pub gid: GroupId,
    /// 그룹 이름
    pub name: String,
    /// 그룹 멤버 목록 (UID)
    pub members: Vec<UserId>,
}

/// 사용자 관리자
pub struct UserManager {
    /// 사용자 목록 (UID -> User)
    users: BTreeMap<UserId, User>,
    /// 그룹 목록 (GID -> Group)
    groups: BTreeMap<GroupId, Group>,
    /// 사용자 이름 -> UID 매핑
    name_to_uid: BTreeMap<String, UserId>,
    /// 그룹 이름 -> GID 매핑
    name_to_gid: BTreeMap<String, GroupId>,
}

impl UserManager {
    /// 새 사용자 관리자 생성
    pub fn new() -> Self {
        let mut manager = Self {
            users: BTreeMap::new(),
            groups: BTreeMap::new(),
            name_to_uid: BTreeMap::new(),
            name_to_gid: BTreeMap::new(),
        };
        
        // 기본 사용자/그룹 추가
        manager.init_default_users();
        manager
    }
    
    /// 기본 사용자/그룹 초기화
    fn init_default_users(&mut self) {
        // Root 사용자 (UID 0)
        let root = User {
            uid: 0,
            gid: 0,
            name: "root".to_string(),
            home: "/root".to_string(),
            shell: "/bin/sh".to_string(),
            groups: vec![0],
        };
        self.add_user(root);
        
        // Root 그룹 (GID 0)
        let root_group = Group {
            gid: 0,
            name: "root".to_string(),
            members: vec![0],
        };
        self.add_group(root_group);
        
        // 일반 사용자 (UID 1000)
        let user = User {
            uid: 1000,
            gid: 1000,
            name: "user".to_string(),
            home: "/home/user".to_string(),
            shell: "/bin/sh".to_string(),
            groups: vec![1000],
        };
        self.add_user(user);
        
        // 일반 사용자 그룹 (GID 1000)
        let user_group = Group {
            gid: 1000,
            name: "users".to_string(),
            members: vec![1000],
        };
        self.add_group(user_group);
    }
    
    /// 사용자 추가
    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.uid, user.clone());
        self.name_to_uid.insert(user.name.clone(), user.uid);
    }
    
    /// 그룹 추가
    pub fn add_group(&mut self, group: Group) {
        self.groups.insert(group.gid, group.clone());
        self.name_to_gid.insert(group.name.clone(), group.gid);
    }
    
    /// UID로 사용자 조회
    pub fn get_user_by_uid(&self, uid: UserId) -> Option<&User> {
        self.users.get(&uid)
    }
    
    /// 이름으로 사용자 조회
    pub fn get_user_by_name(&self, name: &str) -> Option<&User> {
        self.name_to_uid.get(name).and_then(|uid| self.users.get(uid))
    }
    
    /// GID로 그룹 조회
    pub fn get_group_by_gid(&self, gid: GroupId) -> Option<&Group> {
        self.groups.get(&gid)
    }
    
    /// 이름으로 그룹 조회
    pub fn get_group_by_name(&self, name: &str) -> Option<&Group> {
        self.name_to_gid.get(name).and_then(|gid| self.groups.get(gid))
    }
    
    /// 사용자가 그룹에 속하는지 확인
    pub fn is_user_in_group(&self, uid: UserId, gid: GroupId) -> bool {
        if let Some(user) = self.get_user_by_uid(uid) {
            // 기본 그룹 확인
            if user.gid == gid {
                return true;
            }
            // 추가 그룹 확인
            if user.groups.contains(&gid) {
                return true;
            }
            // 그룹 멤버 목록 확인
            if let Some(group) = self.get_group_by_gid(gid) {
                return group.members.contains(&uid);
            }
        }
        false
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 전역 사용자 관리자
static USER_MANAGER: Mutex<UserManager> = Mutex::new(UserManager::new());

/// 현재 사용자 ID 가져오기
pub fn get_current_uid() -> UserId {
    // TODO: 현재 실행 중인 프로세스의 UID 가져오기
    // 현재는 기본값 반환
    1000
}

/// 현재 그룹 ID 가져오기
pub fn get_current_gid() -> GroupId {
    // TODO: 현재 실행 중인 프로세스의 GID 가져오기
    // 현재는 기본값 반환
    1000
}

/// 사용자 관리자 가져오기
pub fn get_user_manager() -> &'static Mutex<UserManager> {
    &USER_MANAGER
}

