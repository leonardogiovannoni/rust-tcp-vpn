use crate::flows;
use crate::handshake;
use crate::tunif::Iface;
use anyhow::Result;
use anyhow::bail;
use std::net::{IpAddr, TcpListener};

pub fn execute_server(
    ifname: String,
    ifaddr: IpAddr,
    netmask: u8,
    local: std::net::SocketAddr,
) -> Result<()> {
    let IpAddr::V4(tmp) = ifaddr else {
        bail!("Cannot accept IPv6");
    };
    let mut iffile = Iface::new(&ifname, tmp, netmask)?;
    // wait for remote connection
    let listener = TcpListener::bind(local)?;
    // spawn thread handler
    let mut sigfile = crate::signals::spawn_sig_handler()?;
    // allow crashing the process if no client is connected
    crate::signals::handle_interrupt(false);
    for stream in listener.incoming() {
        let mut stream = stream?;
        // if true ok
        handshake::handler_server_handshake(&mut stream, &ifaddr, netmask)?;
        crate::signals::handle_interrupt(true);
        let ans = flows::handle_flow(&mut stream, &mut iffile, &mut sigfile);
        crate::signals::handle_interrupt(false);
        if !ans? {
            break;
        }
    }
    Ok(())
}
