//! FAT32 파일시스템 구현
//!
//! 이 모듈은 FAT32 파일시스템의 읽기/쓰기 기능을 제공합니다.
//! 현재는 읽기 전용으로 시작하며, 향후 쓰기 기능을 추가할 예정입니다.

use super::vfs::{FileSystem, File, Directory, FileMetadata, FileType, FileMode, FsResult, FsError, Offset};
use crate::drivers::ata::{BlockDevice, BlockDeviceError};
use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use core::mem::size_of;

/// FAT32 부트 섹터 (BPB - BIOS Parameter Block)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32BootSector {
    // Jump instruction (3 bytes)
    pub jmp_boot: [u8; 3],
    // OEM name (8 bytes)
    pub oem_name: [u8; 8],
    // Bytes per sector (usually 512)
    pub bytes_per_sector: u16,
    // Sectors per cluster
    pub sectors_per_cluster: u8,
    // Reserved sectors
    pub reserved_sector_count: u16,
    // Number of FATs (usually 2)
    pub num_fats: u8,
    // Root directory entries (unused in FAT32, always 0)
    pub root_entry_count: u16,
    // Total sectors (16-bit, 0 if >= 65536)
    pub total_sectors_16: u16,
    // Media descriptor
    pub media: u8,
    // Sectors per FAT (16-bit, unused in FAT32)
    pub sectors_per_fat_16: u16,
    // Sectors per track
    pub sectors_per_track: u16,
    // Number of heads
    pub num_heads: u16,
    // Hidden sectors
    pub hidden_sectors: u32,
    // Total sectors (32-bit)
    pub total_sectors_32: u32,
    // Sectors per FAT (32-bit, FAT32 only)
    pub sectors_per_fat_32: u32,
    // Flags
    pub flags: u16,
    // FAT version (usually 0)
    pub fat_version: u16,
    // Root directory cluster number
    pub root_cluster: u32,
    // FSInfo sector number
    pub fs_info: u16,
    // Backup boot sector
    pub backup_boot_sector: u16,
    // Reserved (12 bytes)
    pub reserved: [u8; 12],
    // Drive number
    pub drive_number: u8,
    // Reserved (1 byte)
    pub reserved1: u8,
    // Boot signature
    pub boot_signature: u8,
    // Volume ID
    pub volume_id: u32,
    // Volume label (11 bytes)
    pub volume_label: [u8; 11],
    // File system type (8 bytes)
    pub fs_type: [u8; 8],
    // Boot code (420 bytes)
    pub boot_code: [u8; 420],
    // Boot signature (0xAA55)
    pub boot_signature_end: u16,
}

impl Fat32BootSector {
    /// 부트 섹터의 유효성 검사
    pub fn is_valid(&self) -> bool {
        self.boot_signature_end == 0xAA55
            && self.bytes_per_sector == 512
            && self.sectors_per_cluster > 0
            && self.sectors_per_fat_32 > 0
    }
    
    /// 파일시스템 타입 확인
    pub fn is_fat32(&self) -> bool {
        // "FAT32   " 문자열 확인
        self.fs_type[0..8] == *b"FAT32   "
    }
    
    /// 루트 디렉토리 클러스터 번호
    pub fn root_cluster(&self) -> u32 {
        self.root_cluster
    }
    
    /// FAT 테이블 시작 섹터
    pub fn fat_start_sector(&self) -> u64 {
        self.reserved_sector_count as u64
    }
    
    /// 데이터 영역 시작 섹터
    pub fn data_start_sector(&self) -> u64 {
        self.fat_start_sector() + (self.sectors_per_fat_32 as u64 * self.num_fats as u64)
    }
    
    /// 클러스터 번호를 섹터 번호로 변환
    pub fn cluster_to_sector(&self, cluster: u32) -> u64 {
        if cluster < 2 {
            return 0;
        }
        self.data_start_sector() + ((cluster - 2) as u64 * self.sectors_per_cluster as u64)
    }
    
    /// 섹터 번호를 클러스터 번호로 변환
    pub fn sector_to_cluster(&self, sector: u64) -> u32 {
        let data_start = self.data_start_sector();
        if sector < data_start {
            return 0;
        }
        ((sector - data_start) / self.sectors_per_cluster as u64) as u32 + 2
    }
}

