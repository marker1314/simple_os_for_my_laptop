//! 보안 및 접근 제어 모듈
//!
//! 이 모듈은 사용자/그룹 관리, 파일 권한, 접근 제어 등을 제공합니다.

pub mod user;
pub mod permissions;
pub mod acl;

pub use user::{UserId, GroupId, User, Group, UserManager, get_current_uid, get_current_gid, get_user_manager};
pub use permissions::{AccessPermission, AccessResult, PermissionChecker, UnixPermissions};
pub use acl::{AclEntry, AccessControlList};

