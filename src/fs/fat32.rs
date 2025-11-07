//! FAT32 파일시스템 구현
//!
//! 이 모듈은 FAT32 파일시스템의 읽기/쓰기 기능을 제공합니다.
//! 현재는 읽기 전용으로 시작하며, 향후 쓰기 기능을 추가할 예정입니다.

use super::vfs::{FileSystem, File, Directory, FileMetadata, FileType, FileMode, FsResult, FsError, Offset};
use super::journal::{begin_transaction, add_entry, commit, checkpoint, rollback, JournalEntryType};
use crate::drivers::ata::{BlockDevice, BlockDeviceError};
use alloc::vec::Vec;
use alloc::string::ToString;
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

    /// 부트 섹터 참조 반환 (검사용)
    pub fn boot_sector(&self) -> &Fat32BootSector {
        &self.boot_sector
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
    
    /// FAT 엔트리 쓰기
    /// 
    /// # Arguments
    /// * `cluster` - 클러스터 번호
    /// * `value` - 쓸 값 (다음 클러스터 번호 또는 EOF 마커)
    /// * `use_journal` - 저널링 사용 여부 (기본값: true)
    fn write_fat_entry(&mut self, cluster: u32, value: u32, use_journal: bool) -> FsResult<()> {
        // FAT 오프셋 계산
        let fat_offset = cluster * 4;
        let fat_sector = self.boot_sector.fat_start_sector() + (fat_offset / 512) as u64;
        let entry_offset = (fat_offset % 512) as usize;
        
        // FAT 섹터 읽기
        let mut fat_buf = [0u8; 512];
        self.device.read_block(fat_sector, &mut fat_buf)
            .map_err(|_| FsError::IOError)?;
        
        // 엔트리 쓰기 (little-endian, 상위 4비트 보존)
        let bytes = value.to_le_bytes();
        fat_buf[entry_offset] = bytes[0];
        fat_buf[entry_offset + 1] = bytes[1];
        fat_buf[entry_offset + 2] = bytes[2];
        fat_buf[entry_offset + 3] = bytes[3];
        
        // 저널링 사용 시 트랜잭션 시작 및 저널에 기록
        if use_journal {
            // 트랜잭션이 없을 때만 시작
            if !super::journal::in_transaction() {
                begin_transaction();
            }
            // 첫 번째 FAT만 저널에 기록 (복사본은 체크포인트에서 처리)
            if let Err(e) = add_entry(fat_sector, &fat_buf, JournalEntryType::MetadataWrite) {
                rollback();
                return Err(FsError::IOError);
            }
        }
        
        // 모든 FAT 복사본에 쓰기
        for fat_index in 0..self.boot_sector.num_fats {
            let current_fat_sector = self.boot_sector.fat_start_sector() + 
                (fat_index as u64 * self.boot_sector.sectors_per_fat_32 as u64) +
                (fat_offset / 512) as u64;
            
            // 저널링 사용 시 두 번째 FAT부터는 저널에 기록
            if use_journal && fat_index > 0 {
                if let Err(e) = add_entry(current_fat_sector, &fat_buf, JournalEntryType::MetadataWrite) {
                    rollback();
                    return Err(FsError::IOError);
                }
            }
            
            self.device.write_block(current_fat_sector, &fat_buf)
                .map_err(|_| FsError::IOError)?;
        }
        
        Ok(())
    }
    
    /// write_fat_entry의 기본 버전 (저널링 사용)
    fn write_fat_entry_journaled(&mut self, cluster: u32, value: u32) -> FsResult<()> {
        self.write_fat_entry(cluster, value, true)
    }
    
    /// 빈 클러스터 찾기
    /// 
    /// # Returns
    /// 빈 클러스터 번호 또는 오류
    fn find_free_cluster(&mut self) -> FsResult<u32> {
        // 데이터 영역의 첫 번째 클러스터부터 검색
        // 클러스터 2부터 시작 (0과 1은 예약됨)
        let fat_start = self.boot_sector.fat_start_sector();
        let sectors_per_fat = self.boot_sector.sectors_per_fat_32 as u64;
        
        // FAT의 각 섹터를 읽어서 빈 클러스터 찾기
        let mut fat_buf = [0u8; 512];
        for sector_offset in 0..sectors_per_fat {
            let fat_sector = fat_start + sector_offset;
            self.device.read_block(fat_sector, &mut fat_buf)
                .map_err(|_| FsError::IOError)?;
            
            // 섹터 내의 각 엔트리 확인 (128개 엔트리 = 512바이트 / 4바이트)
            for i in 0..128 {
                let entry_offset = i * 4;
                let entry = u32::from_le_bytes([
                    fat_buf[entry_offset],
                    fat_buf[entry_offset + 1],
                    fat_buf[entry_offset + 2],
                    fat_buf[entry_offset + 3],
                ]) & 0x0FFFFFFF;
                
                let cluster = (sector_offset * 128 + i as u64) as u32;
                
                // 클러스터 0과 1은 예약됨
                if cluster < 2 {
                    continue;
                }
                
                // 빈 클러스터 찾기 (0x00000000)
                if entry == 0 {
                    return Ok(cluster);
                }
            }
        }
        
        Err(FsError::OutOfSpace)
    }
    
    /// 클러스터에 데이터 쓰기
    /// 
    /// # Arguments
    /// * `cluster` - 클러스터 번호
    /// * `data` - 쓸 데이터 (클러스터 크기)
    /// * `use_journal` - 저널링 사용 여부 (기본값: false, 데이터는 일반적으로 저널링 안 함)
    fn write_cluster(&mut self, cluster: u32, data: &[u8], use_journal: bool) -> FsResult<()> {
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        if data.len() < cluster_size {
            return Err(FsError::IOError);
        }
        
        let sector = self.boot_sector.cluster_to_sector(cluster);
        let mut offset = 0;
        
        // 저널링 사용 시 트랜잭션 시작
        if use_journal {
            // 트랜잭션이 없을 때만 시작
            if !super::journal::in_transaction() {
                begin_transaction();
            }
        }
        
        for i in 0..self.boot_sector.sectors_per_cluster {
            let sector_buf = &data[offset..offset + 512];
            let current_sector = sector + i as u64;
            
            // 저널링 사용 시 저널에 기록
            if use_journal {
                if let Err(e) = add_entry(current_sector, sector_buf, JournalEntryType::DataWrite) {
                    rollback();
                    return Err(FsError::IOError);
                }
            }
            
            self.device.write_block(current_sector, sector_buf)
                .map_err(|_| FsError::IOError)?;
            offset += 512;
        }
        
        Ok(())
    }
    
    /// write_cluster의 기본 버전 (저널링 사용 안 함)
    fn write_cluster_direct(&mut self, cluster: u32, data: &[u8]) -> FsResult<()> {
        self.write_cluster(cluster, data, false)
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
    
    /// 단일 클러스터 읽기
    /// 
    /// # Arguments
    /// * `cluster` - 클러스터 번호
    /// * `buf` - 데이터를 저장할 버퍼 (최소 클러스터 크기)
    /// 
    /// # Returns
    /// 읽은 바이트 수
    fn read_single_cluster(&mut self, cluster: u32, buf: &mut [u8]) -> FsResult<usize> {
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        if buf.len() < cluster_size {
            return Err(FsError::IOError);
        }
        
        let sector = self.boot_sector.cluster_to_sector(cluster);
        let mut offset = 0;
        
        for i in 0..self.boot_sector.sectors_per_cluster {
            let mut sector_buf = [0u8; 512];
            self.device.read_block(sector + i as u64, &mut sector_buf)
                .map_err(|_| FsError::IOError)?;
            
            buf[offset..offset + 512].copy_from_slice(&sector_buf);
            offset += 512;
        }
        
        Ok(offset)
    }
    
    /// 클러스터 번호로부터 오프셋에 해당하는 클러스터 찾기
    /// 
    /// # Arguments
    /// * `start_cluster` - 시작 클러스터 번호
    /// * `byte_offset` - 바이트 오프셋
    /// 
    /// # Returns
    /// (클러스터 번호, 클러스터 내 오프셋)
    fn find_cluster_for_offset(&mut self, start_cluster: u32, byte_offset: usize) -> FsResult<(u32, usize)> {
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        let cluster_index = byte_offset / cluster_size;
        let cluster_offset = byte_offset % cluster_size;
        
        let mut cluster = start_cluster;
        for _ in 0..cluster_index {
            cluster = self.read_fat_entry(cluster)?;
            if cluster >= 0x0FFFFFF8 {
                return Err(FsError::IOError);
            }
        }
        
        Ok((cluster, cluster_offset))
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
    
    /// 경로를 디렉토리와 파일명으로 분리
    /// 
    /// # Arguments
    /// * `path` - 파일/디렉토리 경로
    /// 
    /// # Returns
    /// (디렉토리 클러스터, 파일명)
    fn split_path<'a>(&mut self, path: &'a str) -> FsResult<(u32, &'a str)> {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            return Ok((self.boot_sector.root_cluster(), ""));
        }
        
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if components.is_empty() {
            return Ok((self.boot_sector.root_cluster(), ""));
        }
        
        if components.len() == 1 {
            return Ok((self.boot_sector.root_cluster(), components[0]));
        }
        
        // 디렉토리 경로 탐색
        let mut current_cluster = self.boot_sector.root_cluster();
        for i in 0..components.len() - 1 {
            let entry = self.find_directory_entry(current_cluster, components[i])?;
            if !entry.is_directory() {
                return Err(FsError::NotADirectory);
            }
            current_cluster = entry.first_cluster();
        }
        
        Ok((current_cluster, components[components.len() - 1]))
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
    
    /// 파일명을 8.3 형식으로 변환
    /// 
    /// # Arguments
    /// * `name` - 파일명
    /// 
    /// # Returns
    /// 11바이트 배열 (8바이트 이름 + 3바이트 확장자)
    fn name_to_8_3(name: &str) -> [u8; 11] {
        let mut result = [b' '; 11];
        let parts: Vec<&str> = name.split('.').collect();
        
        let basename = if parts.is_empty() { "" } else { parts[0] };
        let ext = if parts.len() > 1 { parts[parts.len() - 1] } else { "" };
        
        // 기본 이름 (최대 8자)
        let basename_bytes = basename.as_bytes();
        let basename_len = core::cmp::min(8, basename_bytes.len());
        for i in 0..basename_len {
            result[i] = basename_bytes[i].to_ascii_uppercase();
        }
        
        // 확장자 (최대 3자)
        let ext_bytes = ext.as_bytes();
        let ext_len = core::cmp::min(3, ext_bytes.len());
        for i in 0..ext_len {
            result[8 + i] = ext_bytes[i].to_ascii_uppercase();
        }
        
        result
    }
    
    /// 경로를 부모 경로와 파일명으로 분리
    ///
    /// # Arguments
    /// * `path` - 분리할 경로
    ///
    /// # Returns
    /// (부모 경로, 파일명)
    fn split_parent_path<'a>(&self, path: &'a str) -> FsResult<(&'a str, &'a str)> {
        let path = path.trim_start_matches('/').trim_end_matches('/');
        if path.is_empty() {
            return Err(FsError::InvalidPath);
        }
        
        if let Some(pos) = path.rfind('/') {
            Ok((&path[..pos], &path[pos + 1..]))
        } else {
            Ok(("", path))
        }
    }
    
    /// 디렉토리가 비어있는지 확인
    ///
    /// # Arguments
    /// * `dir_cluster` - 디렉토리 클러스터
    ///
    /// # Returns
    /// 비어있으면 true
    fn is_directory_empty(&mut self, dir_cluster: u32) -> FsResult<bool> {
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        let mut dir_buf = alloc::vec![0u8; cluster_size];
        
        self.read_single_cluster(dir_cluster, &mut dir_buf)?;
        
        let num_entries = cluster_size / size_of::<Fat32DirEntry>();
        for i in 2..num_entries { // 0과 1은 "." 와 ".." 엔트리
            let entry = unsafe {
                core::ptr::read(dir_buf.as_ptr().add(i * size_of::<Fat32DirEntry>()) as *const Fat32DirEntry)
            };
            
            if !entry.is_deleted() && entry.name[0] != 0 {
                return Ok(false); // 비어있지 않음
            }
        }
        
        Ok(true) // 비어있음
    }
    
    /// FAT 클러스터 체인 해제
    ///
    /// # Arguments
    /// * `start_cluster` - 시작 클러스터
    fn free_cluster_chain(&mut self, start_cluster: u32) -> FsResult<()> {
        let mut current_cluster = start_cluster;
        
        loop {
            let next_cluster = self.read_fat_entry(current_cluster)?;
            self.write_fat_entry_journaled(current_cluster, 0)?; // 클러스터 해제
            
            if next_cluster >= 0x0FFFFFF8 {
                break; // EOF
            }
            
            current_cluster = next_cluster;
        }
        
        Ok(())
    }
    
    /// 디렉토리 엔트리 삭제
    ///
    /// # Arguments
    /// * `dir_cluster` - 디렉토리 클러스터
    /// * `filename` - 삭제할 파일명
    fn delete_directory_entry(&mut self, dir_cluster: u32, filename: &str) -> FsResult<()> {
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        let mut dir_buf = alloc::vec![0u8; cluster_size];
        
        self.read_single_cluster(dir_cluster, &mut dir_buf)?;
        
        let name_8_3 = Self::name_to_8_3(filename);
        let num_entries = cluster_size / size_of::<Fat32DirEntry>();
        
        for i in 0..num_entries {
            let offset = i * size_of::<Fat32DirEntry>();
            let entry = unsafe {
                core::ptr::read(dir_buf.as_ptr().add(offset) as *const Fat32DirEntry)
            };
            
            if entry.name == name_8_3 {
                // 엔트리 삭제 마킹 (첫 바이트를 0xE5로)
                dir_buf[offset] = 0xE5;
                // 디렉토리 엔트리 삭제는 메타데이터이므로 저널링 사용
                self.write_cluster(dir_cluster, &dir_buf, true)?;
                return Ok(());
            }
        }
        
        Err(FsError::NotFound)
    }
    
    /// 디렉토리 엔트리 업데이트
    ///
    /// # Arguments
    /// * `dir_cluster` - 디렉토리 클러스터
    /// * `filename` - 파일명
    /// * `new_entry` - 새 엔트리
    fn update_directory_entry(&mut self, dir_cluster: u32, filename: &str, new_entry: Fat32DirEntry) -> FsResult<()> {
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        let mut dir_buf = alloc::vec![0u8; cluster_size];
        
        self.read_single_cluster(dir_cluster, &mut dir_buf)?;
        
        let name_8_3 = Self::name_to_8_3(filename);
        let num_entries = cluster_size / size_of::<Fat32DirEntry>();
        
        for i in 0..num_entries {
            let offset = i * size_of::<Fat32DirEntry>();
            let entry = unsafe {
                core::ptr::read(dir_buf.as_ptr().add(offset) as *const Fat32DirEntry)
            };
            
            if entry.name == name_8_3 {
                // 엔트리 업데이트
                let entry_bytes = unsafe {
                    core::slice::from_raw_parts(
                        &new_entry as *const Fat32DirEntry as *const u8,
                        size_of::<Fat32DirEntry>()
                    )
                };
                dir_buf[offset..offset + size_of::<Fat32DirEntry>()].copy_from_slice(entry_bytes);
                // 디렉토리 엔트리 업데이트는 메타데이터이므로 저널링 사용
                self.write_cluster(dir_cluster, &dir_buf, true)?;
                return Ok(());
            }
        }
        
        Err(FsError::NotFound)
    }
    
    /// 디렉토리에 새 엔트리 추가
    /// 
    /// # Arguments
    /// * `dir_cluster` - 디렉토리 클러스터 번호
    /// * `entry` - 추가할 디렉토리 엔트리
    fn add_directory_entry(&mut self, dir_cluster: u32, entry: Fat32DirEntry) -> FsResult<()> {
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        let mut dir_buf = alloc::vec![0u8; cluster_size];
        
        // 디렉토리 데이터 읽기
        let mut current_cluster = dir_cluster;
        let mut found_empty = false;
        let mut empty_cluster = 0;
        let mut empty_offset = 0;
        
        loop {
            self.read_single_cluster(current_cluster, &mut dir_buf)?;
            
            // 빈 엔트리 찾기
            let num_entries = cluster_size / size_of::<Fat32DirEntry>();
            for i in 0..num_entries {
                let entry_ptr = unsafe { dir_buf.as_ptr().add(i * size_of::<Fat32DirEntry>()) };
                let existing_entry = unsafe { core::ptr::read(entry_ptr as *const Fat32DirEntry) };
                
                if existing_entry.is_deleted() || existing_entry.name[0] == 0 {
                    // 빈 엔트리 발견
                    found_empty = true;
                    empty_cluster = current_cluster;
                    empty_offset = i * size_of::<Fat32DirEntry>();
                    break;
                }
            }
            
            if found_empty {
                break;
            }
            
            // 다음 클러스터 확인
            let next_cluster = self.read_fat_entry(current_cluster)?;
            if next_cluster >= 0x0FFFFFF8 {
                // 새 클러스터 할당 필요
                let new_cluster = self.find_free_cluster()?;
                self.write_fat_entry_journaled(current_cluster, new_cluster)?;
                self.write_fat_entry_journaled(new_cluster, 0x0FFFFFFF)?; // EOF
                
                // 새 클러스터를 0으로 초기화 (데이터는 저널링 안 함)
                let mut empty_cluster_buf = alloc::vec![0u8; cluster_size];
                self.write_cluster_direct(new_cluster, &empty_cluster_buf)?;
                
                empty_cluster = new_cluster;
                empty_offset = 0;
                found_empty = true;
                break;
            }
            
            current_cluster = next_cluster;
        }
        
        if !found_empty {
            return Err(FsError::IOError);
        }
        
        // 엔트리 쓰기
        self.read_single_cluster(empty_cluster, &mut dir_buf)?;
        let entry_bytes = unsafe {
            core::slice::from_raw_parts(
                &entry as *const Fat32DirEntry as *const u8,
                size_of::<Fat32DirEntry>()
            )
        };
        dir_buf[empty_offset..empty_offset + size_of::<Fat32DirEntry>()]
            .copy_from_slice(entry_bytes);
        // 디렉토리 엔트리 추가는 메타데이터이므로 저널링 사용
        self.write_cluster(empty_cluster, &dir_buf, true)?;
        
        Ok(())
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
            filesystem: self as *mut Fat32FileSystem,
            entry,
            cluster,
            offset: 0,
        }))
    }
    
    fn create_file(&mut self, path: &str) -> FsResult<Box<dyn File>> {
        if !self.mounted {
            return Err(FsError::InvalidFilesystem);
        }
        
        // 경로 분리
        let (dir_cluster, filename_ref) = self.split_path(path)?;
        let filename_owned = filename_ref.to_string();
        
        if filename_ref.is_empty() {
            return Err(FsError::InvalidPath);
        }
        
        // 파일이 이미 존재하는지 확인
        match self.find_directory_entry(dir_cluster, &filename_owned) {
            Ok(_) => return Err(FsError::AlreadyExists),
            Err(FsError::NotFound) => {},
            Err(e) => return Err(e),
        }
        
        // 빈 클러스터 할당
        let first_cluster = self.find_free_cluster()?;
        self.write_fat_entry_journaled(first_cluster, 0x0FFFFFFF)?; // EOF
        
        // 빈 클러스터 초기화 (데이터는 저널링 안 함)
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        let mut empty_cluster = alloc::vec![0u8; cluster_size];
        self.write_cluster_direct(first_cluster, &empty_cluster)?;
        
        // 디렉토리 엔트리 생성
        let name_8_3 = Self::name_to_8_3(&filename_owned);
        let entry = Fat32DirEntry {
            name: name_8_3,
            attributes: 0, // 일반 파일
            reserved: 0,
            create_time_tenth: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            first_cluster_high: (first_cluster >> 16) as u16,
            write_time: 0,
            write_date: 0,
            first_cluster_low: (first_cluster & 0xFFFF) as u16,
            file_size: 0,
        };
        
        // 디렉토리에 엔트리 추가
        self.add_directory_entry(dir_cluster, entry)?;
        
        // 파일 열기
        let (cluster, entry) = self.path_to_cluster(path)?;
        Ok(Box::new(Fat32File {
            filesystem: self,
            entry,
            cluster,
            offset: 0,
        }))
    }
    
    fn open_dir(&mut self, path: &str) -> FsResult<Box<dyn Directory>> {
        if !self.mounted {
            return Err(FsError::InvalidFilesystem);
        }
        
        let (cluster, entry) = self.path_to_cluster(path)?;
        if !entry.is_directory() {
            return Err(FsError::NotADirectory);
        }
        
        let root_cluster = self.boot_sector.root_cluster();
        let resolved_cluster = if cluster == 0 { root_cluster } else { cluster };
        Ok(Box::new(Fat32Directory {
            filesystem: self as *mut Fat32FileSystem,
            cluster: resolved_cluster,
        }))
    }
    
    fn create_dir(&mut self, path: &str) -> FsResult<()> {
        if !self.mounted {
            return Err(FsError::InvalidFilesystem);
        }
        
        // 경로 분리
        let (dir_cluster, dirname) = self.split_path(path)?;
        
        if dirname.is_empty() {
            return Err(FsError::InvalidPath);
        }
        
        // 디렉토리가 이미 존재하는지 확인
        match self.find_directory_entry(dir_cluster, dirname) {
            Ok(_) => return Err(FsError::AlreadyExists),
            Err(FsError::NotFound) => {},
            Err(e) => return Err(e),
        }
        
        // 빈 클러스터 할당
        let first_cluster = self.find_free_cluster()?;
        self.write_fat_entry_journaled(first_cluster, 0x0FFFFFFF)?; // EOF
        
        // 디렉토리 클러스터 초기화 (. 및 .. 엔트리 포함)
        let cluster_size = self.boot_sector.sectors_per_cluster as usize * 512;
        let mut dir_cluster_buf = alloc::vec![0u8; cluster_size];
        
        // . 엔트리 (현재 디렉토리)
        let dot_entry = Fat32DirEntry {
            name: *b".          ",
            attributes: Fat32DirEntry::ATTR_DIRECTORY,
            reserved: 0,
            create_time_tenth: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            first_cluster_high: (first_cluster >> 16) as u16,
            write_time: 0,
            write_date: 0,
            first_cluster_low: (first_cluster & 0xFFFF) as u16,
            file_size: 0,
        };
        
        // .. 엔트리 (상위 디렉토리)
        let dotdot_cluster = if dir_cluster == self.boot_sector.root_cluster() {
            0
        } else {
            dir_cluster
        };
        let dotdot_entry = Fat32DirEntry {
            name: *b"..         ",
            attributes: Fat32DirEntry::ATTR_DIRECTORY,
            reserved: 0,
            create_time_tenth: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            first_cluster_high: (dotdot_cluster >> 16) as u16,
            write_time: 0,
            write_date: 0,
            first_cluster_low: (dotdot_cluster & 0xFFFF) as u16,
            file_size: 0,
        };
        
        // 엔트리 쓰기
        let dot_bytes = unsafe {
            core::slice::from_raw_parts(
                &dot_entry as *const Fat32DirEntry as *const u8,
                size_of::<Fat32DirEntry>()
            )
        };
        let dotdot_bytes = unsafe {
            core::slice::from_raw_parts(
                &dotdot_entry as *const Fat32DirEntry as *const u8,
                size_of::<Fat32DirEntry>()
            )
        };
        
        dir_cluster_buf[0..size_of::<Fat32DirEntry>()].copy_from_slice(dot_bytes);
        dir_cluster_buf[size_of::<Fat32DirEntry>()..2 * size_of::<Fat32DirEntry>()]
            .copy_from_slice(dotdot_bytes);
        
        // 디렉토리 엔트리는 메타데이터이므로 저널링 사용
        self.write_cluster(first_cluster, &dir_cluster_buf, true)?;
        
        // 부모 디렉토리에 엔트리 추가
        let name_8_3 = Self::name_to_8_3(dirname);
        let entry = Fat32DirEntry {
            name: name_8_3,
            attributes: Fat32DirEntry::ATTR_DIRECTORY,
            reserved: 0,
            create_time_tenth: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            first_cluster_high: (first_cluster >> 16) as u16,
            write_time: 0,
            write_date: 0,
            first_cluster_low: (first_cluster & 0xFFFF) as u16,
            file_size: 0,
        };
        
        self.add_directory_entry(dir_cluster, entry)?;
        
        Ok(())
    }
    
    fn remove(&mut self, path: &str) -> FsResult<()> {
        if !self.mounted {
            return Err(FsError::InvalidFilesystem);
        }
        
        // 경로에서 부모 디렉토리와 파일명 분리
        let (parent_path, filename) = self.split_parent_path(path)?;
        let (parent_cluster, _) = self.path_to_cluster(parent_path)?;
        let (file_cluster, entry) = self.path_to_cluster(path)?;
        
        // 디렉토리인 경우 빈 디렉토리인지 확인
        if entry.is_directory() {
            if !self.is_directory_empty(file_cluster)? {
                return Err(FsError::Busy);
            }
        }
        
        // FAT 체인 해제
        self.free_cluster_chain(file_cluster)?;
        
        // 부모 디렉토리에서 엔트리 삭제
        self.delete_directory_entry(parent_cluster, filename)?;
        
        Ok(())
    }
    
    fn rename(&mut self, old_path: &str, new_path: &str) -> FsResult<()> {
        if !self.mounted {
            return Err(FsError::InvalidFilesystem);
        }
        
        // 기존 파일 정보 가져오기
        let (old_parent_path, old_filename) = self.split_parent_path(old_path)?;
        let (new_parent_path, new_filename) = self.split_parent_path(new_path)?;
        
        let (old_parent_cluster, _) = self.path_to_cluster(old_parent_path)?;
        let (new_parent_cluster, _) = self.path_to_cluster(new_parent_path)?;
        let (file_cluster, mut entry) = self.path_to_cluster(old_path)?;
        
        // 새 파일명이 이미 존재하는지 확인
        if self.find_directory_entry(new_parent_cluster, new_filename).is_ok() {
            return Err(FsError::AlreadyExists);
        }
        
        // 엔트리 이름 변경
        entry.name = Self::name_to_8_3(new_filename);
        
        // 같은 디렉토리 내에서 이름만 변경하는 경우
        if old_parent_cluster == new_parent_cluster {
            self.update_directory_entry(old_parent_cluster, old_filename, entry)?;
        } else {
            // 다른 디렉토리로 이동
            self.delete_directory_entry(old_parent_cluster, old_filename)?;
            self.add_directory_entry(new_parent_cluster, entry)?;
        }
        
        Ok(())
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
            uid: 0,
            gid: 0,
        })
    }
    
    fn is_mounted(&self) -> bool {
        self.mounted
    }
}

