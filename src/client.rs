use crate::flows;
use crate::handshake;

use crate::tunif::Iface;
use anyhow::{bail, Result};
use std::net::{IpAddr, TcpStream};

pub fn execute_client(
    ifname: String,
    ifaddr: IpAddr,
    netmask: u8,
    remote: std::net::SocketAddr,
) -> Result<()> {
    let IpAddr::V4(tmp) = ifaddr else {
        bail!("Cannot accept IPv6");
    };
    let mut iface = Iface::new(&ifname, tmp, netmask)?;
    let mut stream = TcpStream::connect(remote)?;
    handshake::handler_client_handshake(&mut stream, &ifaddr, netmask)?;
    let mut sigfile = crate::signals::spawn_sig_handler()?;
    flows::handle_flow(&mut stream, &mut iface, &mut sigfile)?;
    Ok(())
}
