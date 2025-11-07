//! HDA (High Definition Audio) 드라이버
//!
//! Intel HD Audio 스펙을 구현합니다.
//!
//! # 참고 자료
//! - Intel HD Audio Specification Revision 1.0a

use crate::drivers::pci::PciDevice;
use crate::drivers::audio::AudioError;
use crate::drivers::audio::hda_codec::{HdaCodec, HdaVerbCommand, CodecParameter};
use crate::memory::{allocate_frame, paging::get_physical_memory_offset};
use alloc::vec::Vec;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::Size4KiB;
use core::ptr::{read_volatile, write_volatile};

/// HDA 레지스터 오프셋
const HDA_GCAP: usize = 0x00;
const HDA_GCTL: usize = 0x08;
const HDA_WAKEEN: usize = 0x0C;
const HDA_STATESTS: usize = 0x0E;
const HDA_CORBLBASE: usize = 0x40;
const HDA_CORBUBASE: usize = 0x44;
const HDA_CORBWP: usize = 0x48;
const HDA_CORBRP: usize = 0x4A;
const HDA_CORBCTL: usize = 0x4C;
const HDA_CORBSIZE: usize = 0x4E;
const HDA_RIRBLBASE: usize = 0x50;
const HDA_RIRBUBASE: usize = 0x54;
const HDA_RIRBWP: usize = 0x58;
const HDA_RINTCNT: usize = 0x5A;
const HDA_RIRBCTL: usize = 0x5C;
const HDA_RIRBSIZE: usize = 0x5E;
const HDA_CORB_MAX_SIZE: usize = 0x64; // 256 entries
const HDA_RIRB_MAX_SIZE: usize = 0x64; // 256 entries

/// Stream Descriptor 레지스터 오프셋 (스트림별 0x80 바이트)
const HDA_SD_BASE: usize = 0x80; // Stream 0 Base
const HDA_SD_OFFSET: usize = 0x80; // 각 스트림 간 오프셋
const HDA_SDCTL: usize = 0x00; // Stream Descriptor Control
const HDA_SDSTS: usize = 0x03; // Stream Descriptor Status
const HDA_SDLPIB: usize = 0x04; // Stream Descriptor Last Valid Index Position
const HDA_SDCBL: usize = 0x08; // Stream Descriptor Cyclic Buffer Length
const HDA_SDLVI: usize = 0x0C; // Stream Descriptor Last Valid Index
const HDA_SDFIFOW: usize = 0x0E; // Stream Descriptor FIFO Write
const HDA_SDFIFOS: usize = 0x10; // Stream Descriptor FIFO Size
const HDA_SDFMT: usize = 0x12; // Stream Descriptor Format
const HDA_SDBDPL: usize = 0x18; // Stream Descriptor Buffer Descriptor List Pointer (Low)
const HDA_SDBDPU: usize = 0x1C; // Stream Descriptor Buffer Descriptor List Pointer (High)

/// SDCTL 비트
const HDA_SDCTL_RUN: u8 = 1 << 1; // Run
const HDA_SDCTL_IOCE: u8 = 1 << 2; // Interrupt On Completion Enable
const HDA_SDCTL_FEIE: u8 = 1 << 3; // FIFO Error Interrupt Enable
const HDA_SDCTL_DEIE: u8 = 1 << 5; // Descriptor Error Interrupt Enable

/// BDL (Buffer Descriptor List) 엔트리 구조
#[repr(C, packed)]
struct BdlEntry {
    /// 버퍼 주소 (64-bit)
    address_low: u32,
    address_high: u32,
    /// 버퍼 길이 (32-bit)
    length: u32,
    /// IOC (Interrupt On Completion) 플래그
    ioc: u32,
}

/// GCTL 비트
const HDA_GCTL_RESET: u32 = 1 << 0;
const HDA_GCTL_FCNTRL: u32 = 1 << 1;

