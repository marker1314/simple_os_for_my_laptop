//! RTL8139 이더넷 컨트롤러 드라이버
//!
//! 이 모듈은 Realtek RTL8139 이더넷 네트워크 카드의 드라이버를 구현합니다.
//! RTL8139는 PCI 기반 이더넷 컨트롤러로, QEMU에서도 기본 지원됩니다.

use crate::drivers::pci::PciDevice;
use crate::net::ethernet::{EthernetDriver, NetworkError, MacAddress, PacketBuffer};
use crate::memory::{allocate_frame, paging};
use crate::boot::info;
use x86_64::instructions::port::Port;
use x86_64::VirtAddr;
use x86_64::structures::paging::PhysFrame;

/// RTL8139 PCI 벤더 ID
const RTL8139_VENDOR_ID: u16 = 0x10EC;
/// RTL8139 PCI 디바이스 ID
const RTL8139_DEVICE_ID: u16 = 0x8139;

/// RTL8139 레지스터 오프셋 (IO 공간 기준)
const RTL8139_MAC0: u16 = 0x00;      // MAC 주소 (0-3)
const RTL8139_MAC4: u16 = 0x04;      // MAC 주소 (4-5)
const RTL8139_COMMAND: u16 = 0x37;   // Command Register
const RTL8139_IMR: u16 = 0x3C;       // Interrupt Mask Register
const RTL8139_ISR: u16 = 0x3E;       // Interrupt Status Register
const RTL8139_TCR: u16 = 0x40;       // Transmit Configuration Register
const RTL8139_RCR: u16 = 0x44;       // Receive Configuration Register
const RTL8139_TXSTATUS0: u16 = 0x10; // Transmit Status 0
const RTL8139_TXSTATUS1: u16 = 0x14; // Transmit Status 1
const RTL8139_TXSTATUS2: u16 = 0x18; // Transmit Status 2
const RTL8139_TXSTATUS3: u16 = 0x1C; // Transmit Status 3
const RTL8139_TXADDR0: u16 = 0x20;  // Transmit Address 0
const RTL8139_TXADDR1: u16 = 0x24;  // Transmit Address 1
const RTL8139_TXADDR2: u16 = 0x28;  // Transmit Address 2
const RTL8139_TXADDR3: u16 = 0x2C;  // Transmit Address 3
const RTL8139_RX_BUF: u16 = 0x30;   // Receive Buffer Start Address
const RTL8139_RX_BUF_PTR: u16 = 0x38; // Receive Buffer Pointer
const RTL8139_CAPR: u16 = 0x38;      // Current Address of Packet Read
const RTL8139_CBR: u16 = 0x3A;       // Current Buffer Address Register

/// Command Register 비트
const CMD_RESET: u8 = 0x10;
const CMD_RX_ENABLE: u8 = 0x08;
const CMD_TX_ENABLE: u8 = 0x04;
const CMD_RX_BUF_EMPTY: u8 = 0x01;

/// Interrupt Status Register 비트
const ISR_TOK: u16 = 0x0004;  // Transmit OK
const ISR_ROK: u16 = 0x0001;  // Receive OK
const ISR_TER: u16 = 0x0008;  // Transmit Error
const ISR_RER: u16 = 0x0002;  // Receive Error

/// Interrupt Mask Register 비트
const IMR_TOK: u16 = 0x0004;  // Transmit OK
const IMR_ROK: u16 = 0x0001;  // Receive OK
const IMR_TER: u16 = 0x0008;  // Transmit Error
const IMR_RER: u16 = 0x0002;  // Receive Error

/// Receive Configuration Register 비트
const RCR_APM: u32 = 0x00000080;  // Accept Physical Match
const RCR_AM: u32 = 0x00000040;   // Accept Multicast
const RCR_AB: u32 = 0x00000020;   // Accept Broadcast
const RCR_WRAP: u32 = 0x00000010; // Wrap
const RCR_NO_RX: u32 = 0x00000008; // No Receive
const RCR_MXDMA_SHIFT: u32 = 8;   // Max DMA Burst Size

