//! Minimal DHCPv4 client (skeleton)

#[derive(Debug)]
pub enum DhcpError { NotImplemented }

#[derive(Debug, Clone, Copy)]
pub struct Ipv4Addr(pub u8, pub u8, pub u8, pub u8);

#[derive(Debug)]
pub struct DhcpLease {
    pub yiaddr: Ipv4Addr,
    pub router: Option<Ipv4Addr>,
    pub dns: Option<Ipv4Addr>,
}

/// Start DHCP discovery/request on the active interface (still simplified)
pub fn obtain_lease() -> Result<DhcpLease, DhcpError> {
    // TODO: Build and send real DHCPDISCOVER via UDP:68->67 and parse DHCPOFFER
    // For now, keep simplified path
    let lease = DhcpLease { yiaddr: Ipv4Addr(192,168,1,150), router: Some(Ipv4Addr(192,168,1,1)), dns: None };
    crate::log_info!("DHCP: obtained lease {}",
        alloc::format!("{}.{}.{}.{}", lease.yiaddr.0, lease.yiaddr.1, lease.yiaddr.2, lease.yiaddr.3));
    Ok(lease)
}

/// Apply lease to IP module
pub fn apply_lease(lease: &DhcpLease) {
    crate::net::ip::apply_assigned_ipv4(crate::net::ip::Ipv4Address([lease.yiaddr.0, lease.yiaddr.1, lease.yiaddr.2, lease.yiaddr.3]));
}


