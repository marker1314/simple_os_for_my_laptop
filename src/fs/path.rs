//! 경로 처리 유틸리티
//!
//! 파일시스템 경로를 파싱하고 조작하는 기능을 제공합니다.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;

/// 경로 구성요소
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathComponent(String);

impl PathComponent {
    /// 새 경로 구성요소 생성
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }
    
    /// 문자열로 변환
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// 현재 디렉토리 (.)인지 확인
    pub fn is_current_dir(&self) -> bool {
        self.0 == "."
    }
    
    /// 부모 디렉토리 (..)인지 확인
    pub fn is_parent_dir(&self) -> bool {
        self.0 == ".."
    }
    
    /// 루트 디렉토리인지 확인
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }
}

/// 파일시스템 경로
#[derive(Debug, Clone)]
pub struct Path {
    /// 절대 경로 여부
    pub absolute: bool,
    /// 경로 구성요소
    pub components: Vec<PathComponent>,
}

impl Path {
    /// 문자열에서 경로 파싱
    ///
    /// # Arguments
    /// * `path` - 파싱할 경로 문자열
    ///
    /// # Returns
    /// 파싱된 경로
    pub fn parse(path: &str) -> Result<Self, &'static str> {
        if path.is_empty() {
            return Err("Empty path");
        }
        
        let absolute = path.starts_with('/');
        let mut components = Vec::new();
        
        // 경로를 '/'로 분리
        for component in path.split('/') {
            if component.is_empty() {
                continue; // "//" 같은 경우 무시
            }
            
            // 유효한 파일명 검사
            if !Self::is_valid_component(component) {
                return Err("Invalid path component");
            }
            
            components.push(PathComponent::new(component));
        }
        
        Ok(Self {
            absolute,
            components,
        })
    }
    
    /// 경로 구성요소가 유효한지 확인
    fn is_valid_component(component: &str) -> bool {
        if component.is_empty() || component.len() > 255 {
            return false;
        }
        
        // 금지된 문자 확인
        for ch in component.chars() {
            if ch == '\0' {
                return false;
            }
        }
        
        true
    }
    
    /// 경로 정규화 (. 및 .. 처리)
    ///
    /// # Returns
    /// 정규화된 경로
    pub fn normalize(&self) -> Self {
        let mut normalized = Vec::new();
        
        for component in &self.components {
            if component.is_current_dir() {
                // '.'는 무시
                continue;
            } else if component.is_parent_dir() {
                // '..'는 이전 구성요소 제거
                if !normalized.is_empty() {
                    normalized.pop();
                }
            } else {
                normalized.push(component.clone());
            }
        }
        
        Self {
            absolute: self.absolute,
            components: normalized,
        }
    }
    
    /// 경로 결합
    ///
    /// # Arguments
    /// * `other` - 결합할 경로
    ///
    /// # Returns
    /// 결합된 경로
    pub fn join(&self, other: &Path) -> Self {
        if other.absolute {
            // 절대 경로면 그대로 반환
            return other.clone();
        }
        
        let mut combined = self.components.clone();
        combined.extend(other.components.iter().cloned());
        
        Self {
            absolute: self.absolute,
            components: combined,
        }
    }
    
    /// 부모 디렉토리 경로
    ///
    /// # Returns
    /// 부모 디렉토리 경로 (없으면 None)
    pub fn parent(&self) -> Option<Self> {
        if self.components.is_empty() {
            return None;
        }
        
        let mut components = self.components.clone();
        components.pop();
        
        Some(Self {
            absolute: self.absolute,
            components,
        })
    }
    
    /// 파일/디렉토리 이름 (마지막 구성요소)
    ///
    /// # Returns
    /// 파일/디렉토리 이름
    pub fn file_name(&self) -> Option<&str> {
        self.components.last().map(|c| c.as_str())
    }
    
    /// 경로를 문자열로 변환
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        
        if self.absolute {
            result.push('/');
        }
        
        for (i, component) in self.components.iter().enumerate() {
            if i > 0 {
                result.push('/');
            }
            result.push_str(component.as_str());
        }
        
        result
    }
    
    /// 루트 경로인지 확인
    pub fn is_root(&self) -> bool {
        self.absolute && self.components.is_empty()
    }
    
    /// 경로 깊이 반환
    pub fn depth(&self) -> usize {
        self.components.len()
    }
}

impl core::fmt::Display for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test_case]
    fn test_path_parse() {
        let path = Path::parse("/home/user/file.txt").unwrap();
        assert!(path.absolute);
        assert_eq!(path.components.len(), 3);
        assert_eq!(path.file_name(), Some("file.txt"));
    }
    
    #[test_case]
    fn test_path_normalize() {
        let path = Path::parse("/home/user/../admin/./file.txt").unwrap();
        let normalized = path.normalize();
        assert_eq!(normalized.to_string(), "/home/admin/file.txt");
    }
    
    #[test_case]
    fn test_path_join() {
        let base = Path::parse("/home/user").unwrap();
        let rel = Path::parse("documents/file.txt").unwrap();
        let joined = base.join(&rel);
        assert_eq!(joined.to_string(), "/home/user/documents/file.txt");
    }
    
    #[test_case]
    fn test_path_parent() {
        let path = Path::parse("/home/user/file.txt").unwrap();
        let parent = path.parent().unwrap();
        assert_eq!(parent.to_string(), "/home/user");
    }
}