/// FAT32 디렉토리 엔트리 (8.3 형식)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32DirEntry {
    // Short file name (8.3 format)
    pub name: [u8; 11],
    // File attributes
    pub attributes: u8,
    // Reserved
    pub reserved: u8,
    // Creation time (tenths of second)
    pub create_time_tenth: u8,
    // Creation time
    pub create_time: u16,
    // Creation date
    pub create_date: u16,
    // Last access date
    pub access_date: u16,
    // First cluster high 16 bits
    pub first_cluster_high: u16,
    // Write time
    pub write_time: u16,
    // Write date
    pub write_date: u16,
    // First cluster low 16 bits
    pub first_cluster_low: u16,
    // File size in bytes
    pub file_size: u32,
}

impl Fat32DirEntry {
    /// 파일 속성 플래그
    pub const ATTR_READ_ONLY: u8 = 0x01;
    pub const ATTR_HIDDEN: u8 = 0x02;
    pub const ATTR_SYSTEM: u8 = 0x04;
    pub const ATTR_VOLUME_ID: u8 = 0x08;
    pub const ATTR_DIRECTORY: u8 = 0x10;
    pub const ATTR_ARCHIVE: u8 = 0x20;
    pub const ATTR_LONG_NAME: u8 = 0x0F;
    
    /// 엔트리가 사용되지 않음 (삭제됨)
    pub fn is_deleted(&self) -> bool {
        self.name[0] == 0xE5 || self.name[0] == 0x00
    }
    
    /// 긴 파일명 엔트리인지 확인
    pub fn is_long_name(&self) -> bool {
        (self.attributes & Self::ATTR_LONG_NAME) == Self::ATTR_LONG_NAME
    }
    
    /// 디렉토리인지 확인
    pub fn is_directory(&self) -> bool {
        (self.attributes & Self::ATTR_DIRECTORY) != 0
    }
    
    /// 볼륨 레이블인지 확인
    pub fn is_volume_label(&self) -> bool {
        (self.attributes & Self::ATTR_VOLUME_ID) != 0
    }
    
    /// 첫 번째 클러스터 번호 가져오기
    pub fn first_cluster(&self) -> u32 {
        ((self.first_cluster_high as u32) << 16) | (self.first_cluster_low as u32)
    }
    
    /// 파일명을 문자열로 변환 (8.3 형식)
    pub fn short_name(&self) -> String {
        let mut name = String::new();
        
        // 파일명 (8자)
        let mut i = 0;
        while i < 8 && self.name[i] != b' ' && self.name[i] != 0 {
            name.push(self.name[i] as char);
            i += 1;
        }
        
        // 확장자 (3자)
        if self.name[8] != b' ' && self.name[8] != 0 {
            name.push('.');
            i = 8;
            while i < 11 && self.name[i] != b' ' && self.name[i] != 0 {
                name.push(self.name[i] as char);
                i += 1;
            }
        }
        
        name
    }
}

/// FAT32 파일시스템
pub struct Fat32FileSystem {
    device: Box<dyn BlockDevice>,
    boot_sector: Fat32BootSector,
    mounted: bool,
    // FAT 캐시 (향후 구현)
    // fat_cache: Vec<u32>,
}

impl Fat32FileSystem {
    /// 새 FAT32 파일시스템 생성
    /// 
    /// # Arguments
    /// * `device` - 블록 디바이스
    /// 
    /// # Returns
    /// 파싱된 파일시스템 또는 오류
    pub fn new(mut device: Box<dyn BlockDevice>) -> FsResult<Self> {
        // 부트 섹터 읽기
        let mut boot_sector_buf = [0u8; 512];
        device.read_block(0, &mut boot_sector_buf)
            .map_err(|_| FsError::IOError)?;
        
        // 부트 섹터 파싱
        let boot_sector = unsafe {
            core::ptr::read(boot_sector_buf.as_ptr() as *const Fat32BootSector)
        };
        
        // 유효성 검사
        if !boot_sector.is_valid() {
            return Err(FsError::InvalidFilesystem);
        }
        
        if !boot_sector.is_fat32() {
            return Err(FsError::InvalidFilesystem);
        }
        
        Ok(Self {
            device,
            boot_sector,
            mounted: false,
        })
    }
    
