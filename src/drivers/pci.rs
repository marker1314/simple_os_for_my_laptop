//! PCI (Peripheral Component Interconnect) 버스 관리
//!
//! 이 모듈은 PCI 버스를 스캔하여 디바이스를 발견하고 관리합니다.

use x86_64::instructions::port::Port;

/// PCI 구성 공간 포트
const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

/// PCI 구성 공간 레지스터 오프셋
const PCI_VENDOR_ID: u8 = 0x00;
const PCI_DEVICE_ID: u8 = 0x02;
const PCI_COMMAND: u8 = 0x04;
const PCI_STATUS: u8 = 0x06;
const PCI_CLASS_CODE: u8 = 0x0B;
const PCI_SUBCLASS: u8 = 0x0A;
const PCI_PROG_IF: u8 = 0x09;
const PCI_HEADER_TYPE: u8 = 0x0E;
const PCI_BAR0: u8 = 0x10;

/// PCI 헤더 타입
const PCI_HEADER_TYPE_DEVICE: u8 = 0x00;
const PCI_HEADER_TYPE_BRIDGE: u8 = 0x01;

/// PCI 클래스 코드
pub const PCI_CLASS_NETWORK: u8 = 0x02;
pub const PCI_CLASS_DISPLAY: u8 = 0x03;
pub const PCI_CLASS_STORAGE: u8 = 0x01;

/// PCI 네트워크 서브클래스
pub const PCI_SUBCLASS_ETHERNET: u8 = 0x00;

/// PCI 디바이스 정보
#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    /// 버스 번호
    pub bus: u8,
    /// 디바이스 번호
    pub device: u8,
    /// 함수 번호
    pub function: u8,
    /// 벤더 ID
    pub vendor_id: u16,
    /// 디바이스 ID
    pub device_id: u16,
    /// 클래스 코드
    pub class_code: u8,
    /// 서브클래스
    pub subclass: u8,
    /// 프로그래밍 인터페이스
    pub prog_if: u8,
    /// 헤더 타입
    pub header_type: u8,
    /// BAR0 (베이스 주소 레지스터 0)
    pub bar0: u32,
}

impl PciDevice {
    /// PCI 구성 공간에서 32비트 레지스터 읽기
    ///
    /// # Safety
    /// 유효한 PCI 디바이스에 대한 접근이어야 합니다.
    pub unsafe fn read_config_register(&self, offset: u8) -> u32 {
        let address = self.make_config_address(offset);
        
        // 주소 포트에 주소 쓰기
        let mut address_port: Port<u32> = Port::new(PCI_CONFIG_ADDRESS);
        address_port.write(address);
        
        // 데이터 포트에서 데이터 읽기
        let mut data_port: Port<u32> = Port::new(PCI_CONFIG_DATA);
        data_port.read()
    }
    
    /// PCI 구성 공간에 32비트 레지스터 쓰기
    ///
    /// # Safety
    /// 유효한 PCI 디바이스에 대한 접근이어야 합니다.
    pub unsafe fn write_config_register(&self, offset: u8, value: u32) {
        let address = self.make_config_address(offset);
        
        // 주소 포트에 주소 쓰기
        let mut address_port: Port<u32> = Port::new(PCI_CONFIG_ADDRESS);
        address_port.write(address);
        
        // 데이터 포트에 데이터 쓰기
        let mut data_port: Port<u32> = Port::new(PCI_CONFIG_DATA);
        data_port.write(value);
    }
    
    /// PCI 구성 공간 주소 생성
    fn make_config_address(&self, offset: u8) -> u32 {
        let enable_bit = 1 << 31;
        let bus_bits = (self.bus as u32) << 16;
        let device_bits = (self.device as u32) << 11;
        let function_bits = (self.function as u32) << 8;
        let offset_bits = (offset as u32) & 0xFC; // 하위 2비트는 0 (32비트 정렬)
        
        enable_bit | bus_bits | device_bits | function_bits | offset_bits
    }
    
    /// 디바이스가 존재하는지 확인
    ///
    /// # Safety
    /// 유효한 PCI 버스/디바이스/함수 번호에 대한 접근이어야 합니다.
    pub unsafe fn exists(&self) -> bool {
        let vendor_id = self.read_config_register(PCI_VENDOR_ID) as u16;
        // 0xFFFF는 존재하지 않는 디바이스를 의미
        vendor_id != 0xFFFF
    }
    