/// Transmit Configuration Register 비트
const TCR_IFG_SHIFT: u32 = 24;    // Inter-Frame Gap
const TCR_HWVERID_SHIFT: u32 = 16; // Hardware Version ID
const TCR_MXDMA_SHIFT: u32 = 8;   // Max DMA Burst Size

/// 수신 버퍼 크기 (8KB)
const RX_BUFFER_SIZE: usize = 8192;
/// 최대 패킷 크기
const MAX_PACKET_SIZE: usize = 1518;
/// 송신 디스크립터 수
const TX_DESCRIPTOR_COUNT: usize = 4;
/// 최소 패킷 크기 (이더넷 최소 프레임 크기)
const MIN_PACKET_SIZE: usize = 60;

/// TXSTATUS 레지스터 비트
const TXSTATUS_OWN: u32 = 0x2000_0000;  // Owner bit (1 = NIC owns, 0 = driver owns)
const TXSTATUS_DSIZE_MASK: u32 = 0x0000_07FF; // Data size mask
const TXSTATUS_CRS: u32 = 0x0000_8000;  // Carrier Sense Lost
const TXSTATUS_TABT: u32 = 0x0000_4000; // Transmit Abort
const TXSTATUS_OWC: u32 = 0x0000_2000;  // Out of Window Collision
const TXSTATUS_CDH: u32 = 0x0000_1000;  // CD Heart Beat
const TXSTATUS_OK: u32 = 0x0000_0800;   // Transmit OK

/// 송신 버퍼 디스크립터
struct TxDescriptor {
    /// 물리 프레임 (None이면 사용 가능)
    frame: Option<PhysFrame>,
    /// 가상 주소 (물리 주소를 가상 주소로 변환한 것)
    virt_addr: Option<*mut u8>,
}

/// RTL8139 드라이버 구조체
pub struct Rtl8139Driver {
    /// IO 베이스 주소
    io_base: u16,
    /// MAC 주소
    mac_address: MacAddress,
    /// 수신 버퍼 프레임 (물리 메모리)
    rx_buffer_frame: Option<PhysFrame>,
    /// 수신 버퍼 가상 주소
    rx_buffer_virt: Option<*mut u8>,
    /// 현재 수신 버퍼 포인터
    rx_current: u16,
    /// 송신 디스크립터 (4개)
    tx_descriptors: [TxDescriptor; TX_DESCRIPTOR_COUNT],
    /// 현재 사용할 송신 디스크립터 인덱스
    tx_current: usize,
    /// 초기화 여부
    initialized: bool,
    /// PCI 디바이스 정보
    pci_device: PciDevice,
    /// 저전력 수신 정지 상태
    low_power: bool,
}

impl Rtl8139Driver {
    /// 새 RTL8139 드라이버 생성
    pub fn new(pci_device: PciDevice) -> Self {
        // BAR0에서 IO 베이스 주소 추출
        let bar0 = pci_device.bar0;
        let io_base = (bar0 & 0xFFFC) as u16; // 하위 2비트는 IO 공간 플래그
        
        Self {
            io_base,
            mac_address: MacAddress([0; 6]),
            rx_buffer_frame: None,
            rx_buffer_virt: None,
            rx_current: 0,
            tx_descriptors: [
                TxDescriptor { frame: None, virt_addr: None },
                TxDescriptor { frame: None, virt_addr: None },
                TxDescriptor { frame: None, virt_addr: None },
                TxDescriptor { frame: None, virt_addr: None },
            ],
            tx_current: 0,
            initialized: false,
            pci_device,
            low_power: false,
        }
    }
    
    /// IO 포트에서 8비트 읽기
    unsafe fn read_u8(&self, offset: u16) -> u8 {
        let mut port: Port<u8> = Port::new(self.io_base + offset);
        port.read()
    }
    