    /// FAT 엔트리 읽기
    /// 
    /// # Arguments
    /// * `cluster` - 클러스터 번호
    /// 
    /// # Returns
    /// 다음 클러스터 번호 (EOF면 0x0FFFFFFF 이상)
    fn read_fat_entry(&mut self, cluster: u32) -> FsResult<u32> {
        // FAT 오프셋 계산
        let fat_offset = cluster * 4; // FAT32는 4바이트 엔트리
        let fat_sector = self.boot_sector.fat_start_sector() + (fat_offset / 512) as u64;
        let entry_offset = (fat_offset % 512) as usize;
        
        // FAT 섹터 읽기
        let mut fat_buf = [0u8; 512];
        self.device.read_block(fat_sector, &mut fat_buf)
            .map_err(|_| FsError::IOError)?;
        
        // 엔트리 읽기 (little-endian)
        let entry = u32::from_le_bytes([
            fat_buf[entry_offset],
            fat_buf[entry_offset + 1],
            fat_buf[entry_offset + 2],
            fat_buf[entry_offset + 3],
        ]);
        
        // 상위 4비트 마스크 (FAT32는 하위 28비트만 사용)
        Ok(entry & 0x0FFFFFFF)
    }
    
    /// 클러스터 체인 읽기
    /// 
    /// # Arguments
    /// * `start_cluster` - 시작 클러스터 번호
    /// * `buf` - 데이터를 저장할 버퍼
    /// 
    /// # Returns
    /// 읽은 바이트 수
    fn read_cluster_chain(&mut self, start_cluster: u32, buf: &mut [u8]) -> FsResult<usize> {
        let mut cluster = start_cluster;
        let mut offset = 0;
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        
        while cluster >= 2 && cluster < 0x0FFFFFF8 {
            if offset + cluster_size > buf.len() {
                break;
            }
            
            // 클러스터를 섹터로 변환
            let sector = self.boot_sector.cluster_to_sector(cluster);
            
            // 클러스터 읽기
            for i in 0..self.boot_sector.sectors_per_cluster {
                let mut sector_buf = [0u8; 512];
                self.device.read_block(sector + i as u64, &mut sector_buf)
                    .map_err(|_| FsError::IOError)?;
                
                let copy_len = core::cmp::min(512, buf.len() - offset);
                buf[offset..offset + copy_len].copy_from_slice(&sector_buf[..copy_len]);
                offset += copy_len;
            }
            
            // 다음 클러스터 읽기
            cluster = self.read_fat_entry(cluster)?;
        }
        
        Ok(offset)
    }
    
    /// 디렉토리 엔트리 찾기
    /// 
    /// # Arguments
    /// * `dir_cluster` - 디렉토리 클러스터 번호
    /// * `name` - 찾을 파일/디렉토리 이름
    /// 
    /// # Returns
    /// 찾은 디렉토리 엔트리 또는 오류
    fn find_directory_entry(&mut self, dir_cluster: u32, name: &str) -> FsResult<Fat32DirEntry> {
        // 디렉토리 데이터 읽기
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        let mut dir_buf = alloc::vec![0u8; cluster_size * 4]; // 최대 4 클러스터
        let len = self.read_cluster_chain(dir_cluster, &mut dir_buf)?;
        
        // 디렉토리 엔트리 검색
        let num_entries = len / size_of::<Fat32DirEntry>();
        for i in 0..num_entries {
            let entry = unsafe {
                core::ptr::read(dir_buf.as_ptr().add(i * size_of::<Fat32DirEntry>()) as *const Fat32DirEntry)
            };
            
            if entry.is_deleted() || entry.is_long_name() || entry.is_volume_label() {
                continue;
            }
            
            let entry_name = entry.short_name();
            if entry_name.eq_ignore_ascii_case(name) {
                return Ok(entry);
            }
        }
        
        Err(FsError::NotFound)
    }
    
    /// 경로를 클러스터 번호로 변환
    /// 
    /// # Arguments
    /// * `path` - 파일/디렉토리 경로 (예: "/file.txt" 또는 "dir/file.txt")
    /// 
    /// # Returns
    /// 클러스터 번호 및 디렉토리 엔트리
    fn path_to_cluster(&mut self, path: &str) -> FsResult<(u32, Fat32DirEntry)> {
        // 경로 정규화
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            // 루트 디렉토리
            return Ok((self.boot_sector.root_cluster(), Fat32DirEntry {
                name: *b"           ",
                attributes: Fat32DirEntry::ATTR_DIRECTORY,
                reserved: 0,
                create_time_tenth: 0,
                create_time: 0,
                create_date: 0,
                access_date: 0,
                first_cluster_high: (self.boot_sector.root_cluster() >> 16) as u16,
                write_time: 0,
                write_date: 0,
                first_cluster_low: (self.boot_sector.root_cluster() & 0xFFFF) as u16,
                file_size: 0,
            }));
        }
        
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_cluster = self.boot_sector.root_cluster();
        