/// CORB/RIRB 제어 비트
const HDA_CORBCTL_RUN: u32 = 1 << 1;
const HDA_RIRBCTL_RUN: u32 = 1 << 1;
const HDA_RIRBCTL_IRQ: u32 = 1 << 0;

/// CORB (Command Output Ring Buffer) 구조
struct Corb {
    /// CORB 버퍼 (가상 주소)
    buffer: *mut u32,
    /// CORB 버퍼 (물리 주소)
    buffer_phys: PhysAddr,
    /// CORB 크기 (엔트리 수)
    size: usize,
    /// Write Pointer
    write_ptr: u16,
    /// Read Pointer (하드웨어가 관리)
    read_ptr: u16,
}

impl Corb {
    /// 새 CORB 생성
    unsafe fn new(size: usize) -> Result<Self, AudioError> {
        // 페이지 할당
        let frame = allocate_frame().ok_or(AudioError::InitFailed)?;
        let phys_addr = frame.start_address();
        
        // 가상 주소 매핑
        let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
        let virt_addr = phys_offset + phys_addr.as_u64();
        let buffer = virt_addr.as_mut_ptr::<u32>();
        
        // 버퍼 초기화
        core::ptr::write_bytes(buffer, 0, size * core::mem::size_of::<u32>());
        
        Ok(Self {
            buffer,
            buffer_phys: phys_addr,
            size,
            write_ptr: 0,
            read_ptr: 0,
        })
    }
    
    /// Verb 명령 추가
    fn add_verb(&mut self, verb: u32) -> Result<(), AudioError> {
        let next_ptr = (self.write_ptr as usize + 1) % self.size;
        if next_ptr == self.read_ptr as usize {
            return Err(AudioError::InitFailed); // CORB가 가득 참
        }
        
        unsafe {
            let entry = self.buffer.add(self.write_ptr as usize);
            write_volatile(entry, verb);
        }
        
        self.write_ptr = next_ptr as u16;
        Ok(())
    }
    
    /// 물리 주소 반환
    fn physical_address(&self) -> PhysAddr {
        self.buffer_phys
    }
}

/// RIRB (Response Input Ring Buffer) 구조
struct Rirb {
    /// RIRB 버퍼 (가상 주소)
    buffer: *mut u64,
    /// RIRB 버퍼 (물리 주소)
    buffer_phys: PhysAddr,
    /// RIRB 크기 (엔트리 수)
    size: usize,
    /// Write Pointer (하드웨어가 관리)
    write_ptr: u16,
    /// Read Pointer
    read_ptr: u16,
}

impl Rirb {
    /// 새 RIRB 생성
    unsafe fn new(size: usize) -> Result<Self, AudioError> {
        // 페이지 할당
        let frame = allocate_frame().ok_or(AudioError::InitFailed)?;
        let phys_addr = frame.start_address();
        
        // 가상 주소 매핑
        let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
        let virt_addr = phys_offset + phys_addr.as_u64();
        let buffer = virt_addr.as_mut_ptr::<u64>();
        
        // 버퍼 초기화
        core::ptr::write_bytes(buffer, 0, size * core::mem::size_of::<u64>());
        
        Ok(Self {
            buffer,
            buffer_phys: phys_addr,
            size,
            write_ptr: 0,
            read_ptr: 0,
        })
    }
    
    /// 응답 읽기
    fn read_response(&mut self) -> Option<u32> {
        let next_ptr = (self.read_ptr as usize + 1) % self.size;
        if next_ptr == self.write_ptr as usize {
            return None; // 응답 없음
        }
        
        unsafe {
            let entry = self.buffer.add(self.read_ptr as usize);
            let response = read_volatile(entry);
            self.read_ptr = next_ptr as u16;
            
            // 응답의 하위 32비트 반환 (상위 32비트는 상태 정보)
            Some((response & 0xFFFF_FFFF) as u32)
        }
    }
    
    /// 물리 주소 반환
    fn physical_address(&self) -> PhysAddr {
        self.buffer_phys
    }
}