    /// 디바이스 정보 읽기
    ///
    /// # Safety
    /// 유효한 PCI 디바이스에 대한 접근이어야 합니다.
    pub unsafe fn read_info(&mut self) {
        let vendor_device = self.read_config_register(PCI_VENDOR_ID);
        self.vendor_id = vendor_device as u16;
        self.device_id = (vendor_device >> 16) as u16;
        
        let class_revision = self.read_config_register(PCI_CLASS_CODE);
        self.class_code = ((class_revision >> 24) & 0xFF) as u8;
        self.subclass = ((class_revision >> 16) & 0xFF) as u8;
        self.prog_if = ((class_revision >> 8) & 0xFF) as u8;
        
        let header_type_status = self.read_config_register(PCI_HEADER_TYPE);
        self.header_type = ((header_type_status >> 16) & 0xFF) as u8;
        
        // BAR0 읽기
        self.bar0 = self.read_config_register(PCI_BAR0);
    }
}

/// PCI 버스 스캔 콜백 타입
pub type PciScanCallback = fn(&PciDevice) -> bool;

/// PCI 버스 스캔
///
/// 모든 PCI 버스를 스캔하여 디바이스를 찾고, 콜백 함수를 호출합니다.
/// 콜백이 true를 반환하면 스캔을 중단합니다.
///
/// # Safety
/// 메모리 관리가 초기화된 후에 호출되어야 합니다.
pub unsafe fn scan_pci_bus(callback: PciScanCallback) {
    // 각 버스 스캔 (일반적으로 0-255 버스, 하지만 대부분 0-1만 사용)
    for bus in 0..=255 {
        // 각 디바이스 스캔 (0-31 디바이스)
        for device in 0..=31 {
            // 함수 0만 확인 (일반적으로 함수 0만 존재)
            let mut pci_device = PciDevice {
                bus,
                device,
                function: 0,
                vendor_id: 0,
                device_id: 0,
                class_code: 0,
                subclass: 0,
                prog_if: 0,
                header_type: 0,
                bar0: 0,
            };
            
            if !pci_device.exists() {
                continue;
            }
            
            // 디바이스 정보 읽기
            pci_device.read_info();
            
            // 헤더 타입 확인
            let header_type = pci_device.header_type & 0x7F; // 다중 함수 비트 제거
            
            // 다중 함수 디바이스인 경우 모든 함수 스캔
            if header_type == PCI_HEADER_TYPE_DEVICE {
                if (pci_device.header_type & 0x80) != 0 {
                    // 다중 함수 디바이스
                    for function in 0..8 {
                        let mut func_device = PciDevice {
                            bus,
                            device,
                            function,
                            vendor_id: 0,
                            device_id: 0,
                            class_code: 0,
                            subclass: 0,
                            prog_if: 0,
                            header_type: 0,
                            bar0: 0,
                        };
                        
                        if func_device.exists() {
                            func_device.read_info();
                            if callback(&func_device) {
                                return; // 스캔 중단
                            }
                        }
                    }
                } else {
                    // 단일 함수 디바이스
                    if callback(&pci_device) {
                        return; // 스캔 중단
                    }
                }
            }
        }
    }
}

/// 특정 클래스의 PCI 디바이스 찾기
///
/// # Safety
/// 메모리 관리가 초기화된 후에 호출되어야 합니다.
pub unsafe fn find_pci_device(class_code: u8, subclass: u8) -> Option<PciDevice> {
    let mut found_device: Option<PciDevice> = None;
    
    scan_pci_bus(|device| {
        if device.class_code == class_code && device.subclass == subclass {
            found_device = Some(*device);
            true // 스캔 중단
        } else {
            false // 계속 스캔
        }
    });
    
    found_device
}

/// 네트워크 디바이스 찾기
///
/// 이더넷 컨트롤러를 찾습니다.
///
/// # Safety
/// 메모리 관리가 초기화된 후에 호출되어야 합니다.
pub unsafe fn find_network_device() -> Option<PciDevice> {
    find_pci_device(PCI_CLASS_NETWORK, PCI_SUBCLASS_ETHERNET)
}

