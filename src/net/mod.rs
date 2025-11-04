//! 네트워크 스택 모듈
//!
//! 이 모듈은 네트워크 프로토콜 스택 및 드라이버를 담당합니다.

pub mod ethernet;
pub mod driver;
pub mod ip;
pub mod arp;
pub mod icmp;
pub mod udp;
pub mod tcp;
pub mod ethernet_frame;

pub use ethernet::{MacAddress, PacketBuffer, NetworkError, EthernetDriver};
pub use driver::{init as init_network, send_packet, receive_packet, get_mac_address, low_power_tick};
pub use ip::{Ipv4Address, IpProtocol};
pub use arp::{resolve_ip, handle_arp_packet};
pub use icmp::{handle_icmp_packet, ping};
pub use udp::{handle_udp_packet, bind as udp_bind, send_udp_packet};
pub use tcp::{handle_tcp_packet, TcpPort, TcpFlags};