/// HDA 컨트롤러
pub struct HdaController {
    /// PCI 디바이스
    pci_device: PciDevice,
    /// MMIO 베이스 주소 (물리)
    base_address: PhysAddr,
    /// MMIO 베이스 주소 (가상)
    virt_base: Option<VirtAddr>,
    /// 초기화 여부
    initialized: bool,
    /// 검색된 코덱 수
    codec_count: u8,
    /// 검색된 코덱 목록
    codecs: Vec<HdaCodec>,
    /// CORB
    corb: Option<Corb>,
    /// RIRB
    rirb: Option<Rirb>,
    /// Output BDL 물리 주소
    output_bdl: Option<PhysAddr>,
    /// Output PCM 버퍼 물리 주소
    output_buffer: Option<PhysAddr>,
    /// Output PCM 버퍼 크기 (바이트)
    output_buffer_size: Option<usize>,
}

impl HdaController {
    /// 새 HDA 컨트롤러 생성
    pub fn new(pci_device: PciDevice) -> Result<Self, AudioError> {
        // BAR0에서 MMIO 베이스 주소 읽기
        let bar0 = pci_device.bar0;
        let base_address = if (bar0 & 0x01) == 0 {
            // MMIO 공간
            PhysAddr::new((bar0 & !0xF) as u64)
        } else {
            return Err(AudioError::InitFailed);
        };
        
        Ok(Self {
            pci_device,
            base_address,
            virt_base: None,
            initialized: false,
            codec_count: 0,
            codecs: Vec::new(),
            corb: None,
            rirb: None,
            output_bdl: None,
            output_buffer: None,
            output_buffer_size: None,
        })
    }
    
    /// 가상 주소로 변환
    unsafe fn get_virt_base(&mut self) -> Result<VirtAddr, AudioError> {
        if let Some(virt) = self.virt_base {
            return Ok(virt);
        }
        
        // 물리 메모리 오프셋 가져오기
        use crate::memory::paging;
        let offset = {
            let guard = paging::PHYSICAL_MEMORY_OFFSET.lock();
            guard.ok_or(AudioError::InitFailed)?
        };
        
        let virt = offset + self.base_address.as_u64();
        self.virt_base = Some(virt);
        Ok(virt)
    }
    
    /// MMIO 레지스터 읽기 (32비트)
    unsafe fn read_u32(&mut self, offset: usize) -> Result<u32, AudioError> {
        let virt_base = self.get_virt_base()?;
        let addr = (virt_base.as_u64() + offset as u64) as *const u32;
        Ok(read_volatile(addr))
    }
    
    /// MMIO 레지스터 쓰기 (32비트)
    unsafe fn write_u32(&mut self, offset: usize, value: u32) -> Result<(), AudioError> {
        let virt_base = self.get_virt_base()?;
        let addr = (virt_base.as_u64() + offset as u64) as *mut u32;
        write_volatile(addr, value);
        Ok(())
    }
    
    /// MMIO 레지스터 읽기 (16비트)
    unsafe fn read_u16(&mut self, offset: usize) -> Result<u16, AudioError> {
        let virt_base = self.get_virt_base()?;
        let addr = (virt_base.as_u64() + offset as u64) as *const u16;
        Ok(read_volatile(addr))
    }
    
    /// MMIO 레지스터 쓰기 (16비트)
    unsafe fn write_u16(&mut self, offset: usize, value: u16) -> Result<(), AudioError> {
        let virt_base = self.get_virt_base()?;
        let addr = (virt_base.as_u64() + offset as u64) as *mut u16;
        write_volatile(addr, value);
        Ok(())
    }
    
    /// HDA 컨트롤러 초기화
    ///
    /// # Safety
    /// PCI 버스가 초기화된 후에 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), AudioError> {
        if self.initialized {
            return Ok(());
        }
        
        // PCI 버스 마스터 활성화
        let command = self.pci_device.read_config_register(0x04);
        self.pci_device.write_config_register(0x04, command | 0x05); // Bus Master + Memory Space
        