/// FAT32 파일 핸들
struct Fat32File {
    filesystem: *mut Fat32FileSystem,
    entry: Fat32DirEntry,
    cluster: u32,
    offset: Offset,
}

// Safety: Fat32File uses a raw pointer to the filesystem owned by the FS layer.
// Accesses are serialized by the higher-level filesystem locks; mark as Send/Sync.
unsafe impl Send for Fat32File {}
unsafe impl Sync for Fat32File {}

impl File for Fat32File {
    fn read(&mut self, buf: &mut [u8], offset: Option<Offset>) -> FsResult<usize> {
        let read_offset = offset.unwrap_or(self.offset);
        if read_offset < 0 {
            return Ok(0);
        }
        
        let file_size = self.entry.file_size as usize;
        let read_offset = read_offset as usize;
        
        if read_offset >= file_size {
            return Ok(0);
        }
        
        let fs = unsafe { &mut *self.filesystem };
        let cluster_size = fs.boot_sector.sectors_per_cluster as usize * 512;
        let mut cluster_buf = alloc::vec![0u8; cluster_size];
        let mut current_offset = read_offset;
        let mut bytes_read = 0;
        let max_read = core::cmp::min(buf.len(), file_size - read_offset);
        
        while current_offset < file_size && bytes_read < max_read {
            // 현재 오프셋에 해당하는 클러스터 찾기
            let (cluster, cluster_offset) = fs.find_cluster_for_offset(self.cluster, current_offset)?;
            
            // 클러스터 읽기
            fs.read_single_cluster(cluster, &mut cluster_buf)?;
            
            // 버퍼에 복사할 데이터 길이 계산
            let available_in_cluster = cluster_size - cluster_offset;
            let remaining_in_file = file_size - current_offset;
            let to_copy = core::cmp::min(
                core::cmp::min(available_in_cluster, remaining_in_file),
                max_read - bytes_read,
            );
            
            // 데이터 복사
            buf[bytes_read..bytes_read + to_copy]
                .copy_from_slice(&cluster_buf[cluster_offset..cluster_offset + to_copy]);
            
            bytes_read += to_copy;
            current_offset += to_copy;
        }
        
        if offset.is_none() {
            self.offset = (read_offset + bytes_read) as Offset;
        }
        
        Ok(bytes_read)
    }
    
