//! EHCI (Enhanced Host Controller Interface) 드라이버
//!
//! USB 2.0 호스트 컨트롤러를 지원합니다.
//!
//! # 참고 자료
//! - EHCI Specification Revision 1.0

use crate::drivers::pci::PciDevice;
use crate::drivers::usb::error::UsbError;
use crate::drivers::usb::host_controller::{UsbHostController, UsbHostControllerType};
use crate::drivers::usb::request::UsbControlRequest;
use core::ptr::{read_volatile, write_volatile};

/// MMIO 레지스터 오프셋
const EHCI_CAPLENGTH: usize = 0x00;
const EHCI_HCIVERSION: usize = 0x02;
const EHCI_HCSPARAMS: usize = 0x04;
const EHCI_HCCPARAMS: usize = 0x08;

/// Operational 레지스터 오프셋
const EHCI_USBCMD: usize = 0x00;
const EHCI_USBSTS: usize = 0x04;
const EHCI_USBINTR: usize = 0x08;
const EHCI_FRINDEX: usize = 0x0C;
const EHCI_CTRLDSSEGMENT: usize = 0x10;
const EHCI_PERIODICLISTBASE: usize = 0x14;
const EHCI_ASYNCLISTADDR: usize = 0x18;
const EHCI_CONFIGFLAG: usize = 0x40;
const EHCI_PORTSC: usize = 0x44; // Port Status and Control (포트별 0x04 오프셋)

/// 명령 레지스터 비트
const EHCI_CMD_RUN: u32 = 1 << 0;
const EHCI_CMD_RESET: u32 = 1 << 1;
const EHCI_CMD_HCRESET: u32 = 1 << 2;

/// 상태 레지스터 비트
const EHCI_STS_HALTED: u32 = 1 << 12;
const EHCI_STS_HSE: u32 = 1 << 2;

/// 포트 상태 비트
const EHCI_PORTSC_CCS: u32 = 1 << 0; // Current Connect Status
const EHCI_PORTSC_PED: u32 = 1 << 2; // Port Enabled/Disabled
const EHCI_PORTSC_PR: u32 = 1 << 8; // Port Reset
const EHCI_PORTSC_PP: u32 = 1 << 12; // Port Power

/// EHCI 호스트 컨트롤러
pub struct EhciController {
    /// PCI 디바이스
    pci_device: PciDevice,
    /// MMIO 베이스 주소
    mmio_base: u64,
    /// Capability 레지스터 베이스 (CAPLENGTH 값)
    cap_length: u8,
    /// Operational 레지스터 베이스
    op_base: u64,
    /// 초기화 여부
    initialized: bool,
    /// 포트 수
    port_count: u8,
}