        crate::log_info!("HDA controller MMIO base: {:#016X}", self.base_address.as_u64());
        
        // 1. Global Capabilities 읽기
        let gcap = self.read_u32(HDA_GCAP)?;
        let num_input_streams = ((gcap >> 8) & 0x0F) as u8;
        let num_output_streams = ((gcap >> 12) & 0x0F) as u8;
        let num_bidir_streams = ((gcap >> 16) & 0x0F) as u8;
        crate::log_info!("HDA: Input streams={}, Output streams={}, Bidir streams={}",
                        num_input_streams, num_output_streams, num_bidir_streams);
        
        // 2. GCTL 리셋
        let mut gctl = self.read_u32(HDA_GCTL)?;
        if (gctl & HDA_GCTL_RESET) == 0 {
            crate::log_info!("HDA: Resetting controller...");
            gctl |= HDA_GCTL_RESET;
            self.write_u32(HDA_GCTL, gctl)?;
            
            // 리셋 완료 대기
            let mut timeout = 10000;
            while (self.read_u32(HDA_GCTL)? & HDA_GCTL_RESET) == 0 && timeout > 0 {
                timeout -= 1;
                for _ in 0..100 {
                    core::hint::spin_loop();
                }
            }
            
            if timeout == 0 {
                return Err(AudioError::InitFailed);
            }
        }
        
        // 3. 리셋 해제
        gctl = self.read_u32(HDA_GCTL)?;
        gctl &= !HDA_GCTL_RESET;
        self.write_u32(HDA_GCTL, gctl)?;
        
        // 리셋 해제 대기
        let mut timeout = 10000;
        while (self.read_u32(HDA_GCTL)? & HDA_GCTL_RESET) != 0 && timeout > 0 {
            timeout -= 1;
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }
        
        // 4. CORB/RIRB 초기화 (코덱과 통신하기 위해)
        unsafe {
            self.init_corb_rirb()?;
        }
        
        // 5. 코덱 검색 및 초기화
        self.scan_and_init_codecs()?;
        