    /// IO 포트에 8비트 쓰기
    unsafe fn write_u8(&self, offset: u16, value: u8) {
        let mut port: Port<u8> = Port::new(self.io_base + offset);
        port.write(value);
    }
    
    /// IO 포트에서 16비트 읽기
    unsafe fn read_u16(&self, offset: u16) -> u16 {
        let mut port: Port<u16> = Port::new(self.io_base + offset);
        port.read()
    }
    
    /// IO 포트에 16비트 쓰기
    unsafe fn write_u16(&self, offset: u16, value: u16) {
        let mut port: Port<u16> = Port::new(self.io_base + offset);
        port.write(value);
    }
    
    /// IO 포트에서 32비트 읽기
    unsafe fn read_u32(&self, offset: u16) -> u32 {
        let mut port: Port<u32> = Port::new(self.io_base + offset);
        port.read()
    }
    
    /// IO 포트에 32비트 쓰기
    unsafe fn write_u32(&self, offset: u16, value: u32) {
        let mut port: Port<u32> = Port::new(self.io_base + offset);
        port.write(value);
    }
    
    /// MAC 주소 읽기
    unsafe fn read_mac_address(&mut self) -> MacAddress {
        let mac0 = self.read_u32(RTL8139_MAC0);
        let mac4 = self.read_u16(RTL8139_MAC4);
        
        MacAddress([
            (mac0 & 0xFF) as u8,
            ((mac0 >> 8) & 0xFF) as u8,
            ((mac0 >> 16) & 0xFF) as u8,
            ((mac0 >> 24) & 0xFF) as u8,
            (mac4 & 0xFF) as u8,
            ((mac4 >> 8) & 0xFF) as u8,
        ])
    }
    
    /// 리셋 수행
    unsafe fn reset(&self) {
        // 리셋 비트 설정
        let mut cmd = self.read_u8(RTL8139_COMMAND);
        cmd |= CMD_RESET;
        self.write_u8(RTL8139_COMMAND, cmd);
        
        // 리셋 완료 대기 (최대 100ms)
        let mut timeout = 100000;
        while timeout > 0 {
            cmd = self.read_u8(RTL8139_COMMAND);
            if (cmd & CMD_RESET) == 0 {
                break;
            }
            timeout -= 1;
            // 짧은 대기 (대략 1마이크로초)
            x86_64::instructions::nop();
        }
        
        if timeout == 0 {
            crate::log_error!("RTL8139 reset timeout");
        }
    }
    
    /// 수신 버퍼 설정
    unsafe fn setup_rx_buffer(&mut self) -> Result<(), NetworkError> {
        // 수신 버퍼 할당 (8KB = 2 프레임, 8바이트 정렬 필요)
        // RTL8139는 8KB 버퍼를 사용하므로 2개의 4KB 프레임 할당
        let frame1 = allocate_frame().ok_or(NetworkError::BufferFull)?;
        let frame2 = allocate_frame().ok_or(NetworkError::BufferFull)?;
        
        // 첫 번째 프레임의 물리 주소를 사용 (RTL8139는 연속된 물리 메모리 필요)
        // 실제로는 8KB가 연속된 물리 메모리여야 하지만, 간단한 구현을 위해
        // 첫 번째 프레임만 사용하고 4KB 버퍼로 제한합니다
        // TODO: 연속된 8KB 물리 메모리 할당 구현 필요
        
        let phys_addr = frame1.start_address();
        self.rx_buffer_frame = Some(frame1);
        
        // 물리 주소를 가상 주소로 변환
        let boot_info = info::get();
        let phys_offset = paging::get_physical_memory_offset(boot_info);
        let virt_addr = phys_offset + phys_addr.as_u64();
        self.rx_buffer_virt = Some(virt_addr.as_mut_ptr());
        
        // 수신 버퍼 시작 주소 설정 (물리 주소)
        self.write_u32(RTL8139_RX_BUF, phys_addr.as_u64() as u32);
        
        // 수신 버퍼 포인터 초기화
        self.rx_current = 0;
        self.write_u16(RTL8139_RX_BUF_PTR, 0);
        
        crate::log_info!("RTL8139 RX buffer allocated at physical 0x{:X}", phys_addr.as_u64());
        
        Ok(())
    }
    
