

pub mod parsing;
pub mod handshake;
pub mod flows;
pub mod tunif;

use std::net::{IpAddr, TcpListener, TcpStream};
use std::process;

// How to use multiple module:
//  https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html


const IFNAME: &str = "tun0";


pub fn execute_server(ifname: String, ifaddr: IpAddr, netmask: u8, local: std::net::SocketAddr) {
    let mut iffile = tunif::initialize_tun_interface(&ifname, ifaddr, netmask);
    // wait for remote connection
    let listener = TcpListener::bind(local).unwrap();
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let h = handshake::handler_server_handshake(&mut stream, &ifaddr, netmask);
        if !h {
            eprintln!("Failed server handshake");
            std::process::exit(1)
        }
        // bring interface up
        tunif::set_interface_up(&iffile, &ifname);
        flows::handle_flow(&mut stream, &mut iffile);
        eprintln!("End of server work!");
        std::process::exit(1)
    }
}

pub fn execute_client(ifname: String, ifaddr: IpAddr, netmask: u8, remote: std::net::SocketAddr) {
    let mut iffile = tunif::initialize_tun_interface(&ifname, ifaddr, netmask);
    // try to connect to remote server
    let mut stream = match TcpStream::connect(remote) {
        Ok(stream) => {
            println!("Connection established!");
            stream
        },
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
    tunif::set_interface_up(&iffile, &ifname);
    flows::handle_flow(&mut stream, &mut iffile);
}

pub fn run(args: parsing::Args) {
    // different behaviour in case of client or server
    match args {
        parsing::Args::Client { ifname, ifaddr, netmask, remote } => execute_client(ifname, ifaddr, netmask, remote),
        parsing::Args::Server { ifname, ifaddr, netmask, local } => execute_server(ifname, ifaddr, netmask, local),
    }
}


