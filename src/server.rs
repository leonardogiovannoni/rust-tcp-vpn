use crate::flows;
use crate::handshake;
use crate::tunif;

use std::net::{IpAddr, TcpListener};

pub fn execute_server(ifname: String, ifaddr: IpAddr, netmask: u8, local: std::net::SocketAddr) {
    let mut iffile = tunif::initialize_tun_interface(&ifname, ifaddr, netmask);
    // wait for remote connection
    let listener = TcpListener::bind(local).unwrap();
    // spawn thread handler
    let mut sigfile = crate::signals::spawn_sig_handler();
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let h = handshake::handler_server_handshake(&mut stream, &ifaddr, netmask);
        if !h {
            eprintln!("Failed server handshake");
            std::process::exit(1)
        }
        // bring interface up
        tunif::set_interface_up(&iffile, &ifname);
        if !flows::handle_flow(&mut stream, &mut iffile, &mut sigfile) {
            break;
        }
    }
}