    /// 송신 버퍼 할당
    unsafe fn allocate_tx_buffer(&mut self) -> Result<usize, NetworkError> {
        // 사용 가능한 송신 디스크립터 찾기
        for i in 0..TX_DESCRIPTOR_COUNT {
            let idx = (self.tx_current + i) % TX_DESCRIPTOR_COUNT;
            let desc = &mut self.tx_descriptors[idx];
            
            // 디스크립터가 비어있거나 사용 가능한지 확인
            if desc.frame.is_none() {
                // 새 프레임 할당
                let frame = allocate_frame().ok_or(NetworkError::BufferFull)?;
                let phys_addr = frame.start_address();
                
                // 물리 주소를 가상 주소로 변환
                let boot_info = info::get();
                let phys_offset = paging::get_physical_memory_offset(boot_info);
                let virt_addr = phys_offset + phys_addr.as_u64();
                
                desc.frame = Some(frame);
                desc.virt_addr = Some(virt_addr.as_mut_ptr());
                
                return Ok(idx);
            }
            
            // 디스크립터가 사용 중인지 확인 (TXSTATUS의 OWN 비트 확인)
            let txstatus_reg = match idx {
                0 => RTL8139_TXSTATUS0,
                1 => RTL8139_TXSTATUS1,
                2 => RTL8139_TXSTATUS2,
                3 => RTL8139_TXSTATUS3,
                _ => unreachable!(),
            };
            
            let status = self.read_u32(txstatus_reg);
            if (status & TXSTATUS_OWN) == 0 {
                // 드라이버가 소유권을 가지고 있음 (전송 완료 또는 에러)
                // 버퍼를 재사용 가능
                return Ok(idx);
            }
        }
        
        // 모든 디스크립터가 사용 중
        Err(NetworkError::BufferFull)
    }
    
    /// 수신 설정
    unsafe fn setup_receive(&self) {
        // Receive Configuration Register 설정
        let mut rcr = 0u32;
        rcr |= RCR_APM;  // Accept Physical Match (MAC 주소 일치)
        rcr |= RCR_AM;   // Accept Multicast
        rcr |= RCR_AB;   // Accept Broadcast
        rcr |= RCR_WRAP; // Wrap mode
        rcr |= (256 << RCR_MXDMA_SHIFT); // Max DMA Burst Size: 256 bytes (reduce interrupts)
        
        self.write_u32(RTL8139_RCR, rcr);
    }

    /// 수신 정지 (저전력)
    unsafe fn stop_receive(&self) {
        // RCR_NO_RX 설정으로 Rx 차단
        let mut rcr = self.read_u32(RTL8139_RCR);
        rcr |= RCR_NO_RX;
        self.write_u32(RTL8139_RCR, rcr);
        // 수신 관련 인터럽트 마스크 해제
        let mut imr = self.read_u16(RTL8139_IMR);
        imr &= !IMR_ROK;
        self.write_u16(RTL8139_IMR, imr);
    }

    /// 수신 재개
    unsafe fn resume_receive(&self) {
        let mut rcr = self.read_u32(RTL8139_RCR);
        rcr &= !RCR_NO_RX;
        self.write_u32(RTL8139_RCR, rcr);
        // 인터럽트 복원
        let imr = IMR_TOK | IMR_ROK | IMR_TER | IMR_RER;
        self.write_u16(RTL8139_IMR, imr);
    }
    
    /// 송신 설정
    unsafe fn setup_transmit(&self) {
        // Transmit Configuration Register 설정
        let mut tcr = 0u32;
        tcr |= (3 << TCR_IFG_SHIFT); // Inter-Frame Gap: 3
        tcr |= (256 << TCR_MXDMA_SHIFT); // Max DMA Burst Size: 256 bytes
        
        self.write_u32(RTL8139_TCR, tcr);
    }
    
