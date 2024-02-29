use crate::flows;
use crate::handshake;
use crate::tunif;

use std::net::{IpAddr, TcpStream};
use std::process;

pub fn execute_client(
    ifname: String,
    ifaddr: IpAddr,
    netmask: u8,
    remote: std::net::SocketAddr,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
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
            return Err("Failed client handshake due to protocol error".into());
        }
        Ok(h) => h,
        Err(err) => {
            let msg = format!("Failed client handshake: {}", err);
            return Err(msg.into());
        }
    };
    // bring interface up
    tunif::set_interface_up(&iffile, &ifname);
    let mut sigfile = crate::signals::spawn_sig_handler();
    let ans = flows::handle_flow(&mut stream, &mut iffile, &mut sigfile);
    tunif::set_interface_down(&iffile, &ifname);
    match ans {
        Err(e) => Err(e),
        _ => Ok(()),
    }
}
