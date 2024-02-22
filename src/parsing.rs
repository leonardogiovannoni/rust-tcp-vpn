
use argparse::{ArgumentParser, Store, StoreTrue};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::process;


// Program can execute both as client or server
pub enum Args{
    // when connecting to remote need both ip and port
    Client {
        // properties of virtual interface
        ifname: String,
        ifaddr: IpAddr,
        netmask: u8,
        // TCP related data
        remote:std::net::SocketAddr
    },
    // when acting as server require address and port to
    // bind to for incoming connections
    Server {
        // properties of virtual interface
        ifname: String,
        ifaddr: IpAddr,
        netmask: u8,
        // TCP related data
        local:std::net::SocketAddr
    }
}

// https://docs.rs/crate/argparse/0.2.2
pub fn parse_arg() -> Args {
    let mut host:String = String::new();
    let mut ifname:String = String::new();
    let mut ifaddr:String = String::new();
    let mut netmask:u8 = 0;
    let mut port:u16 = 0;
    let mut server = false;
    {
        let mut parser = ArgumentParser::new();
        parser.set_description("TCP receiver: accept tcp connections and print received data");
        // Interface parameters
        parser.refer(&mut ifname)
            .add_option(&["--ifname"], Store, "Name of the local virtual interface")
            .required();
        parser.refer(&mut ifaddr)
            .add_option(&["--ifaddr"], Store, "IPv4 of the local virtual interface")
            .required();
        parser.refer(&mut netmask)
            .add_option(&["--netmask"], Store, "Netmask (0..32) of the local virtual interface")
            .required();
        // BOOLEAN: server or client?
        parser.refer(&mut server)
        .add_option(&["--server"], StoreTrue, "Should act as a server (default: client)");
        // SERVER: bind here    CLIENT: remote endpoint
        parser.refer(&mut host)
            .add_option(&["--host"], Store, "(Server) address to bind to (Client) remote server address")
            .required();
        parser.refer(&mut port)
            .add_option(&["--port"], Store, "(Server) TCP port to bind to (Client) remote server TCP port")
            .required();
        parser.parse_args_or_exit();
    }
    // https://doc.rust-lang.org/std/str/trait.FromStr.html#tymethod.from_str
    let host = match IpAddr::from_str(&host) {
        Ok(addr) => addr,
        Err(err) => {
            eprintln!("Error parsing address: {}", err);
            process::exit(1)
        }
    };
    let ifaddr = match IpAddr::from_str(&ifaddr) {
        Ok(addr) => addr,
        Err(err) => {
            eprintln!("Error parsing address: {}", err);
            process::exit(1)
        }
    };
    // IP address to be used in network connection
    let addr = SocketAddr::new(host, port);
    if server {
        Args::Server { ifname, ifaddr, netmask, local: addr }
    } else {
        Args::Client { ifname, ifaddr, netmask, remote: addr }
    }
}