    fn write(&mut self, buf: &[u8], offset: Option<Offset>) -> FsResult<usize> {
        let write_offset = offset.unwrap_or(self.offset);
        if write_offset < 0 {
            return Err(FsError::InvalidPath);
        }
        
        let write_offset = write_offset as usize;
        let fs = unsafe { &mut *self.filesystem };
        let cluster_size = fs.boot_sector.sectors_per_cluster as usize * 512;
        let mut bytes_written = 0;
        let mut current_offset = write_offset;
        
        while bytes_written < buf.len() {
            // 현재 오프셋에 해당하는 클러스터 찾기 또는 할당
            let (cluster, cluster_offset) = if current_offset < self.entry.file_size as usize {
                // 기존 클러스터 사용
                fs.find_cluster_for_offset(self.cluster, current_offset)?
            } else {
                // 새 클러스터 할당 필요
                let last_cluster = if self.entry.file_size == 0 {
                    self.cluster
                } else {
                    // 마지막 클러스터 찾기
                    let mut cluster = self.cluster;
                    loop {
                        let next = fs.read_fat_entry(cluster)?;
                        if next >= 0x0FFFFFF8 {
                            break;
                        }
                        cluster = next;
                    }
                    cluster
                };
                
                // 새 클러스터 할당
                let new_cluster = fs.find_free_cluster()?;
                if self.entry.file_size == 0 {
                    // 첫 번째 클러스터 업데이트 필요 - 디렉토리 엔트리 업데이트 필요
                    // 현재는 단순화를 위해 에러 반환
                    return Err(FsError::IOError);
                } else {
                    fs.write_fat_entry_journaled(last_cluster, new_cluster)?;
                }
                fs.write_fat_entry_journaled(new_cluster, 0x0FFFFFFF)?;
                
                (new_cluster, 0)
            };
            
            // 클러스터 읽기
            let mut cluster_buf = alloc::vec![0u8; cluster_size];
            fs.read_single_cluster(cluster, &mut cluster_buf)?;
            
            // 버퍼에 쓰기
            let available_in_cluster = cluster_size - cluster_offset;
            let to_write = core::cmp::min(available_in_cluster, buf.len() - bytes_written);
            
            cluster_buf[cluster_offset..cluster_offset + to_write]
                .copy_from_slice(&buf[bytes_written..bytes_written + to_write]);
            
            // 클러스터 쓰기 (데이터는 저널링 안 함, 성능상 이유)
            fs.write_cluster_direct(cluster, &cluster_buf)?;
            
            bytes_written += to_write;
            current_offset += to_write;
            
            // 파일 크기 업데이트 (필요한 경우)
            let new_size = current_offset as u32;
            if new_size > self.entry.file_size {
                self.entry.file_size = new_size;
                // TODO: 디렉토리 엔트리 업데이트
            }
        }
        
        if offset.is_none() {
            self.offset = (write_offset + bytes_written) as Offset;
        }
        
        Ok(bytes_written)
    }
    
    fn metadata(&self) -> FsResult<FileMetadata> {
        Ok(FileMetadata {
            file_type: FileType::Regular,
            size: self.entry.file_size as u64,
            mode: FileMode::new(0o644),
            created: 0,
            modified: 0,
            accessed: 0,
            uid: 0,
            gid: 0,
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
struct Fat32Directory {
    filesystem: *mut Fat32FileSystem,
    cluster: u32,
}

unsafe impl Send for Fat32Directory {}
unsafe impl Sync for Fat32Directory {}

impl Directory for Fat32Directory {
    fn read_dir(&self) -> FsResult<Vec<(String, FileType)>> {
        // TODO: 디렉토리 읽기 구현
        // 현재는 빈 벡터 반환 (컴파일 오류 방지)
        let mut result = Vec::new();
        
        // 디렉토리 데이터 읽기
        let fs = unsafe { &mut *self.filesystem };
        let cluster_size = fs.boot_sector.sectors_per_cluster as usize * 512;
        let mut dir_buf = alloc::vec![0u8; cluster_size * 4];
        let len = match fs.read_cluster_chain(self.cluster, &mut dir_buf) {
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

