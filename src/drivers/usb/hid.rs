//! USB HID (Human Interface Device) support skeleton
//!
//! Goal: Provide minimal keyboard/mouse input over Interrupt IN endpoints.

use crate::drivers::usb::descriptor::{InterfaceDescriptor, EndpointDescriptor};
use crate::drivers::usb::request::UsbControlRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HidDeviceKind {
    Keyboard,
    Mouse,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct HidEndpoint {
    pub address: u8,      // endpoint address (IN)
    pub interval_ms: u8,  // polling interval
    pub max_packet_size: u16,
}

#[derive(Debug)]
pub struct HidDevice {
    pub kind: HidDeviceKind,
    pub interrupt_in: Option<HidEndpoint>,
    pub interface_number: u8,
}

impl HidDevice {
    pub fn from_interface(intf: &InterfaceDescriptor, endpoints: &[EndpointDescriptor]) -> Option<Self> {
        if intf.class_code != 0x03 { // HID class
            return None;
        }

        let kind = match intf.protocol_code {
            0x01 => HidDeviceKind::Keyboard, // Boot Keyboard
            0x02 => HidDeviceKind::Mouse,    // Boot Mouse
            _ => HidDeviceKind::Unknown,
        };

        let mut ep_in: Option<HidEndpoint> = None;
        for ep in endpoints.iter() {
            if ep.endpoint_address & 0x80 != 0 && (ep.attributes & 0x03) == 0x03 {
                ep_in = Some(HidEndpoint {
                    address: ep.endpoint_address,
                    interval_ms: ep.interval,
                    max_packet_size: ep.max_packet_size,
                });
                break;
            }
        }

        Some(HidDevice {
            kind,
            interrupt_in: ep_in,
            interface_number: intf.interface_number,
        })
    }
}

/// Minimal HID report parser hooks (to be implemented)
pub mod report {
    #[derive(Debug, Clone, Copy)]
    pub struct KeyboardReport {
        pub modifiers: u8,
        pub keys: [u8; 6],
    }

    #[derive(Debug, Clone, Copy)]
    pub struct MouseReport {
        pub buttons: u8,
        pub dx: i8,
        pub dy: i8,
        pub wheel: i8,
    }
}

/// Build a HID Class GET_REPORT control request (bmRequestType=0xA1, bRequest=0x01)
pub fn build_get_report_request(interface_number: u8, report_type: u8, report_id: u8, length: u16) -> UsbControlRequest {
    // wValue: high byte = report type, low byte = report id
    let value = ((report_type as u16) << 8) | (report_id as u16);
    UsbControlRequest {
        request_type: 0xA1, // Device-to-Host, Class, Interface
        request: 0x01,      // GET_REPORT
        value,
        index: interface_number as u16,
        length,
    }
}


