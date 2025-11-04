//! 네트워크 스택 모듈
//!
//! 이 모듈은 네트워크 프로토콜 스택 및 드라이버를 담당합니다.

pub mod ethernet;
pub mod driver;

pub use ethernet::{MacAddress, PacketBuffer, NetworkError, EthernetDriver};
pub use driver::{init as init_network, send_packet, receive_packet, get_mac_address};

