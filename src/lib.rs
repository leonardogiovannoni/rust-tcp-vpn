pub mod client;
pub mod flows;
pub mod handshake;
pub mod parsing;
pub mod server;
pub mod signals;
pub mod tunif;
use anyhow::Result;

pub fn run(args: parsing::Args) -> Result<()> {
    let ifname = args.interface.ifname;
    let ifaddr = args.interface.ifaddr;
    let netmask = args.interface.netmask;
    // different behaviour in case of client or server
    match args.mode {
        parsing::Mode::Client { remote } => client::execute_client(ifname, ifaddr, netmask, remote),
        parsing::Mode::Server { local } => server::execute_server(ifname, ifaddr, netmask, local),
    }
}