    /// 인터럽트 마스크 설정
    unsafe fn setup_interrupts(&self) {
        // 인터럽트 마스크 설정 (송수신 OK, 에러 인터럽트 활성화)
        let imr = IMR_TOK | IMR_ROK | IMR_TER | IMR_RER;
        self.write_u16(RTL8139_IMR, imr);
    }
    
    /// 디바이스 활성화
    unsafe fn enable(&self) {
        let mut cmd = self.read_u8(RTL8139_COMMAND);
        cmd |= CMD_RX_ENABLE | CMD_TX_ENABLE;
        self.write_u8(RTL8139_COMMAND, cmd);
    }
}

impl EthernetDriver for Rtl8139Driver {
    fn name(&self) -> &str {
        "RTL8139"
    }
    
    unsafe fn init(&mut self, pci_device: &PciDevice) -> Result<(), NetworkError> {
        // PCI 디바이스 검증
        if pci_device.vendor_id != RTL8139_VENDOR_ID || 
           pci_device.device_id != RTL8139_DEVICE_ID {
            return Err(NetworkError::DeviceNotFound);
        }
        
        crate::log_info!("Initializing RTL8139 at IO base 0x{:04X}", self.io_base);
        
        // PCI 버스 마스터 및 IO 공간 활성화
        let command = pci_device.read_config_register(0x04);
        pci_device.write_config_register(0x04, command | 0x05); // Bus Master + IO Space
        
        // 리셋
        self.reset();
        
        // MAC 주소 읽기
        self.mac_address = self.read_mac_address();
        crate::log_info!("RTL8139 MAC address: {}", self.mac_address);
        
        // 수신 버퍼 설정
        self.setup_rx_buffer()?;
        
        // 수신/송신 설정
        self.setup_receive();
        self.setup_transmit();
        
        // 인터럽트 설정
        self.setup_interrupts();
        
        // 디바이스 활성화
        self.enable();
        
        self.initialized = true;
        crate::log_info!("RTL8139 initialized successfully");
        
        Ok(())
    }
    
    fn get_mac_address(&self) -> Result<MacAddress, NetworkError> {
        if !self.initialized {
            return Err(NetworkError::NotInitialized);
        }
        Ok(self.mac_address)
    }
    
    fn send_packet(&mut self, packet: &PacketBuffer) -> Result<(), NetworkError> {
        if !self.initialized {
            return Err(NetworkError::NotInitialized);
        }
        
        if packet.length > MAX_PACKET_SIZE {
            return Err(NetworkError::InvalidPacket);
        }
        
        unsafe {
            // 활동 기록 및 필요시 수신 재개
            self.low_power = false;
            self.resume_receive();
            // 1. 송신 버퍼 할당
            let desc_idx = self.allocate_tx_buffer()?;
            let desc = &mut self.tx_descriptors[desc_idx];
            
            let virt_addr = desc.virt_addr.expect("TX buffer not allocated");
            let frame = desc.frame.expect("TX frame not allocated");
            let phys_addr = frame.start_address();
            
            // 2. 패킷 데이터를 송신 버퍼에 복사
            // 최소 패킷 크기 보장 (패딩)
            let packet_len = packet.length.max(MIN_PACKET_SIZE);
            let buffer = core::slice::from_raw_parts_mut(virt_addr, packet_len);
            buffer[..packet.length].copy_from_slice(&packet.data[..packet.length]);
            if packet.length < MIN_PACKET_SIZE {
                // 패딩을 0으로 채움
                buffer[packet.length..].fill(0);
            }
            
            // 3. 송신 주소 레지스터에 물리 주소 설정
            let txaddr_reg = match desc_idx {
                0 => RTL8139_TXADDR0,
                1 => RTL8139_TXADDR1,
                2 => RTL8139_TXADDR2,
                3 => RTL8139_TXADDR3,
                _ => unreachable!(),
            };
            self.write_u32(txaddr_reg, phys_addr.as_u64() as u32);
            
            // 4. 송신 시작 (TXSTATUS 레지스터에 길이 설정)
            let txstatus_reg = match desc_idx {
                0 => RTL8139_TXSTATUS0,
                1 => RTL8139_TXSTATUS1,
                2 => RTL8139_TXSTATUS2,
                3 => RTL8139_TXSTATUS3,
                _ => unreachable!(),
            };
            
            // TXSTATUS에 패킷 길이 설정 (OWN 비트는 하드웨어가 설정)
            let status = (packet_len as u32) & TXSTATUS_DSIZE_MASK;
            self.write_u32(txstatus_reg, status);
            
            // 다음 송신 디스크립터로 이동
            self.tx_current = (desc_idx + 1) % TX_DESCRIPTOR_COUNT;
            
            crate::log_debug!("RTL8139: Sent packet (length: {}, desc: {})", packet_len, desc_idx);
            
            Ok(())
        }
    }
    
