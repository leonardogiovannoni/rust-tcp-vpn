// https://docs.rs/crate/argparse/0.2.2
// https://docs.rs/clap/latest/clap/
use clap::Parser;

use anyhow::Result;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

const DEFAULT_IFNAME: &str = "tun0";

// properties of virtual interface
pub struct Interface {
    pub ifname: String,
    pub ifaddr: IpAddr,
    pub netmask: u8,
}

pub enum Mode {
    // when connecting to remote need both ip and port
    Client {
        // TCP related data
        remote: std::net::SocketAddr,
    },
    // when acting as server require address and port to
    // bind to for incoming connections
    Server {
        // TCP related data
        local: std::net::SocketAddr,
    },
}

// Program can execute both as client or server
pub struct Args {
    pub interface: Interface,
    pub mode: Mode,
}

// clap seems better than argparse
/// Simple TCP based L3 (TUN) point-to-point VPN server or client
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Opts {
    // properties of local (server) or remote (client) endpoint
    /// (server) IP to accept connections on (client) remote server IP
    #[arg(long)]
    host: String,
    /// (server) TCP port to listen for connection (client) remote server port
    #[arg(short, long)]
    port: u16,

    // properties describing virtual interface
    /// virtual interface name
    #[arg(long, default_value_t = String::from(DEFAULT_IFNAME))]
    ifname: String,
    /// IPv4 address of virtual interface
    #[arg(long)]
    ifaddr: IpAddr,
    /// netmask (1,32) of virtual interface address
    #[arg(short, long)]
    netmask: u8,

    /// run as server (default: client)
    #[arg(short, long)]
    server: bool,
}

pub fn parse_arg() -> Result<Args> {
    let args = Opts::parse();

    let Opts {
        host,
        port,
        ifname,
        ifaddr,
        netmask,
        server,
    } = args;
    // https://doc.rust-lang.org/std/str/trait.FromStr.html#tymethod.from_str
    let host = IpAddr::from_str(&host)?;
    // IP address to be used in network connection
    let addr = SocketAddr::new(host, port);
    Ok(Args {
        interface: Interface {
            ifname,
            ifaddr,
            netmask,
        },
        mode: if server {
            Mode::Server { local: addr }
        } else {
            Mode::Client { remote: addr }
        },
    })
}
