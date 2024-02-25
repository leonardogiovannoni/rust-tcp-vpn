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
    // allow crashing the process if no client is connected
    crate::signals::handle_interrupt(false);
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        // if true ok
        let _h = match handshake::handler_server_handshake(&mut stream, &ifaddr, netmask) {
            Ok(false) => {
                eprintln!("Failed server handshake due to protocol error");
                continue;
            }
            Ok(h) => h,
            Err(err) => {
                eprintln!("Failed server handshake: {}", err);
                continue;
            }
        };
        // bring interface up
        tunif::set_interface_up(&iffile, &ifname);
        crate::signals::handle_interrupt(true);
        if !flows::handle_flow(&mut stream, &mut iffile, &mut sigfile) {
            break;
        }
        crate::signals::handle_interrupt(false);
        tunif::set_interface_down(&iffile, &ifname);
    }
}