impl EhciController {
    /// 새 EHCI 컨트롤러 생성
    pub fn new(pci_device: PciDevice) -> Self {
        Self {
            pci_device,
            mmio_base: 0,
            cap_length: 0,
            op_base: 0,
            initialized: false,
            port_count: 0,
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
        let offset = EHCI_PORTSC + ((port - 1) as usize * 0x04);
        self.read_op(offset)
    }
    
    /// 포트 상태 및 제어 레지스터 쓰기
    unsafe fn write_portsc(&self, port: u8, value: u32) {
        if port == 0 || port > self.port_count {
            return;
        }
        let offset = EHCI_PORTSC + ((port - 1) as usize * 0x04);
        self.write_op(offset, value);
    }
    
    /// 베이스 주소 설정
    pub unsafe fn set_base_address(&mut self, base: u64) {
        self.mmio_base = base;
        
        // CAPLENGTH 읽기 (하위 8비트)
        self.cap_length = (self.read_cap(EHCI_CAPLENGTH) & 0xFF) as u8;
        
        // Operational 레지스터 베이스 = MMIO 베이스 + CAPLENGTH
        self.op_base = self.mmio_base + self.cap_length as u64;
        
        // HCSPARAMS에서 포트 수 읽기
        let hcsparams = self.read_cap(EHCI_HCSPARAMS);
        self.port_count = ((hcsparams >> 0) & 0x0F) as u8;
        
        crate::log_info!(
            "EHCI: CAPLENGTH={}, OPS={:X}, Ports={}",
            self.cap_length,
            self.op_base,
            self.port_count
        );
    }
    
    /// 컨트롤러 초기화
    pub unsafe fn initialize(&mut self) -> Result<(), UsbError> {
        if self.mmio_base == 0 {
            return Err(UsbError::DeviceNotFound);
        }
        
        crate::log_info!("Initializing EHCI controller...");
        
        // 1. 컨트롤러가 Halted 상태인지 확인
        let usbsts = self.read_op(EHCI_USBSTS);
        if (usbsts & EHCI_STS_HALTED) == 0 {
            crate::log_warn!("EHCI controller is not halted, attempting to halt...");
            // Halt 컨트롤러
            let mut usbcmd = self.read_op(EHCI_USBCMD);
            usbcmd &= !EHCI_CMD_RUN;
            self.write_op(EHCI_USBCMD, usbcmd);
            
            // Halted 상태가 될 때까지 대기
            let mut timeout = 1000;
            while (self.read_op(EHCI_USBSTS) & EHCI_STS_HALTED) == 0 && timeout > 0 {
                timeout -= 1;
                for _ in 0..100 {
                    core::hint::spin_loop();
                }
            }
            
            if timeout == 0 {
                return Err(UsbError::DeviceError);
            }
        }
        
        // 2. 컨트롤러 리셋
        let mut usbcmd = self.read_op(EHCI_USBCMD);
        usbcmd |= EHCI_CMD_HCRESET;
        self.write_op(EHCI_USBCMD, usbcmd);
        
        // 리셋 완료 대기
        let mut timeout = 10000;
        while (self.read_op(EHCI_USBCMD) & EHCI_CMD_HCRESET) != 0 && timeout > 0 {
            timeout -= 1;
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }
        
        if timeout == 0 {
            return Err(UsbError::DeviceError);
        }
        
        // 3. 포트 전원 켜기 (CONFIGFLAG 설정)
        let configflag = self.read_op(EHCI_CONFIGFLAG);
        if (configflag & 1) == 0 {
            self.write_op(EHCI_CONFIGFLAG, 1);
        }
        
        // 4. 포트 상태 확인
        crate::log_info!("EHCI: Checking {} ports...", self.port_count);
        for port in 1..=self.port_count {
            let portsc = self.read_portsc(port);
            if (portsc & EHCI_PORTSC_CCS) != 0 {
                crate::log_info!("EHCI: Port {} has device connected", port);
            }
        }
        
        self.initialized = true;
        crate::log_info!("EHCI controller initialized successfully");
        
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
        Ok((portsc & EHCI_PORTSC_CCS) != 0)
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
        portsc |= EHCI_PORTSC_PR;
        self.write_portsc(port, portsc);
        
        // 리셋 완료 대기
        let mut timeout = 10000;
        while (self.read_portsc(port) & EHCI_PORTSC_PR) != 0 && timeout > 0 {
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
    
    /// 제어 요청 전송 (기본 구현)
    ///
    /// # Note
    /// 실제 EHCI 제어 요청은 Queue Head (QH)와 Transfer Descriptor (TD)를 통해 전송됩니다.
    pub unsafe fn send_control_request(
        &self,
        _device_address: u8,
        _request: &UsbControlRequest,
        _buffer: &mut [u8],
    ) -> Result<usize, UsbError> {
        if !self.initialized {
            return Err(UsbError::NotInitialized);
        }
        
        // TODO: Queue Head와 Transfer Descriptor를 통한 실제 제어 요청 전송 구현
        
        crate::log_warn!("EHCI control request not yet fully implemented");
        Err(UsbError::NotImplemented)
    }
    
    /// 포트 수 가져오기
    pub fn port_count(&self) -> u8 {
        self.port_count
    }
}

impl UsbHostController for EhciController {
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
                return Err(UsbError::DeviceError); // EHCI는 MMIO만 지원
            }
            
            // 초기화
            self.initialize()
        }
    }
    
    fn reset(&mut self) -> Result<(), UsbError> {
        unsafe {
            let mut usbcmd = self.read_op(EHCI_USBCMD);
            usbcmd |= EHCI_CMD_HCRESET;
            self.write_op(EHCI_USBCMD, usbcmd);
            Ok(())
        }
    }
    
    fn controller_type(&self) -> UsbHostControllerType {
        UsbHostControllerType::Ehci
    }
    
    fn is_running(&self) -> bool {
        self.initialized
    }
}