    fn receive_packet(&mut self) -> Option<PacketBuffer> {
        if !self.initialized {
            return None;
        }
        
        unsafe {
            // 인터럽트 상태 확인
            let isr = self.read_u16(RTL8139_ISR);
            
            if (isr & ISR_ROK) == 0 {
                // 수신된 패킷 없음
                return None;
            }
            
            // 수신 버퍼 확인
            let rx_buffer_virt = match self.rx_buffer_virt {
                Some(ptr) => ptr,
                None => {
                    crate::log_error!("RTL8139: RX buffer not allocated");
                    self.write_u16(RTL8139_ISR, ISR_ROK);
                    return None;
                }
            };
            
            // 현재 읽기 위치 확인
            let capr = self.read_u16(RTL8139_CAPR);
            let cbr = self.read_u16(RTL8139_CBR);
            
            // 수신 버퍼가 비어있는지 확인
            if capr == cbr {
                // 버퍼가 비어있음
                self.write_u16(RTL8139_ISR, ISR_ROK);
                return None;
            }
            
            // 패킷 헤더 읽기 (4바이트: 상태 u16, 길이 u16)
            let rx_offset = self.rx_current as usize;
            if rx_offset + 4 > RX_BUFFER_SIZE {
                // 버퍼 오버플로우 방지
                self.rx_current = 0;
                self.write_u16(RTL8139_ISR, ISR_ROK);
                return None;
            }
            
            let buffer = core::slice::from_raw_parts(rx_buffer_virt, RX_BUFFER_SIZE);
            let status = u16::from_le_bytes([buffer[rx_offset], buffer[rx_offset + 1]]);
            let packet_len = u16::from_le_bytes([buffer[rx_offset + 2], buffer[rx_offset + 3]]) as usize;
            
            // 패킷 길이 검증
            if packet_len == 0 || packet_len > MAX_PACKET_SIZE || packet_len < 4 {
                crate::log_warn!("RTL8139: Invalid packet length: {}", packet_len);
                // 잘못된 패킷 건너뛰기
                self.rx_current = (self.rx_current as usize + packet_len + 4) as u16;
                if self.rx_current as usize >= RX_BUFFER_SIZE {
                    self.rx_current = 0;
                }
                self.write_u16(RTL8139_RX_BUF_PTR, self.rx_current);
                self.write_u16(RTL8139_ISR, ISR_ROK);
                return None;
            }
            
            // 패킷 데이터 읽기 (헤더 제외)
            let data_start = rx_offset + 4;
            if data_start + packet_len > RX_BUFFER_SIZE {
                // 버퍼 오버플로우
                crate::log_warn!("RTL8139: Packet extends beyond buffer");
                self.rx_current = 0;
                self.write_u16(RTL8139_RX_BUF_PTR, 0);
                self.write_u16(RTL8139_ISR, ISR_ROK);
                return None;
            }
            
            // PacketBuffer 생성
            let mut packet = PacketBuffer::new();
            packet.length = packet_len;
            packet.data[..packet_len].copy_from_slice(&buffer[data_start..data_start + packet_len]);
            
            // 수신 버퍼 포인터 업데이트
            self.rx_current = (data_start + packet_len) as u16;
            // 4바이트 정렬
            self.rx_current = ((self.rx_current + 3) & !3) as u16;
            
            // 버퍼 랩 처리
            if self.rx_current as usize >= RX_BUFFER_SIZE {
                self.rx_current = 0;
            }
            
            // CAPR 업데이트 (하드웨어에 읽기 완료 알림)
            self.write_u16(RTL8139_CAPR, self.rx_current.wrapping_sub(0x10));
            
            // 인터럽트 상태 클리어
            self.write_u16(RTL8139_ISR, ISR_ROK);
            
            crate::log_debug!("RTL8139: Received packet (length: {})", packet_len);
            
            Some(packet)
        }
    }
    
