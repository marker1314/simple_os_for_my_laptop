//! Minimal stateful firewall (ingress default drop with allow rules)

use crate::net::ethernet::{PacketBuffer, NetworkError};
use crate::net::ip::{Ipv4Address};
use spin::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proto { Any, Tcp(u16), Udp(u16), Icmp }

#[derive(Clone, Copy, Debug)]
pub struct Rule { pub src: Option<Ipv4Address>, pub dst_port: Proto, pub allow: bool }

static RULES: Mutex<[Option<Rule>; 16]> = Mutex::new([None;16]);

pub fn add_allow_any() { let mut r = RULES.lock(); r[0] = Some(Rule { src: None, dst_port: Proto::Any, allow: true }); }

pub fn should_allow(_packet: &PacketBuffer) -> bool {
    // TODO: parse IP/ports and evaluate; permit all if any allow-any rule exists
    let r = RULES.lock();
    r.iter().any(|e| matches!(e, Some(Rule { dst_port: Proto::Any, allow: true, ..})))
}

pub fn filter_ingress(packet: &PacketBuffer) -> Result<(), NetworkError> {
    if should_allow(packet) { Ok(()) } else { Err(NetworkError::InvalidPacket) }
}


