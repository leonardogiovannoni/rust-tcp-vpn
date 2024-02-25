use crate::flows;
use crate::handshake;
use crate::tunif;

use std::net::{IpAddr, TcpStream};
use std::process;

pub fn execute_client(ifname: String, ifaddr: IpAddr, netmask: u8, remote: std::net::SocketAddr) {
    let mut iffile = tunif::initialize_tun_interface(&ifname, ifaddr, netmask);
    // try to connect to remote server
    let mut stream = match TcpStream::connect(remote) {
        Ok(stream) => {
            println!("Connection established!");
            stream
        }
        Err(err) => {
            eprintln!("Cannot connect to: {} cause {}", remote, err);
            process::exit(1)
        }
    };
    // start handshake as client
    let h = handshake::handler_client_handshake(&mut stream, &ifaddr, netmask);
    if !h {
        eprintln!("Failed client handshake");
        std::process::exit(1)
    }
    // bring interface up
    tunif::set_interface_up(&mut iffile, &ifname);
    let mut sigfile = crate::signals::spawn_sig_handler();
    flows::handle_flow(&mut stream, &mut iffile, &mut sigfile);
}