        // 경로의 각 컴포넌트 탐색
        for (i, component) in components.iter().enumerate() {
            let entry = self.find_directory_entry(current_cluster, component)?;
            
            if i == components.len() - 1 {
                // 마지막 컴포넌트
                return Ok((entry.first_cluster(), entry));
            } else {
                // 중간 디렉토리
                if !entry.is_directory() {
                    return Err(FsError::NotADirectory);
                }
                current_cluster = entry.first_cluster();
            }
        }
        
        Err(FsError::NotFound)
    }
}

impl FileSystem for Fat32FileSystem {
    fn mount(&mut self) -> FsResult<()> {
        if self.mounted {
            return Err(FsError::Busy);
        }
        self.mounted = true;
        Ok(())
    }
    
    fn unmount(&mut self) -> FsResult<()> {
        if !self.mounted {
            return Err(FsError::InvalidFilesystem);
        }
        self.mounted = false;
        Ok(())
    }
    
    fn open_file(&mut self, path: &str) -> FsResult<Box<dyn File>> {
        if !self.mounted {
            return Err(FsError::InvalidFilesystem);
        }
        
        let (cluster, entry) = self.path_to_cluster(path)?;
        if entry.is_directory() {
            return Err(FsError::IsDirectory);
        }
        
        Ok(Box::new(Fat32File {
            filesystem: self,
            entry,
            cluster,
            offset: 0,
        }))
    }
    
    fn create_file(&mut self, _path: &str) -> FsResult<Box<dyn File>> {
        // TODO: 파일 생성 구현
        Err(FsError::IOError)
    }
    
    fn open_dir(&mut self, path: &str) -> FsResult<Box<dyn Directory>> {
        if !self.mounted {
            return Err(FsError::InvalidFilesystem);
        }
        
        let (cluster, entry) = self.path_to_cluster(path)?;
        if !entry.is_directory() {
            return Err(FsError::NotADirectory);
        }
        
        Ok(Box::new(Fat32Directory {
            filesystem: self,
            cluster: if cluster == 0 { self.boot_sector.root_cluster() } else { cluster },
        }))
    }
    
    fn create_dir(&mut self, _path: &str) -> FsResult<()> {
        // TODO: 디렉토리 생성 구현
        Err(FsError::IOError)
    }
    
    fn remove(&mut self, _path: &str) -> FsResult<()> {
        // TODO: 파일/디렉토리 삭제 구현
        Err(FsError::IOError)
    }
    
    fn rename(&mut self, _old_path: &str, _new_path: &str) -> FsResult<()> {
        // TODO: 이름 변경 구현
        Err(FsError::IOError)
    }
    
    fn metadata(&mut self, path: &str) -> FsResult<FileMetadata> {
        if !self.mounted {
            return Err(FsError::InvalidFilesystem);
        }
        
        let (_, entry) = self.path_to_cluster(path)?;
        
        Ok(FileMetadata {
            file_type: if entry.is_directory() {
                FileType::Directory
            } else {
                FileType::Regular
            },
            size: entry.file_size as u64,
            mode: FileMode::new(0o644),
            created: 0, // TODO: 날짜/시간 변환
            modified: 0,
            accessed: 0,
        })
    }
    
    fn is_mounted(&self) -> bool {
        self.mounted
    }
}

/// FAT32 파일 핸들
struct Fat32File<'a> {
    filesystem: &'a mut Fat32FileSystem,
    entry: Fat32DirEntry,
    cluster: u32,
    offset: Offset,
}

