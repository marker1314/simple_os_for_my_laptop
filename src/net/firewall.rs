//! Minimal stateful firewall (ingress default drop with allow rules)

use crate::net::ethernet::{PacketBuffer, NetworkError};
use crate::net::ip::{Ipv4Address};
use spin::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proto { Any, Tcp(u16), Udp(u16), Icmp }

#[derive(Clone, Copy, Debug)]
pub struct Rule { pub src: Option<Ipv4Address>, pub dst_port: Proto, pub allow: bool }

static RULES: Mutex<[Option<Rule>; 16]> = Mutex::new([None;16]);
// very small state table for TCP 5-tuples (ingress)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TcpState { src: Ipv4Address, dst: Ipv4Address, sport: u16, dport: u16 }
static STATE: Mutex<[Option<TcpState>; 64]> = Mutex::new([None;64]);

pub fn add_allow_any() { let mut r = RULES.lock(); r[0] = Some(Rule { src: None, dst_port: Proto::Any, allow: true }); }

pub fn should_allow(packet: &PacketBuffer) -> bool {
    // Default: deny unless allow-any rule exists or flow is established/allowed
    if RULES.lock().iter().any(|e| matches!(e, Some(Rule { dst_port: Proto::Any, allow: true, ..}))) {
        return true;
    }

    // Parse Ethernet EtherType (offset 12-13)
    if packet.length < 34 { return false; }
    let eth_type = u16::from_be_bytes([packet.data[12], packet.data[13]]);
    if eth_type != 0x0800 { return false; } // only IPv4 considered

    // IPv4 header minimal parse
    let iphdr = &packet.data[14..];
    let ihl = (iphdr[0] & 0x0F) as usize * 4;
    if ihl < 20 || 14 + ihl > packet.length { return false; }
    let proto = iphdr[9];
    let src = Ipv4Address([iphdr[12], iphdr[13], iphdr[14], iphdr[15]]);
    let dst = Ipv4Address([iphdr[16], iphdr[17], iphdr[18], iphdr[19]]);

    // TCP
    if proto == 6 {
        if 14 + ihl + 4 > packet.length { return false; }
        let l4 = &packet.data[14 + ihl..];
        let sport = u16::from_be_bytes([l4[0], l4[1]]);
        let dport = u16::from_be_bytes([l4[2], l4[3]]);
        let flags = l4[13];
        let syn = (flags & 0x02) != 0;
        let ack = (flags & 0x10) != 0;

        // Track established flows when ACK seen
        if ack {
            let mut st = STATE.lock();
            // insert if empty slot
            if !st.iter().any(|e| matches!(e, Some(t) if t.src==src && t.dst==dst && t.sport==sport && t.dport==dport)) {
                if let Some(slot) = st.iter_mut().find(|e| e.is_none()) {
                    *slot = Some(TcpState { src, dst, sport, dport });
                }
            }
            return true;
        }

        // Allow if rule exists for this destination port
        if RULES.lock().iter().any(|e| matches!(e, Some(Rule { dst_port: Proto::Tcp(p), allow: true, ..}) if *p == dport)) {
            return true;
        }

        // Allow initial SYN only if there is an explicit allow rule
        return syn && RULES.lock().iter().any(|e| matches!(e, Some(Rule { dst_port: Proto::Tcp(p), allow: true, ..}) if *p == dport));
    }

    // UDP allow by rule
    if proto == 17 {
        let l4 = &packet.data[14 + ihl..];
        if 14 + ihl + 4 > packet.length { return false; }
        let dport = u16::from_be_bytes([l4[2], l4[3]]);
        return RULES.lock().iter().any(|e| matches!(e, Some(Rule { dst_port: Proto::Udp(p), allow: true, ..}) if *p == dport));
    }

    false
}

pub fn filter_ingress(packet: &PacketBuffer) -> Result<(), NetworkError> {
    if should_allow(packet) { Ok(()) } else { Err(NetworkError::InvalidPacket) }
}


