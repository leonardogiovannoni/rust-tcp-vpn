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
    // if true ok
    let _h = match handshake::handler_client_handshake(&mut stream, &ifaddr, netmask) {
        Ok(false) => {
            eprintln!("Failed client handshake due to protocol error");
            return;
        }
        Ok(h) => h,
        Err(err) => {
            eprintln!("Failed client handshake: {}", err);
            return;
        }
    };
    // bring interface up
    tunif::set_interface_up(&iffile, &ifname);
    let mut sigfile = crate::signals::spawn_sig_handler();
    flows::handle_flow(&mut stream, &mut iffile, &mut sigfile);
    tunif::set_interface_down(&iffile, &ifname);
}