        self.initialized = true;
        crate::log_info!("HDA controller initialized successfully ({} codec(s))", self.codec_count);
        Ok(())
    }
    
    /// CORB/RIRB 초기화
    unsafe fn init_corb_rirb(&mut self) -> Result<(), AudioError> {
        // CORB 생성 (256 엔트리)
        let mut corb = Corb::new(256)?;
        let corb_phys = corb.physical_address();
        
        // RIRB 생성 (256 엔트리)
        let mut rirb = Rirb::new(256)?;
        let rirb_phys = rirb.physical_address();
        
        // CORB Base Address 설정
        self.write_u32(HDA_CORBLBASE, (corb_phys.as_u64() & 0xFFFF_FFFF) as u32)?;
        self.write_u32(HDA_CORBUBASE, ((corb_phys.as_u64() >> 32) & 0xFFFF_FFFF) as u32)?;
        
        // CORB Size 설정 (256 엔트리 = 0x02)
        self.write_u16(HDA_CORBSIZE, 0x02)?; // Size = 256
        
        // CORB Write Pointer 초기화
        self.write_u16(HDA_CORBWP, 0)?;
        
        // CORB Read Pointer 초기화 (하드웨어가 관리)
        self.write_u16(HDA_CORBRP, 0)?;
        
        // CORB Control 활성화
        self.write_u16(HDA_CORBCTL, HDA_CORBCTL_RUN)?;
        
        // RIRB Base Address 설정
        self.write_u32(HDA_RIRBLBASE, (rirb_phys.as_u64() & 0xFFFF_FFFF) as u32)?;
        self.write_u32(HDA_RIRBUBASE, ((rirb_phys.as_u64() >> 32) & 0xFFFF_FFFF) as u32)?;
        
        // RIRB Size 설정 (256 엔트리 = 0x02)
        self.write_u16(HDA_RIRBSIZE, 0x02)?; // Size = 256
        
        // RIRB Write Pointer 초기화 (하드웨어가 관리)
        self.write_u16(HDA_RIRBWP, 0)?;
        
        // RIRB Response Count 초기화
        self.write_u16(HDA_RINTCNT, 1)?; // 인터럽트마다 응답 확인
        
        // RIRB Control 활성화
        self.write_u16(HDA_RIRBCTL, HDA_RIRBCTL_RUN | HDA_RIRBCTL_IRQ)?;
        
        self.corb = Some(corb);
        self.rirb = Some(rirb);
        
        crate::log_info!("HDA: CORB/RIRB initialized (CORB: {:#016X}, RIRB: {:#016X})", 
                        corb_phys.as_u64(), rirb_phys.as_u64());
        
        Ok(())
    }
    
    /// 코덱 검색 및 초기화
    unsafe fn scan_and_init_codecs(&mut self) -> Result<(), AudioError> {
        // STATESTS 레지스터에서 연결된 코덱 확인
        let statests = self.read_u32(HDA_STATESTS)?;
        
        // 각 비트는 코덱 번호를 나타냄 (0-14)
        self.codec_count = 0;
        self.codecs.clear();
        
        for i in 0..15 {
            if (statests & (1 << i)) != 0 {
                self.codec_count += 1;
                crate::log_info!("HDA: Codec {} detected", i);
                
                // 코덱 초기화
                if let Ok(mut codec) = self.init_codec(i) {
                    self.codecs.push(codec);
                } else {
                    crate::log_warn!("HDA: Failed to initialize codec {}", i);
                }
            }
        }
        
        if self.codec_count == 0 {
            crate::log_warn!("HDA: No codecs detected");
        }
        
        Ok(())
    }
    
    /// 코덱 초기화
    unsafe fn init_codec(&mut self, codec_id: CodecId) -> Result<HdaCodec, AudioError> {
        let mut codec = HdaCodec::new(codec_id);
        
        // 1. Vendor ID 읽기
        let vendor_verb = HdaVerbCommand::GetParameter(0, CodecParameter::VendorId as u16);
        let vendor_response = self.send_verb(codec_id, vendor_verb)?;
        codec.vendor_id = vendor_response;
        
        // 2. Revision ID 읽기
        let revision_verb = HdaVerbCommand::GetParameter(0, CodecParameter::RevisionId as u16);
        codec.revision_id = self.send_verb(codec_id, revision_verb)?;
        
        // 3. Subsystem ID 읽기
        let subsystem_verb = HdaVerbCommand::GetParameter(0, CodecParameter::SubsystemId as u16);
        codec.subsystem_id = self.send_verb(codec_id, subsystem_verb)?;
        
        // 4. Node Count 읽기
        let node_count_verb = HdaVerbCommand::GetParameter(0, CodecParameter::NodeCount as u16);
        let node_count_response = self.send_verb(codec_id, node_count_verb)?;
        codec.node_count = ((node_count_response >> 16) & 0xFF) as u32;
        
        crate::log_info!("HDA Codec {}: VID=0x{:08X}, REV=0x{:08X}, Nodes={}", 
                        codec_id, codec.vendor_id, codec.revision_id, codec.node_count);
        
        // 5. Audio Function Group 찾기
        for node_id in 1..=codec.node_count as u16 {
            let fg_type_verb = HdaVerbCommand::GetParameter(node_id, CodecParameter::FunctionGroupType as u16);
            let fg_type = self.send_verb(codec_id, fg_type_verb)?;
            
            // Audio Function Group (Type = 0x01)
            if (fg_type & 0xFF) == 0x01 {
                codec.audio_function_group = Some(node_id);
                crate::log_info!("HDA Codec {}: Audio Function Group found at node {}", codec_id, node_id);
                
                // Audio Function Group Capabilities 읽기
                let afg_cap_verb = HdaVerbCommand::GetParameter(node_id, CodecParameter::AudioFunctionGroupCap as u16);
                let _afg_cap = self.send_verb(codec_id, afg_cap_verb)?;
                
                // Output/Input Stream 노드 찾기
                // 실제로는 Connection List를 읽어야 하지만, 간단한 구현
                break;
            }
        }
        
        Ok(codec)
    }
    
    /// Verb 명령 전송
    ///
    /// # Arguments
    /// * `codec_id` - 코덱 ID
    /// * `verb` - Verb 명령
    ///
    /// # Safety
    /// CORB/RIRB가 초기화되어 있어야 합니다.
    unsafe fn send_verb(&mut self, codec_id: CodecId, verb: HdaVerbCommand) -> Result<u32, AudioError> {
        let corb = self.corb.as_mut().ok_or(AudioError::InitFailed)?;
        let rirb = self.rirb.as_mut().ok_or(AudioError::InitFailed)?;
        
        // Verb를 32비트 값으로 변환
        let verb_value = verb.to_u32(codec_id);
        
        // CORB에 Verb 추가
        corb.add_verb(verb_value)?;
        
        // CORB Write Pointer 업데이트
        self.write_u16(HDA_CORBWP, corb.write_ptr)?;
        
        // RIRB에서 응답 대기
        let mut timeout = 10000;
        while timeout > 0 {
            if let Some(response) = rirb.read_response() {
                // RIRB Read Pointer 업데이트
                self.write_u16(HDA_RIRBRP, rirb.read_ptr)?;
                return Ok(response);
            }
            
            timeout -= 1;
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }
        
        Err(AudioError::InitFailed) // 타임아웃
    }
    
    /// 초기화 여부 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// 베이스 주소 가져오기
    pub fn base_address(&self) -> PhysAddr {
        self.base_address
    }
    
    /// PCM 출력 스트림 설정
    ///
    /// # Arguments
    /// * `stream_id` - Stream Descriptor ID (0부터 시작, 출력 스트림)
    /// * `format` - PCM 포맷
    /// * `buffer` - 오디오 데이터 버퍼 (물리 주소)
    /// * `buffer_size` - 버퍼 크기 (바이트)
    pub unsafe fn setup_pcm_output(
        &mut self,
        stream_id: u8,
        format: &crate::drivers::audio::pcm::PcmFormat,
        buffer: PhysAddr,
        buffer_size: usize,
    ) -> Result<(), AudioError> {
        if !self.initialized {
            return Err(AudioError::InitFailed);
        }
        
        // Stream Descriptor 오프셋 계산
        let sd_offset = HDA_SD_BASE + (stream_id as usize * HDA_SD_OFFSET);
        
        // 1. BDL (Buffer Descriptor List) 생성
        let bdl_frame = allocate_frame().ok_or(AudioError::InitFailed)?;
        let bdl_phys = bdl_frame.start_address();
        let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
        let bdl_virt = (phys_offset + bdl_phys.as_u64()).as_mut_ptr::<BdlEntry>();
        
        // 1a. Cyclic BDL: 4 엔트리로 분할하여 순환 (각 엔트리 마지막은 IOC)
        let segments = 4usize;
        let seg_len = (buffer_size / segments) & !0x7; // 8바이트 정렬
        for i in 0..segments {
            let addr = buffer.as_u64() + (i * seg_len) as u64;
            let entry = BdlEntry {
                address_low: (addr & 0xFFFF_FFFF) as u32,
                address_high: ((addr >> 32) & 0xFFFF_FFFF) as u32,
                length: if i == segments - 1 { (buffer_size - seg_len * (segments - 1)) as u32 } else { seg_len as u32 },
                ioc: 1, // 각 엔트리 완료 시 인터럽트 요청 (간단화)
            };
            unsafe { core::ptr::write_volatile(bdl_virt.add(i), entry); }
        }
        
        // BDL 주소 설정 (Stream Descriptor 레지스터)
        self.write_u32(sd_offset + HDA_SDBDPL, bdl_phys.as_u64() as u32)?;
        self.write_u32(sd_offset + HDA_SDBDPU, ((bdl_phys.as_u64() >> 32) & 0xFFFF_FFFF) as u32)?;
        
        // 2. Cyclic Buffer Length 설정
        self.write_u32(sd_offset + HDA_SDCBL, buffer_size as u32)?;
        
        // 3. Last Valid Index 설정 (BDL 엔트리 수 - 1)
        self.write_u16(sd_offset + HDA_SDLVI, (segments as u16 - 1))?;
        
        // 4. Format 설정
        // Format: [23:20] = Sample Rate, [19:16] = Sample Bits, [15:8] = Channels, [7:0] = Base Rate
        let sample_rate_code = match format.sample_rate {
            8000 => 0x00,
            11025 => 0x01,
            16000 => 0x02,
            22050 => 0x03,
            32000 => 0x04,
            44100 => 0x05,
            48000 => 0x06,
            88200 => 0x07,
            96000 => 0x08,
            176400 => 0x09,
            192000 => 0x0A,
            _ => 0x06, // 기본값: 48kHz
        };
        
        let sample_bits_code = match format.sample_size {
            8 => 0x00,
            16 => 0x01,
            20 => 0x02,
            24 => 0x03,
            32 => 0x04,
            _ => 0x01, // 기본값: 16-bit
        };
        
        let channels_code = (format.channels.saturating_sub(1)) as u16;
        let format_value = ((sample_rate_code as u16) << 20)
            | ((sample_bits_code as u16) << 16)
            | ((channels_code as u16) << 8)
            | (sample_rate_code as u16);
        
        self.write_u32(sd_offset + HDA_SDFMT, format_value as u32)?;
        
        // 5. Last Valid Index Position 초기화
        self.write_u32(sd_offset + HDA_SDLPIB, 0)?;
        
        self.output_bdl = Some(bdl_phys);
        self.output_buffer = Some(buffer);
        self.output_buffer_size = Some(buffer_size);
        
        crate::log_info!("HDA: PCM output setup: stream={}, rate={}Hz, bits={}, channels={}",
                        stream_id, format.sample_rate, format.sample_size, format.channels);
        
        Ok(())
    }

    /// PCM 출력 버퍼에 데이터 쓰기 및 재생 위치 업데이트
    pub unsafe fn write_pcm_data(&mut self, stream_id: u8, offset: usize, data: &[u8]) -> Result<(), AudioError> {
        if !self.initialized { return Err(AudioError::InitFailed); }
        let buf_phys = self.output_buffer.ok_or(AudioError::NotInitialized)?;
        let buf_size = self.output_buffer_size.ok_or(AudioError::NotInitialized)?;
        if offset >= buf_size { return Err(AudioError::BufferTooSmall); }
        let write_len = core::cmp::min(data.len(), buf_size - offset);
        let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
        let dst_ptr = (phys_offset + buf_phys.as_u64() + offset as u64).as_mut_ptr::<u8>();
        core::ptr::copy_nonoverlapping(data.as_ptr(), dst_ptr, write_len);
        // 재생 위치 레지스터 업데이트 (LPIB)
        let sd_offset = HDA_SD_BASE + (stream_id as usize * HDA_SD_OFFSET);
        // LPIB는 마지막 유효 인덱스 위치를 바이트 단위로 쓸 수 있는 구현이 있는 제품도 있으나, 보수적으로 바이트 위치 기록
        self.write_u32(sd_offset + HDA_SDLPIB, (offset + write_len) as u32)?;
        Ok(())
    }
    
    /// PCM 출력 스트림 시작
    pub unsafe fn start_pcm_output(&mut self, stream_id: u8) -> Result<(), AudioError> {
        if !self.initialized {
            return Err(AudioError::InitFailed);
        }
        
        let sd_offset = HDA_SD_BASE + (stream_id as usize * HDA_SD_OFFSET);
        
        // Stream Descriptor Control 읽기
        let mut sdctl = self.read_u8(sd_offset + HDA_SDCTL)?;
        
        // Run 비트 설정
        sdctl |= HDA_SDCTL_RUN;
        sdctl |= HDA_SDCTL_IOCE; // Interrupt On Completion Enable
        
        self.write_u8(sd_offset + HDA_SDCTL, sdctl)?;
        
        // Last Valid Index Position을 0으로 설정 (처음부터 재생)
        self.write_u32(sd_offset + HDA_SDLPIB, 0)?;
        
        crate::log_info!("HDA: PCM output stream {} started", stream_id);
        Ok(())
    }
    
    /// PCM 출력 스트림 중지
    pub unsafe fn stop_pcm_output(&mut self, stream_id: u8) -> Result<(), AudioError> {
        if !self.initialized {
            return Err(AudioError::InitFailed);
        }
        
        let sd_offset = HDA_SD_BASE + (stream_id as usize * HDA_SD_OFFSET);
        
        // Stream Descriptor Control 읽기
        let mut sdctl = self.read_u8(sd_offset + HDA_SDCTL)?;
        
        // Run 비트 클리어
        sdctl &= !HDA_SDCTL_RUN;
        
        self.write_u8(sd_offset + HDA_SDCTL, sdctl)?;
        
        crate::log_info!("HDA: PCM output stream {} stopped", stream_id);
        Ok(())
    }
    
    /// 간단한 PCM 진행 상황 폴링 및 언더런 감시 (임시)
    /// 드라이버가 IOC를 처리하기 전까지, LPIB를 주기적으로 확인하여 진행 정지 시 경고를 남깁니다.
    pub unsafe fn poll_pcm_progress(&mut self, stream_id: u8) -> Result<(), AudioError> {
        if !self.initialized { return Err(AudioError::InitFailed); }
        let sd_offset = HDA_SD_BASE + (stream_id as usize * HDA_SD_OFFSET);
        let lpib = self.read_u32(sd_offset + HDA_SDLPIB)?;
        // 간단한 진행 기록 (통계 모듈 연동 시 계수 증가)
        crate::log_debug!("HDA: LPIB={} (stream {})", lpib, stream_id);
        Ok(())
    }
    
    /// MMIO 레지스터 읽기 (8비트)
    unsafe fn read_u8(&mut self, offset: usize) -> Result<u8, AudioError> {
        let virt_base = self.get_virt_base()?;
        let addr = (virt_base.as_u64() + offset as u64) as *const u8;
        Ok(read_volatile(addr))
    }
    
    /// MMIO 레지스터 쓰기 (8비트)
    unsafe fn write_u8(&mut self, offset: usize, value: u8) -> Result<(), AudioError> {
        let virt_base = self.get_virt_base()?;
        let addr = (virt_base.as_u64() + offset as u64) as *mut u8;
        write_volatile(addr, value);
        Ok(())
    }

    /// 간단한 언더런 복구: 스트림 포지션이 버퍼 길이를 초과하거나 정지 시 재시작
    pub unsafe fn recover_pcm_underrun(&mut self, stream_id: u8) -> Result<(), AudioError> {
        if !self.initialized { return Err(AudioError::InitFailed); }
        let sd_offset = HDA_SD_BASE + (stream_id as usize * HDA_SD_OFFSET);
        let lpib = self.read_u32(sd_offset + HDA_SDLPIB)?;
        let cbl = self.read_u32(sd_offset + HDA_SDCBL)?;
        if lpib >= cbl {
            // 정지 후 재시작
            let mut sdctl = self.read_u8(sd_offset + HDA_SDCTL)?;
            sdctl &= !HDA_SDCTL_RUN;
            self.write_u8(sd_offset + HDA_SDCTL, sdctl)?;
            self.write_u32(sd_offset + HDA_SDLPIB, 0)?;
            sdctl |= HDA_SDCTL_RUN | HDA_SDCTL_IOCE;
            self.write_u8(sd_offset + HDA_SDCTL, sdctl)?;
            crate::log_warn!("HDA: underrun recovered on stream {}", stream_id);
        }
        Ok(())
    }
}

