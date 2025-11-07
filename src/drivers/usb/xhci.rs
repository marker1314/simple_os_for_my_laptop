//! xHCI (eXtensible Host Controller Interface) 드라이버
//!
//! USB 3.0 호스트 컨트롤러를 지원합니다.
//!
//! # 참고 자료
//! - xHCI Specification Revision 1.2
//! - Intel xHCI Architecture Overview

use crate::drivers::pci::PciDevice;
use crate::drivers::usb::error::UsbError;
use crate::drivers::usb::host_controller::{UsbHostController, UsbHostControllerType};
use crate::drivers::usb::request::UsbControlRequest;
use crate::drivers::usb::xhci_trb::Trb;
use crate::memory::{allocate_frame, paging::get_physical_memory_offset};
use core::ptr::{read_volatile, write_volatile};
use x86_64::structures::paging::{Page, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

/// MMIO 레지스터 오프셋
const XHCI_CAPLENGTH: usize = 0x00;
const XHCI_HCSPARAMS1: usize = 0x04;
const XHCI_HCSPARAMS2: usize = 0x08;
const XHCI_HCSPARAMS3: usize = 0x0C;
const XHCI_HCCPARAMS1: usize = 0x10;
const XHCI_DOORBELL: usize = 0x00; // Doorbell Array Base (런타임 레지스터)
const XHCI_RUNTIME: usize = 0x00; // Runtime Register Space Base

/// Operational 레지스터 오프셋 (CAPLENGTH 이후)
const XHCI_USBCMD: usize = 0x00;
const XHCI_USBSTS: usize = 0x04;
const XHCI_PAGESIZE: usize = 0x08;
const XHCI_DNCTRL: usize = 0x14;
const XHCI_CRCR: usize = 0x18; // Command Ring Control Register
const XHCI_CONFIG: usize = 0x38;
const XHCI_PORTSC: usize = 0x400; // Port Status and Control (포트별 0x10 오프셋)

/// Interrupter 레지스터 오프셋 (Runtime Register Space)
const XHCI_IMAN: usize = 0x00; // Interrupter Management (Interrupter 0)
const XHCI_IMOD: usize = 0x04; // Interrupter Moderation
const XHCI_ERSTSZ: usize = 0x08; // Event Ring Segment Table Size
const XHCI_ERSTBA: usize = 0x10; // Event Ring Segment Table Base Address
const XHCI_ERDP: usize = 0x18; // Event Ring Dequeue Pointer

/// xHCI 명령 코드
const XHCI_CMD_RUN: u32 = 1 << 0;
const XHCI_CMD_HCRST: u32 = 1 << 1;
const XHCI_CMD_RS: u32 = 1 << 0;

/// xHCI 상태 비트
const XHCI_STS_HCH: u32 = 1 << 0; // Halted
const XHCI_STS_HSE: u32 = 1 << 2; // Host System Error
const XHCI_STS_EINT: u32 = 1 << 4; // Event Interrupt

/// 포트 상태 비트
const XHCI_PORTSC_CCS: u32 = 1 << 0; // Current Connect Status
const XHCI_PORTSC_PED: u32 = 1 << 1; // Port Enabled/Disabled
const XHCI_PORTSC_PR: u32 = 1 << 4; // Port Reset
const XHCI_PORTSC_PLS: u32 = 0xF << 5; // Port Link State

/// Command Ring 구조
struct CommandRing {
    /// Ring 버퍼 (가상 주소)
    buffer: *mut Trb,
    /// Ring 버퍼 (물리 주소)
    buffer_phys: PhysAddr,
    /// Ring 크기 (TRB 개수)
    size: usize,
    /// 현재 Dequeue Pointer
    dequeue_ptr: usize,
    /// Cycle State
    cycle_state: bool,
}

/// Event Ring 구조
struct EventRing {
    /// Ring 버퍼 (가상 주소)
    buffer: *mut Trb,
    /// Ring 버퍼 (물리 주소)
    buffer_phys: PhysAddr,
    /// Ring 크기 (TRB 개수)
    size: usize,
    /// 현재 Dequeue Pointer (읽은 위치)
    dequeue_ptr: usize,
    /// Cycle State
    cycle_state: bool,
}

impl EventRing {
    /// 새 Event Ring 생성
    unsafe fn new(size: usize) -> Result<Self, UsbError> {
        // 페이지 할당
        let frame = allocate_frame().ok_or(UsbError::DeviceError)?;
        let phys_addr = frame.start_address();
        
        // 가상 주소 매핑
        let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
        let virt_addr = phys_offset + phys_addr.as_u64();
        let buffer = virt_addr.as_mut_ptr::<Trb>();
        
        // 버퍼 초기화
        core::ptr::write_bytes(buffer, 0, size * core::mem::size_of::<Trb>());
        
        Ok(Self {
            buffer,
            buffer_phys: phys_addr,
            size,
            dequeue_ptr: 0,
            cycle_state: true,
        })
    }
    
    /// Event TRB 읽기 (다음 이벤트)
    unsafe fn read_event(&mut self) -> Option<Trb> {
        let trb_ptr = self.buffer.add(self.dequeue_ptr);
        let trb = core::ptr::read_volatile(trb_ptr);
        
        // Cycle bit 확인
        let cycle_bit = (trb.control & 1) != 0;
        if cycle_bit != self.cycle_state {
            // 이벤트가 없음 (Ring의 끝)
            return None;
        }
        
        // Dequeue Pointer 업데이트
        self.dequeue_ptr = (self.dequeue_ptr + 1) % self.size;
        
        // Ring의 끝에 도달하면 Cycle State 토글
        if self.dequeue_ptr == 0 {
            self.cycle_state = !self.cycle_state;
        }
        
        Some(trb)
    }
    
    /// Dequeue Pointer 가져오기 (물리 주소)
    fn dequeue_pointer(&self) -> PhysAddr {
        let offset = self.dequeue_ptr * core::mem::size_of::<Trb>();
        self.buffer_phys + offset as u64
    }
    
    /// 물리 주소 반환
    fn physical_address(&self) -> PhysAddr {
        self.buffer_phys
    }
}

impl CommandRing {
    /// 새 Command Ring 생성
    unsafe fn new(size: usize) -> Result<Self, UsbError> {
        // 페이지 할당 (4KB = 256 TRB, 16바이트씩)
        let frame = allocate_frame().ok_or(UsbError::DeviceError)?;
        let phys_addr = frame.start_address();
        
        // 가상 주소 매핑
        let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
        let virt_addr = phys_offset + phys_addr.as_u64();
        let buffer = virt_addr.as_mut_ptr::<Trb>();
        
        // 버퍼 초기화 (모든 TRB를 0으로)
        core::ptr::write_bytes(buffer, 0, size * core::mem::size_of::<Trb>());
        
        // 마지막 TRB를 Link TRB로 설정 (Ring을 순환시키기 위해)
        let last_trb = buffer.add(size - 1);
        let link_trb = Trb::new_link(phys_addr.as_u64(), true);
        core::ptr::write_volatile(last_trb, link_trb);
        
        Ok(Self {
            buffer,
            buffer_phys: phys_addr,
            size,
            dequeue_ptr: 0,
            cycle_state: true,
        })
    }
    
    /// TRB 추가
    fn add_trb(&mut self, trb: Trb) -> Result<(), UsbError> {
        if self.dequeue_ptr >= self.size - 1 {
            return Err(UsbError::DeviceError); // Ring이 가득 참
        }
        
        unsafe {
            let trb_ptr = self.buffer.add(self.dequeue_ptr);
            let mut trb_with_cycle = trb;
            // Control 필드의 Cycle bit 설정
            let mut control = core::ptr::read_volatile(&trb_with_cycle.control);
            if self.cycle_state {
                control |= 1;
            } else {
                control &= !1;
            }
            trb_with_cycle.control = control;
            core::ptr::write_volatile(trb_ptr, trb_with_cycle);
        }
        
        self.dequeue_ptr = (self.dequeue_ptr + 1) % (self.size - 1);
        
        Ok(())
    }
    
    /// Ring 물리 주소 반환
    fn physical_address(&self) -> PhysAddr {
        self.buffer_phys
    }
}

/// xHCI 호스트 컨트롤러
pub struct XhciController {
    /// PCI 디바이스
    pci_device: PciDevice,
    /// MMIO 베이스 주소
    mmio_base: u64,
    /// Capability 레지스터 베이스 (CAPLENGTH 값)
    cap_length: u8,
    /// Operational 레지스터 베이스
    op_base: u64,
    /// Runtime 레지스터 베이스
    runtime_base: u64,
    /// 초기화 여부
    initialized: bool,
    /// 포트 수
    port_count: u8,
    /// Command Ring
    command_ring: Option<CommandRing>,
    /// Event Ring
    event_ring: Option<EventRing>,
    /// Interrupter Register Base (Interrupter 0)
    interrupter_base: u64,
}

impl XhciController {
    /// 새 xHCI 컨트롤러 생성
    pub fn new(pci_device: PciDevice) -> Self {
        Self {
            pci_device,
            mmio_base: 0,
            command_ring: None,
            event_ring: None,
            interrupter_base: 0,
            cap_length: 0,
            op_base: 0,
            runtime_base: 0,
            initialized: false,
            port_count: 0,
            command_ring: None,
        }
    }
    
    /// MMIO 레지스터 읽기 (32비트)
    unsafe fn read_u32(&self, offset: usize) -> u32 {
        let addr = (self.mmio_base + offset as u64) as *const u32;
        read_volatile(addr)
    }
    
    /// MMIO 레지스터 쓰기 (32비트)
    unsafe fn write_u32(&self, offset: usize, value: u32) {
        let addr = (self.mmio_base + offset as u64) as *mut u32;
        write_volatile(addr, value);
    }
    
    /// Capability 레지스터 읽기
    unsafe fn read_cap(&self, offset: usize) -> u32 {
        self.read_u32(offset)
    }
    
    /// Operational 레지스터 읽기
    unsafe fn read_op(&self, offset: usize) -> u32 {
        self.read_u32(self.op_base as usize + offset)
    }
    
    /// Operational 레지스터 쓰기
    unsafe fn write_op(&self, offset: usize, value: u32) {
        self.write_u32(self.op_base as usize + offset, value);
    }
    
    /// 포트 상태 및 제어 레지스터 읽기
    unsafe fn read_portsc(&self, port: u8) -> u32 {
        if port == 0 || port > self.port_count {
            return 0;
        }
        let offset = XHCI_PORTSC + ((port - 1) as usize * 0x10);
        self.read_op(offset)
    }
    
    /// 포트 상태 및 제어 레지스터 쓰기
    unsafe fn write_portsc(&self, port: u8, value: u32) {
        if port == 0 || port > self.port_count {
            return;
        }
        let offset = XHCI_PORTSC + ((port - 1) as usize * 0x10);
        self.write_op(offset, value);
    }
    
    /// 베이스 주소 설정
    pub unsafe fn set_base_address(&mut self, base: u64) {
        self.mmio_base = base;
        
        // CAPLENGTH 읽기 (하위 8비트)
        self.cap_length = (self.read_cap(XHCI_CAPLENGTH) & 0xFF) as u8;
        
        // Operational 레지스터 베이스 = MMIO 베이스 + CAPLENGTH
        self.op_base = self.mmio_base + self.cap_length as u64;
        
        // HCCPARAMS1에서 Runtime Register Space Offset 읽기
        let hccparams1 = self.read_cap(XHCI_HCCPARAMS1);
        let rtso_offset = ((hccparams1 >> 5) & 0xFFFF) as u64;
        self.runtime_base = self.mmio_base + rtso_offset;
        
        // HCSPARAMS1에서 포트 수 읽기
        let hcsparams1 = self.read_cap(XHCI_HCSPARAMS1);
        self.port_count = ((hcsparams1 >> 0) & 0xFF) as u8;
        
        crate::log_info!(
            "xHCI: CAPLENGTH={}, OPS={:X}, RTSO={:X}, Ports={}",
            self.cap_length,
            self.op_base,
            self.runtime_base,
            self.port_count
        );
    }
    
    /// 컨트롤러 초기화
    pub unsafe fn initialize(&mut self) -> Result<(), UsbError> {
        if self.mmio_base == 0 {
            return Err(UsbError::DeviceNotFound);
        }
        
        crate::log_info!("Initializing xHCI controller...");
        
        // 1. 컨트롤러가 Halted 상태인지 확인
        let usbsts = self.read_op(XHCI_USBSTS);
        if (usbsts & XHCI_STS_HCH) == 0 {
            crate::log_warn!("xHCI controller is not halted, attempting to halt...");
            // Halt 컨트롤러
            let mut usbcmd = self.read_op(XHCI_USBCMD);
            usbcmd &= !XHCI_CMD_RUN;
            self.write_op(XHCI_USBCMD, usbcmd);
            
            // Halted 상태가 될 때까지 대기
            let mut timeout = 1000;
            while (self.read_op(XHCI_USBSTS) & XHCI_STS_HCH) == 0 && timeout > 0 {
                timeout -= 1;
                // 짧은 지연 (실제로는 더 정교한 지연 필요)
                for _ in 0..100 {
                    core::hint::spin_loop();
                }
            }
            
            if timeout == 0 {
                return Err(UsbError::DeviceError);
            }
        }
        
        // 2. 컨트롤러 리셋
        let mut usbcmd = self.read_op(XHCI_USBCMD);
        usbcmd |= XHCI_CMD_HCRST;
        self.write_op(XHCI_USBCMD, usbcmd);
        
        // 리셋 완료 대기
        let mut timeout = 10000;
        while (self.read_op(XHCI_USBCMD) & XHCI_CMD_HCRST) != 0 && timeout > 0 {
            timeout -= 1;
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }
        
        if timeout == 0 {
            return Err(UsbError::DeviceError);
        }
        
        // 3. Event Ring 초기화 (Command Ring보다 먼저)
        unsafe {
            let mut event_ring = EventRing::new(256)?; // 256 TRB
            let event_ring_phys = event_ring.physical_address();
            
            // Event Ring Segment Table (ERST) - 단일 세그먼트
            let erst_frame = allocate_frame().ok_or(UsbError::DeviceError)?;
            let erst_phys = erst_frame.start_address();
            let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
            let erst_virt = (phys_offset + erst_phys.as_u64()).as_mut_ptr::<u64>();
            
            // ERST 엔트리: [Ring Base Address, Ring Size, Reserved]
            core::ptr::write_volatile(erst_virt, event_ring_phys.as_u64());
            core::ptr::write_volatile(erst_virt.add(1), 256); // Size
            core::ptr::write_volatile(erst_virt.add(2), 0); // Reserved
            
            // Interrupter 0 레지스터 설정
            self.interrupter_base = self.runtime_base; // Interrupter 0
            
            // ERSTBA (Event Ring Segment Table Base Address)
            let erstba_low = (erst_phys.as_u64() & 0xFFFF_FFFF) as u32;
            let erstba_high = ((erst_phys.as_u64() >> 32) & 0xFFFF_FFFF) as u32;
            self.write_u32(self.interrupter_base as usize + XHCI_ERSTBA, erstba_low);
            self.write_u32(self.interrupter_base as usize + XHCI_ERSTBA + 4, erstba_high);
            
            // ERSTSZ (Event Ring Segment Table Size) = 1
            self.write_u32(self.interrupter_base as usize + XHCI_ERSTSZ, 1);
            
            // ERDP (Event Ring Dequeue Pointer)
            let erdp = event_ring.dequeue_pointer();
            let erdp_low = (erdp.as_u64() & 0xFFFF_FFFF) as u32 | (1 << 3); // EHB (Event Handler Busy) = 0
            let erdp_high = ((erdp.as_u64() >> 32) & 0xFFFF_FFFF) as u32;
            self.write_u32(self.interrupter_base as usize + XHCI_ERDP, erdp_low);
            self.write_u32(self.interrupter_base as usize + XHCI_ERDP + 4, erdp_high);
            
            // IMAN (Interrupter Management) - Interrupt 활성화
            let iman = (1 << 0) | (1 << 1); // IP (Interrupt Pending) clear, IE (Interrupt Enable)
            self.write_u32(self.interrupter_base as usize + XHCI_IMAN, iman);
            
            self.event_ring = Some(event_ring);
            crate::log_info!("xHCI: Event Ring initialized at {:#016X}", event_ring_phys.as_u64());
        }
        
        // 4. Command Ring 초기화
        unsafe {
            let mut command_ring = CommandRing::new(256)?; // 256 TRB
            let ring_phys = command_ring.physical_address();
            
            // Command Ring Control Register 설정
            // CRCR[63:6] = Ring Base Address
            // CRCR[5:4] = Reserved
            // CRCR[3] = RCS (Ring Cycle State)
            // CRCR[2] = CS (Command Stop)
            // CRCR[1] = CA (Command Abort)
            // CRCR[0] = RCS (Ring Cycle State)
            let crcr_low = (ring_phys.as_u64() & 0xFFFF_FFFF) as u32 | (1 << 0); // RCS = 1
            let crcr_high = ((ring_phys.as_u64() >> 32) & 0xFFFF_FFFF) as u32;
            
            self.write_op(XHCI_CRCR, crcr_low);
            self.write_op(XHCI_CRCR + 4, crcr_high);
            
            self.command_ring = Some(command_ring);
            crate::log_info!("xHCI: Command Ring initialized at {:#016X}", ring_phys.as_u64());
        }
        
        // 5. 컨트롤러 시작 (Run)
        let mut usbcmd = self.read_op(XHCI_USBCMD);
        usbcmd |= XHCI_CMD_RUN;
        self.write_op(XHCI_USBCMD, usbcmd);
        
        // Running 상태 대기
        let mut timeout = 1000;
        while (self.read_op(XHCI_USBSTS) & XHCI_STS_HCH) != 0 && timeout > 0 {
            timeout -= 1;
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }
        
        // 6. 포트 상태 확인
        crate::log_info!("xHCI: Checking {} ports...", self.port_count);
        for port in 1..=self.port_count {
            let portsc = self.read_portsc(port);
            if (portsc & XHCI_PORTSC_CCS) != 0 {
                crate::log_info!("xHCI: Port {} has device connected", port);
            }
        }
        
        self.initialized = true;
        crate::log_info!("xHCI controller initialized successfully");
        
        Ok(())
    }
    
    /// 포트 연결 상태 확인
    pub unsafe fn check_port_connection(&self, port: u8) -> Result<bool, UsbError> {
        if !self.initialized {
            return Err(UsbError::NotInitialized);
        }
        
        if port == 0 || port > self.port_count {
            return Err(UsbError::InvalidParameter);
        }
        
        let portsc = self.read_portsc(port);
        Ok((portsc & XHCI_PORTSC_CCS) != 0)
    }
    
    /// 포트 리셋
    pub unsafe fn reset_port(&self, port: u8) -> Result<(), UsbError> {
        if !self.initialized {
            return Err(UsbError::NotInitialized);
        }
        
        if port == 0 || port > self.port_count {
            return Err(UsbError::InvalidParameter);
        }
        
        let mut portsc = self.read_portsc(port);
        
        // 포트 리셋 시작
        portsc |= XHCI_PORTSC_PR;
        self.write_portsc(port, portsc);
        
        // 리셋 완료 대기
        let mut timeout = 10000;
        while (self.read_portsc(port) & XHCI_PORTSC_PR) != 0 && timeout > 0 {
            timeout -= 1;
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }
        
        if timeout == 0 {
            return Err(UsbError::DeviceError);
        }
        
        Ok(())
    }
    
    /// 포트 수 가져오기
    pub fn port_count(&self) -> u8 {
        self.port_count
    }

    /// Interrupt IN 전송으로 데이터 수신 (스켈레톤)
    /// 현재 디바이스/엔드포인트 컨텍스트 및 트랜스퍼 링 설정이 미구현이므로
    /// 안전하게 NotImplemented를 반환합니다.
    pub unsafe fn recv_interrupt_in(
        &mut self,
        _endpoint_address: u8,
        _data_buffer: *mut u8,
        _data_length: u16,
    ) -> Result<(), UsbError> {
        if !self.initialized {
            return Err(UsbError::NotInitialized);
        }

        // 입력 인자 검증
        if _data_buffer.is_null() || _data_length == 0 {
            return Err(UsbError::InvalidParameter);
        }

        // 현재 구현은 간소화를 위해 Command Ring을 전송 큐로 재사용합니다.
        // 실제 xHCI에서는 각 엔드포인트별 Transfer Ring이 필요합니다.
        let command_ring = self.command_ring.as_mut().ok_or(UsbError::DeviceError)?;

        // 데이터 버퍼의 물리 주소를 계산
        let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
        let virt_addr = _data_buffer as u64;
        let data_buffer_phys = virt_addr.wrapping_sub(phys_offset.as_u64());

        // Normal Transfer TRB (IN) 작성: 길이는 _data_length, 완료 인터럽트 요청
        let in_trb = Trb::new_normal_transfer(
            data_buffer_phys,
            _data_length as u32,
            true,
        );
        command_ring.add_trb(in_trb)?;

        // Doorbell: 엔드포인트 주소의 하위 4비트가 EP 번호
        // Slot 0 가정 (스켈레톤), EP 번호는 1..15. 주소 0인 경우 EP0로 강하.
        let ep_num = (_endpoint_address & 0x0F) as u32;
        let doorbell_ep = if ep_num == 0 { 1 } else { ep_num };

        // 런타임 Doorbell 베이스에서 Slot 0의 Doorbell을 사용 (스켈레톤)
        let doorbell_offset = 0x800; // Slot 0 Doorbell
        self.write_u32(self.runtime_base as usize + doorbell_offset, doorbell_ep);

        // Event Ring에서 Transfer Event를 기다림
        let event_ring = self.event_ring.as_mut().ok_or(UsbError::DeviceError)?;
        let mut timeout = 10000u32;
        let mut completion_received = false;
        while !completion_received && timeout > 0 {
            timeout -= 1;

            if let Some(event_trb) = event_ring.read_event() {
                let trb_type = ((event_trb.control >> 4) & 0x3F) as u8;
                if trb_type == 32 { // Transfer Event
                    let completion_code = ((event_trb.parameter2 >> 24) & 0xFF) as u8;
                    if completion_code == 0x01 || completion_code == 0x00 || completion_code == 0x0D {
                        completion_received = true;
                    } else {
                        crate::log_warn!("xHCI: Interrupt IN completed with code: 0x{:02X}", completion_code);
                        // Short packet 등 일부 코드는 허용
                        if completion_code == 0x0D { completion_received = true; }
                    }
                }

                // ERDP 업데이트 (이벤트 처리 완료 알림)
                let erdp = event_ring.dequeue_pointer();
                let erdp_low = (erdp.as_u64() & 0xFFFF_FFFF) as u32;
                let erdp_high = ((erdp.as_u64() >> 32) & 0xFFFF_FFFF) as u32;
                self.write_u32(self.interrupter_base as usize + XHCI_ERDP, erdp_low);
                self.write_u32(self.interrupter_base as usize + XHCI_ERDP + 4, erdp_high);
            }

            // 짧은 지연
            for _ in 0..10 { core::hint::spin_loop(); }
        }

        if !completion_received {
            return Err(UsbError::DeviceError);
        }

        Ok(())
    }
}

impl UsbHostController for XhciController {
    fn init(&mut self) -> Result<(), UsbError> {
        unsafe {
            // PCI 버스 마스터 활성화
            let command = self.pci_device.read_config_register(0x04);
            self.pci_device.write_config_register(0x04, command | 0x05);
            
            // 베이스 주소 읽기
            let bar0 = self.pci_device.bar0;
            if (bar0 & 0x01) == 0 {
                // MMIO
                let base = (bar0 & !0xF) as u64;
                self.set_base_address(base);
            } else {
                return Err(UsbError::DeviceError); // xHCI는 MMIO만 지원
            }
            
            // 초기화
            self.initialize()
        }
    }
    
    fn reset(&mut self) -> Result<(), UsbError> {
        unsafe {
            let mut usbcmd = self.read_op(XHCI_USBCMD);
            usbcmd |= XHCI_CMD_HCRST;
            self.write_op(XHCI_USBCMD, usbcmd);
        }
        Ok(())
    }
    
    /// Interrupt IN 전송을 통해 엔드포인트에서 데이터를 수신합니다.
    ///
    /// 간단 구현: Command Ring에 Normal TRB를 게시하고 해당 EP Doorbell을 울린 뒤
    /// Event Ring에서 완료 코드를 폴링합니다. 실제 xHCI에서는 각 EP별 Transfer Ring이
    /// 필요하지만, 본 구현은 단순화를 위해 동일 링을 재사용합니다.
    pub unsafe fn recv_interrupt_in(
        &mut self,
        endpoint_address: u8,
        data_buffer: *mut u8,
        data_length: u16,
    ) -> Result<(), UsbError> {
        if !self.initialized {
            return Err(UsbError::NotInitialized);
        }
        if data_buffer.is_null() || data_length == 0 {
            return Err(UsbError::InvalidParam);
        }

        let command_ring = self.command_ring.as_mut().ok_or(UsbError::DeviceError)?;

        // 버퍼의 물리 주소 계산 (정적 매핑 가정)
        let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
        let virt_addr = data_buffer as u64;
        let data_buffer_phys = virt_addr.saturating_sub(phys_offset.as_u64());

        // Normal Transfer TRB (IN) – 길이 만큼 읽기, 완료 시 인터럽트 요청
        let in_trb = Trb::new_normal_transfer(
            data_buffer_phys,
            data_length as u32,
            true,
        );
        command_ring.add_trb(in_trb)?;

        // Doorbell: Slot 0, 대상 엔드포인트 번호 (하위 4비트)
        let ep_num = (endpoint_address & 0x0F) as u32;
        let doorbell_offset = 0x800; // Slot 0 Doorbell
        self.write_u32(self.runtime_base as usize + doorbell_offset, ep_num);

        // Event Ring 완료 대기
        let event_ring = self.event_ring.as_mut().ok_or(UsbError::DeviceError)?;
        let mut timeout = 10000;
        while timeout > 0 {
            timeout -= 1;
            if let Some(event_trb) = event_ring.read_event() {
                let trb_type = ((event_trb.control >> 4) & 0x3F) as u8;
                if trb_type == 32 { // Transfer Event
                    let completion_code = ((event_trb.parameter2 >> 24) & 0xFF) as u8;
                    // 0x01: Success, 0x13: Short Packet 등 일부 정상 코드 허용
                    if completion_code == 0x01 || completion_code == 0x13 || completion_code == 0x00 {
                        // ERDP 업데이트
                        let erdp = event_ring.dequeue_pointer();
                        let erdp_low = (erdp.as_u64() & 0xFFFF_FFFF) as u32;
                        let erdp_high = ((erdp.as_u64() >> 32) & 0xFFFF_FFFF) as u32;
                        self.write_u32(self.interrupter_base as usize + XHCI_ERDP, erdp_low);
                        self.write_u32(self.interrupter_base as usize + XHCI_ERDP + 4, erdp_high);
                        return Ok(());
                    } else {
                        crate::log_warn!("xHCI: Interrupt IN completion code: 0x{:02X}", completion_code);
                    }
                }
                // ERDP 갱신 (이벤트 소비)
                let erdp = event_ring.dequeue_pointer();
                let erdp_low = (erdp.as_u64() & 0xFFFF_FFFF) as u32;
                let erdp_high = ((erdp.as_u64() >> 32) & 0xFFFF_FFFF) as u32;
                self.write_u32(self.interrupter_base as usize + XHCI_ERDP, erdp_low);
                self.write_u32(self.interrupter_base as usize + XHCI_ERDP + 4, erdp_high);
            }
            core::hint::spin_loop();
        }

        Err(UsbError::Timeout)
    }

    /// USB 제어 요청 전송
    ///
    /// # Arguments
    /// * `request` - USB 제어 요청
    /// * `data_buffer` - 데이터 버퍼 (IN/OUT)
    /// * `data_length` - 데이터 길이
    ///
    /// # Safety
    /// 컨트롤러가 초기화되어 있어야 합니다.
    pub unsafe fn send_control_request(
        &mut self,
        request: &UsbControlRequest,
        data_buffer: *mut u8,
        data_length: u16,
    ) -> Result<(), UsbError> {
        if !self.initialized {
            return Err(UsbError::NotInitialized);
        }
        
        let command_ring = self.command_ring.as_mut().ok_or(UsbError::DeviceError)?;
        
        // Setup Stage TRB 생성
        let setup_data = core::slice::from_raw_parts(
            request as *const UsbControlRequest as *const u8,
            8,
        );
        let setup_array: [u8; 8] = [
            setup_data[0], setup_data[1], setup_data[2], setup_data[3],
            setup_data[4], setup_data[5], setup_data[6], setup_data[7],
        ];
        let setup_trb = Trb::new_setup_stage(&setup_array, false);
        command_ring.add_trb(setup_trb)?;
        
        // Data Stage TRB (데이터가 있는 경우)
        if data_length > 0 && data_buffer != core::ptr::null_mut() {
            // 데이터 버퍼의 물리 주소 가져오기
            let phys_offset = get_physical_memory_offset(crate::boot::get_boot_info());
            let virt_addr = data_buffer as u64;
            let data_buffer_phys = virt_addr - phys_offset.as_u64();
            
            let direction = (request.request_type & 0x80) != 0; // Device to Host
            let data_trb = Trb::new_data_stage(
                data_buffer_phys,
                data_length as u32,
                direction,
                false,
            );
            command_ring.add_trb(data_trb)?;
        }
        
        // Status Stage TRB
        let direction = (request.request_type & 0x80) != 0; // Device to Host
        let status_trb = Trb::new_status_stage(!direction, true); // Status는 반대 방향
        command_ring.add_trb(status_trb)?;
        
        // Doorbell 레지스터를 통해 명령 전송 (Slot 0, EP 0)
        // Doorbell은 Runtime Register Space에 있음
        let doorbell_offset = 0x800; // Slot 0 Doorbell
        let doorbell_value = 1; // EP 0
        self.write_u32(self.runtime_base as usize + doorbell_offset, doorbell_value);
        
        // Event Ring에서 완료 이벤트 대기
        let event_ring = self.event_ring.as_mut().ok_or(UsbError::DeviceError)?;
        let mut timeout = 10000;
        let mut completion_received = false;
        
        while !completion_received && timeout > 0 {
            timeout -= 1;
            
            // Event Ring에서 이벤트 읽기
            if let Some(event_trb) = event_ring.read_event() {
                // TRB Type 확인 (Control 필드의 비트 4-15)
                let trb_type = ((event_trb.control >> 4) & 0x3F) as u8;
                
                // Transfer Event (Type 32) 또는 Command Completion Event (Type 33)
                if trb_type == 32 || trb_type == 33 {
                    // Completion Code 확인 (Parameter 2의 비트 24-31)
                    let completion_code = ((event_trb.parameter2 >> 24) & 0xFF) as u8;
                    
                    // Success (0x01) 또는 다른 성공 코드
                    if completion_code == 0x01 || completion_code == 0x00 {
                        completion_received = true;
                        crate::log_debug!("xHCI: Control request completed successfully");
                    } else {
                        crate::log_warn!("xHCI: Control request completed with code: 0x{:02X}", completion_code);
                        // 일부 오류 코드는 허용 가능
                        if completion_code == 0x0D { // Short Packet (정상적인 완료)
                            completion_received = true;
                        }
                    }
                }
                
                // ERDP 업데이트 (Event Ring Dequeue Pointer)
                let erdp = event_ring.dequeue_pointer();
                let erdp_low = (erdp.as_u64() & 0xFFFF_FFFF) as u32; // EHB = 0 (이벤트 처리 완료)
                let erdp_high = ((erdp.as_u64() >> 32) & 0xFFFF_FFFF) as u32;
                self.write_u32(self.interrupter_base as usize + XHCI_ERDP, erdp_low);
                self.write_u32(self.interrupter_base as usize + XHCI_ERDP + 4, erdp_high);
            }
            
            // 짧은 지연
            for _ in 0..10 {
                core::hint::spin_loop();
            }
        }
        
        if !completion_received {
            crate::log_warn!("xHCI: Control request timeout");
            return Err(UsbError::DeviceError);
        }
        
        Ok(())
    }
    
    /// Event Ring에서 이벤트 처리 (주기적 호출)
    pub unsafe fn process_events(&mut self) -> Result<(), UsbError> {
        let event_ring = self.event_ring.as_mut().ok_or(UsbError::DeviceError)?;
        
        while let Some(event_trb) = event_ring.read_event() {
            // TRB Type 확인
            let trb_type = ((event_trb.control >> 4) & 0x3F) as u8;
            
            match trb_type {
                32 => {
                    // Transfer Event
                    let completion_code = ((event_trb.parameter2 >> 24) & 0xFF) as u8;
                    crate::log_debug!("xHCI: Transfer Event, code: 0x{:02X}", completion_code);
                }
                33 => {
                    // Command Completion Event
                    let slot_id = ((event_trb.parameter2 >> 24) & 0xFF) as u8;
                    crate::log_debug!("xHCI: Command Completion Event, slot: {}", slot_id);
                }
                34 => {
                    // Port Status Change Event
                    let port_id = ((event_trb.parameter0 >> 24) & 0xFF) as u8;
                    crate::log_info!("xHCI: Port {} status changed", port_id);
                }
                _ => {
                    crate::log_debug!("xHCI: Unknown event type: {}", trb_type);
                }
            }
            
            // ERDP 업데이트
            let erdp = event_ring.dequeue_pointer();
            let erdp_low = (erdp.as_u64() & 0xFFFF_FFFF) as u32;
            let erdp_high = ((erdp.as_u64() >> 32) & 0xFFFF_FFFF) as u32;
            self.write_u32(self.interrupter_base as usize + XHCI_ERDP, erdp_low);
            self.write_u32(self.interrupter_base as usize + XHCI_ERDP + 4, erdp_high);
        }
        
        Ok(())
    }
}

impl UsbHostController for XhciController {
    fn controller_type(&self) -> UsbHostControllerType {
        UsbHostControllerType::Xhci
    }
    
    fn is_running(&self) -> bool {
        self.initialized
    }
}