impl File for Fat32File<'_> {
    fn read(&mut self, buf: &mut [u8], offset: Option<Offset>) -> FsResult<usize> {
        let read_offset = offset.unwrap_or(self.offset);
        if read_offset < 0 || read_offset as u64 >= self.entry.file_size as u64 {
            return Ok(0);
        }
        
        // 클러스터 체인에서 데이터 읽기
        let cluster_size = self.filesystem.boot_sector.sectors_per_cluster as usize * 512;
        let mut cluster_buf = alloc::vec![0u8; cluster_size];
        
        // 시작 클러스터 찾기
        let mut current_cluster = self.cluster;
        let mut current_offset = read_offset as usize;
        let mut bytes_read = 0;
        
        while current_offset < self.entry.file_size as usize && bytes_read < buf.len() {
            // 현재 오프셋이 속한 클러스터 찾기
            let cluster_index = current_offset / cluster_size;
            for _ in 0..cluster_index {
                current_cluster = self.filesystem.read_fat_entry(current_cluster)?;
                if current_cluster >= 0x0FFFFFF8 {
                    break;
                }
            }
            
            if current_cluster >= 0x0FFFFFF8 {
                break;
            }
            
            // 클러스터 읽기
            let cluster_offset = current_offset % cluster_size;
            let read_len = self.filesystem.read_cluster_chain(current_cluster, &mut cluster_buf)?;
            
            let copy_len = core::cmp::min(
                core::cmp::min(read_len - cluster_offset, buf.len() - bytes_read),
                self.entry.file_size as usize - current_offset,
            );
            
            buf[bytes_read..bytes_read + copy_len]
                .copy_from_slice(&cluster_buf[cluster_offset..cluster_offset + copy_len]);
            
            bytes_read += copy_len;
            current_offset += copy_len;
            
            // 다음 클러스터로 이동
            current_cluster = self.filesystem.read_fat_entry(current_cluster)?;
        }
        
        if offset.is_none() {
            self.offset = read_offset + bytes_read as Offset;
        }
        
        Ok(bytes_read)
    }
    
    fn write(&mut self, _buf: &[u8], _offset: Option<Offset>) -> FsResult<usize> {
        // TODO: 쓰기 구현
        Err(FsError::IOError)
    }
    
    fn metadata(&self) -> FsResult<FileMetadata> {
        Ok(FileMetadata {
            file_type: FileType::Regular,
            size: self.entry.file_size as u64,
            mode: FileMode::new(0o644),
            created: 0,
            modified: 0,
            accessed: 0,
        })
    }
    
    fn size(&self) -> FsResult<u64> {
        Ok(self.entry.file_size as u64)
    }
    
    fn seek(&mut self, offset: Offset) -> FsResult<Offset> {
        if offset < 0 || offset as u64 > self.entry.file_size as u64 {
            return Err(FsError::InvalidPath);
        }
        self.offset = offset;
        Ok(offset)
    }
    
    fn tell(&self) -> FsResult<Offset> {
        Ok(self.offset)
    }
}

/// FAT32 디렉토리 핸들
struct Fat32Directory<'a> {
    filesystem: &'a mut Fat32FileSystem,
    cluster: u32,
}

impl Directory for Fat32Directory<'_> {
    fn read_dir(&self) -> FsResult<Vec<(String, FileType)>> {
        // TODO: 디렉토리 읽기 구현
        // 현재는 빈 벡터 반환 (컴파일 오류 방지)
        let mut result = Vec::new();
        
        // 디렉토리 데이터 읽기
        let cluster_size = self.filesystem.boot_sector.sectors_per_cluster as usize * 512;
        let mut dir_buf = alloc::vec![0u8; cluster_size * 4];
        let len = match self.filesystem.read_cluster_chain(self.cluster, &mut dir_buf) {
            Ok(l) => l,
            Err(_) => return Ok(result),
        };
        
        // 디렉토리 엔트리 파싱
        let num_entries = len / size_of::<Fat32DirEntry>();
        for i in 0..num_entries {
            let entry = unsafe {
                core::ptr::read(dir_buf.as_ptr().add(i * size_of::<Fat32DirEntry>()) as *const Fat32DirEntry)
            };
            
            if entry.is_deleted() || entry.is_long_name() || entry.is_volume_label() {
                continue;
            }
            
            let name = entry.short_name();
            let file_type = if entry.is_directory() {
                FileType::Directory
            } else {
                FileType::Regular
            };
            
            result.push((name, file_type));
        }
        
        Ok(result)
    }
    
    fn create_file(&mut self, _name: &str) -> FsResult<()> {
        Err(FsError::IOError)
    }
    
    fn create_dir(&mut self, _name: &str) -> FsResult<()> {
        Err(FsError::IOError)
    }
    
    fn remove(&mut self, _name: &str) -> FsResult<()> {
        Err(FsError::IOError)
    }
    
    fn rename(&mut self, _old_name: &str, _new_name: &str) -> FsResult<()> {
        Err(FsError::IOError)
    }
}

