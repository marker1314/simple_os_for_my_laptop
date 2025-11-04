//! RTL8139 이더넷 컨트롤러 드라이버
//!
//! 이 모듈은 Realtek RTL8139 이더넷 네트워크 카드의 드라이버를 구현합니다.
//! RTL8139는 PCI 기반 이더넷 컨트롤러로, QEMU에서도 기본 지원됩니다.

use crate::drivers::pci::PciDevice;
use crate::net::ethernet::{EthernetDriver, NetworkError, MacAddress, PacketBuffer};
use x86_64::instructions::port::Port;
use spin::Mutex;

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

/// RTL8139 드라이버 구조체
pub struct Rtl8139Driver {
    /// IO 베이스 주소
    io_base: u16,
    /// MAC 주소
    mac_address: MacAddress,
    /// 수신 버퍼 (물리 주소)
    rx_buffer: Option<*mut u8>,
    /// 현재 수신 버퍼 포인터
    rx_current: u16,
    /// 초기화 여부
    initialized: bool,
    /// PCI 디바이스 정보
    pci_device: PciDevice,
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
            rx_buffer: None,
            rx_current: 0,
            initialized: false,
            pci_device,
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
        // 수신 버퍼 할당 (8KB, 8바이트 정렬 필요)
        // TODO: 실제로는 물리 메모리 할당자를 사용해야 하지만,
        // 초기 구현에서는 정적 버퍼를 사용하거나 힙 할당자를 사용합니다.
        // 여기서는 간단히 힙 할당을 사용합니다 (실제로는 DMA를 위해 물리 메모리 필요)
        
        // 임시로 null 포인터로 설정 (실제 구현에서는 물리 메모리 할당 필요)
        // 실제 구현에서는 memory::allocate_physical_frame() 같은 함수 사용
        self.rx_buffer = None;
        
        // 수신 버퍼 시작 주소 설정 (물리 주소)
        // 실제로는 물리 주소를 사용해야 하지만, 초기 구현에서는 0으로 설정
        self.write_u32(RTL8139_RX_BUF, 0);
        
        // 수신 버퍼 포인터 초기화
        self.rx_current = 0;
        self.write_u16(RTL8139_RX_BUF_PTR, 0);
        
        Ok(())
    }
    
    /// 수신 설정
    unsafe fn setup_receive(&self) {
        // Receive Configuration Register 설정
        let mut rcr = 0u32;
        rcr |= RCR_APM;  // Accept Physical Match (MAC 주소 일치)
        rcr |= RCR_AM;   // Accept Multicast
        rcr |= RCR_AB;   // Accept Broadcast
        rcr |= RCR_WRAP; // Wrap mode
        rcr |= (64 << RCR_MXDMA_SHIFT); // Max DMA Burst Size: 64 bytes
        
        self.write_u32(RTL8139_RCR, rcr);
    }
    
    /// 송신 설정
    unsafe fn setup_transmit(&self) {
        // Transmit Configuration Register 설정
        let mut tcr = 0u32;
        tcr |= (3 << TCR_IFG_SHIFT); // Inter-Frame Gap: 3
        tcr |= (64 << TCR_MXDMA_SHIFT); // Max DMA Burst Size: 64 bytes
        
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
            // TODO: 실제 송신 구현
            // 1. 송신 버퍼 할당 (물리 메모리)
            // 2. 패킷 데이터를 송신 버퍼에 복사
            // 3. 송신 주소 레지스터에 물리 주소 설정
            // 4. 송신 시작
            
            // 현재는 단순히 로그만 출력
            crate::log_debug!("RTL8139: Sending packet (length: {})", packet.length);
            
            // 임시로 성공 반환 (실제 구현 필요)
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
            
            // TODO: 실제 수신 구현
            // 1. 수신 버퍼에서 패킷 읽기
            // 2. 패킷 헤더 파싱 (길이, 상태 등)
            // 3. 패킷 데이터를 PacketBuffer로 복사
            // 4. 수신 버퍼 포인터 업데이트
            // 5. 인터럽트 상태 클리어
            
            // 인터럽트 상태 클리어
            self.write_u16(RTL8139_ISR, ISR_ROK);
            
            // 현재는 None 반환 (실제 구현 필요)
            None
        }
    }
    
    fn handle_interrupt(&mut self) {
        if !self.initialized {
            return;
        }
        
        unsafe {
            // 인터럽트 상태 읽기
            let isr = self.read_u16(RTL8139_ISR);
            
            if (isr & ISR_ROK) != 0 {
                // 수신 완료
                crate::log_debug!("RTL8139: Receive OK interrupt");
                // 수신 패킷 처리는 receive_packet()에서 수행
            }
            
            if (isr & ISR_TOK) != 0 {
                // 송신 완료
                crate::log_debug!("RTL8139: Transmit OK interrupt");
            }
            
            if (isr & ISR_RER) != 0 {
                // 수신 에러
                crate::log_warn!("RTL8139: Receive Error interrupt");
            }
            
            if (isr & ISR_TER) != 0 {
                // 송신 에러
                crate::log_warn!("RTL8139: Transmit Error interrupt");
            }
            
            // 인터럽트 상태 클리어
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