    fn handle_interrupt(&mut self) {
        if !self.initialized {
            return;
        }
        
        unsafe {
            // 인터럽트 상태 읽기 및 배치 처리
            let mut isr = self.read_u16(RTL8139_ISR);

            if (isr & ISR_ROK) != 0 {
                // Drain as many packets as available
                let mut drained = 0;
                while let Some(_pkt) = self.receive_packet() {
                    drained += 1;
                    if drained > 32 { break; } // avoid livelock
                }
                crate::log_debug!("RTL8139: Receive OK, drained {} packets", drained);
                // 활동이 있으니 저전력 해제
                self.low_power = false;
            }

            if (isr & ISR_TOK) != 0 {
                // 송신 완료
                // crate::log_debug!("RTL8139: Transmit OK");
            }

            if (isr & ISR_RER) != 0 {
                crate::log_warn!("RTL8139: Receive Error interrupt");
            }

            if (isr & ISR_TER) != 0 {
                crate::log_warn!("RTL8139: Transmit Error interrupt");
            }

            // 인터럽트 상태 클리어 (일괄)
            self.write_u16(RTL8139_ISR, isr);
        }
    }
    
    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// RTL8139 드라이버가 지원하는 PCI 디바이스인지 확인
pub fn is_rtl8139(pci_device: &PciDevice) -> bool {
    pci_device.vendor_id == RTL8139_VENDOR_ID && 
    pci_device.device_id == RTL8139_DEVICE_ID
}

// 전역 저전력 관리: 유휴 타임아웃과 상태
struct NetPowerConfig { last_activity_ms: u64, idle_timeout_ms: u64 }
static NET_POWER: spin::Mutex<NetPowerConfig> = spin::Mutex::new(NetPowerConfig { last_activity_ms: 0, idle_timeout_ms: 0 });

/// 네트워크 전원관리: 유휴 타임아웃 설정 (0 = 비활성)
pub fn set_idle_timeout_ms(ms: u64) { let mut c = NET_POWER.lock(); c.idle_timeout_ms = ms; }

/// 네트워크 전원관리: 활동 기록
pub fn note_activity(now_ms: u64) { let mut c = NET_POWER.lock(); c.last_activity_ms = now_ms; }

/// 네트워크 전원관리: 유휴이면 RX 중지로 저전력 진입
pub fn maybe_enter_low_power(now_ms: u64, driver: &mut Rtl8139Driver) {
    let (last, timeout) = { let c = NET_POWER.lock(); (c.last_activity_ms, c.idle_timeout_ms) };
    if timeout == 0 { return; }
    if now_ms.saturating_sub(last) < timeout { return; }
    if !driver.low_power && driver.initialized {
        unsafe { driver.stop_receive(); }
        driver.low_power = true;
        crate::log_info!("RTL8139 entered low-power receive stop");
    }
}

