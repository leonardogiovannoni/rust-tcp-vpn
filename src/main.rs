
// https://doc.rust-lang.org/cargo/reference/build-scripts.html
// https://doc.rust-lang.org/cargo/reference/build-script-examples.html#linking-to-system-libraries
// https://docs.rust-embedded.org/book/interoperability/c-with-rust.html


extern crate argparse;

use argparse::{ArgumentParser, Store, StoreTrue};
use byteorder::{NetworkEndian, ReadBytesExt};
use std::io::{prelude::*, BufRead, BufReader, BufWriter, Read, stdin, stdout, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::os::unix::fs::FileExt;
use std::process;
use std::str::FromStr;
use std::thread::{self, sleep};
use std::time::Duration;


const DEV_FILE: &str = "/dev/net/tun";
const IFNAME: &str = "tun0";
const MAGIC: u32 = 0x12345678;

mod tunif {
    pub mod wrapper {
        extern "C" {
            pub fn set_interface_name(
                if_fd: cty::c_int,
                ifname: *const cty::c_char
            ) -> ();
            pub fn set_interface_address(
                if_fd: cty::c_int,
                ifname: *const cty::c_char,
                addr: *const cty::c_char,
                netmask: cty::c_int
            ) -> ();
            pub fn set_interface_up(
                if_fd: cty::c_int,
                ifname: *const cty::c_char
            ) -> ();
            pub fn set_interface_down(
                if_fd: cty::c_int,
                ifname: *const cty::c_char
            ) -> ();
        }
    }

    use std::os::fd::AsRawFd;
    use std::net::IpAddr;

    pub fn set_interface_name(
        iffile: &std::fs::File,
        ifname: &str
    ) -> () {
        let if_fd = iffile.as_raw_fd();
        let if_fd = if_fd as cty::c_int;
        let ifname = match std::ffi::CString::new(ifname) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error creating cstring: {}", e);
                std::process::exit(1)
            }
        };
        unsafe {
            wrapper::set_interface_name(if_fd, ifname.as_ptr());
        }
    }

    pub fn set_interface_address(
        iffile: &std::fs::File,
        ifname: &str,
        addr: &IpAddr,
        netmask: i32
    ) -> () {
        let if_fd = iffile.as_raw_fd();
        let if_fd = if_fd as cty::c_int;
        let ifname = match std::ffi::CString::new(ifname) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error creating cstring: {}", e);
                std::process::exit(1)
            }
        };
        let addr = match addr {
            IpAddr::V4(_) => {
                addr.to_string()
            },
            IpAddr::V6(_) => {
                eprintln!("IPv6 is currently unsupported for virtual interface");
                std::process::exit(1)
            }
        };
        let addr = match std::ffi::CString::new(addr) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error creating cstring: {}", e);
                std::process::exit(1)
            }
        };
        let netmask = netmask as cty::c_int;
        unsafe {
            wrapper::set_interface_address(if_fd, ifname.as_ptr(), addr.as_ptr(), netmask);
        }
    }

    pub fn set_interface_up(
        iffile: &std::fs::File,
        ifname: &str
    ) -> () {
        let if_fd = iffile.as_raw_fd();
        let if_fd = if_fd as cty::c_int;
        let ifname = match std::ffi::CString::new(ifname) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error creating cstring: {}", e);
                std::process::exit(1)
            }
        };
        unsafe {
            wrapper::set_interface_up(if_fd, ifname.as_ptr());
        }
    }

    pub fn set_interface_down(
        iffile: &std::fs::File,
        ifname: &str
    ) -> () {
        let if_fd = iffile.as_raw_fd();
        let if_fd = if_fd as cty::c_int;
        let ifname = match std::ffi::CString::new(ifname) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error creating cstring: {}", e);
                std::process::exit(1)
            }
        };
        unsafe {
            wrapper::set_interface_down(if_fd, ifname.as_ptr());
        }
    }
}





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
fn parse_arg() -> Args {
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



// inizialize virtual interface but do not bring it up
fn initialize_tun_interface(ifname: &str, ifaddr: IpAddr, netmask: u8) -> std::fs::File {
    // open virtual device
    let mut iffile = match std::fs::File::options()
        .read(true)
        .write(true)
        .open(DEV_FILE) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Error opening file {}: {}", DEV_FILE, err);
            std::process::exit(1)
        }
    };
    // set interface name
    tunif::set_interface_name(&iffile, &ifname);
    // set interface ip
    tunif::set_interface_address(&iffile, &ifname, &ifaddr, netmask as i32);
    // return file handler
    iffile
}


// INITIAL HANDSHAKE:
//      1. client send packet containing (ifaddr,netmask)
//      2. server check received packet from client
//      3. server sends its ifaddr
//      4. client double check server if properties and send OK to server
//      5. client can now bring interface UP
//      6. server receive Ok from client
//      7. server can now bring interface UP
//      8. server and client can now exchange packets
fn handler_server_handshake(stream: &mut TcpStream, ifaddr: &IpAddr, netmask: u8) -> bool {
    let ifaddr: &Ipv4Addr = match ifaddr {
        IpAddr::V4(addr) => &addr,
        _ => {
            eprintln!("Cannot accept IPv6");
            std::process::exit(1)
        }
    };
    // https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.try_clone
    let mut ostream = stream.try_clone().unwrap();
    // https://doc.rust-lang.org/std/io/struct.BufWriter.html#method.with_capacity
    let mut ostream = BufWriter::with_capacity(64, ostream);
    // read stream
    let mut istream = stream.try_clone().unwrap();
    let mut istream = BufReader::with_capacity(64, istream);

    // classic netmask
    let netmask: u32 = (!0) ^ ((1<<(32-netmask))-1);
    // local addr
    let local_addr: u32 = u32::from_ne_bytes(ifaddr.octets());
    // 2. parse first packet
    {
        let mut packet1: [u8; 16] = [0; 16];
        // https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
        istream.read_exact(&mut packet1).unwrap();
        // check magic
        // https://doc.rust-lang.org/std/primitive.slice.html#method.split_at
        let found_magick = u32::from_ne_bytes(packet1[..4].try_into().unwrap());
        if MAGIC != found_magick {
            eprintln!("HANDSHAKE error, magic: {} instead of {}", found_magick, MAGIC);
            return false;
        }
        // check packet id: should be 1
        let pktid = u32::from_ne_bytes(packet1[4..8].try_into().unwrap());
        if 1 != pktid {
            eprintln!("HANDSHAKE error, pktid: {} instead of {}", pktid, 1);
            return false;
        }
        // get remote address and netmask
        let remote_addr = u32::from_ne_bytes(packet1[8..12].try_into().unwrap());
        let remote_netmask = u32::from_ne_bytes(packet1[12..16].try_into().unwrap());
        // check netmask
        if netmask != remote_netmask {
            eprintln!("HANDSHAKE error, netmask: {} instead of {}", remote_netmask, netmask);
            return false;
        }
        // check addresse: should not be equals but in the same subnet
        if (local_addr & netmask == remote_addr & netmask) && (local_addr != remote_addr) {
            eprintln!("HANDSHAKE error, address: local {} remote {}", local_addr, remote_addr);
            return false;
        }
    }
    // 3. send server ifaddr
    {
        // packet id: 2
        ostream.write(&(2 as u32).to_ne_bytes()).unwrap();
        // server interface address
        ostream.write(&local_addr.to_ne_bytes()).unwrap();
        // send packet
        ostream.flush().unwrap();
    }
    // 5 check client response
    {
        let mut packet3: [u8; 8] = [0; 8];
        // read packet
        istream.read_exact(&mut packet3).unwrap();
        let pktid = u32::from_ne_bytes(packet3[..4].try_into().unwrap());
        if 3 != pktid {
            eprintln!("HANDSHAKE error, pktid: {} instead of {}", pktid, 3);
            return false;
        }
        let status = u32::from_ne_bytes(packet3[4..8].try_into().unwrap());
        if status != 0 {
            eprintln!("HANDSHAKE error, client status: {} instead of {}", status, 0);
            return false;
        }
    }

    // SUCCESS
    true
}
fn handler_client_handshake(stream: &mut TcpStream, ifaddr: &IpAddr, netmask: u8) -> bool {
    let ifaddr: &Ipv4Addr = match ifaddr {
        IpAddr::V4(addr) => &addr,
        _ => {
            eprintln!("Cannot accept IPv6");
            std::process::exit(1)
        }
    };
    // https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.try_clone
    let mut ostream = stream.try_clone().unwrap();
    // https://doc.rust-lang.org/std/io/struct.BufWriter.html#method.with_capacity
    let mut ostream = BufWriter::with_capacity(64, ostream);
    // read stream
    let mut istream = stream.try_clone().unwrap();
    let mut istream = BufReader::with_capacity(64, istream);

    // classic netmask
    let netmask: u32 = (!0) ^ ((1<<(32-netmask))-1);
    // local addr
    let local_addr: u32 = u32::from_ne_bytes(ifaddr.octets());
    // 1. send intial packet: 16 bytes
    {
        // insert magic
        ostream.write(&MAGIC.to_ne_bytes()).unwrap();
        // packet id: 1
        ostream.write(&(1 as u32).to_ne_bytes()).unwrap();
        // IPv4 address - already in network byte order
        // https://doc.rust-lang.org/std/net/struct.Ipv4Addr.html#method.octets
        ostream.write(&local_addr.to_ne_bytes()).unwrap();
        // netmask
        ostream.write(&netmask.to_ne_bytes()).unwrap();
        // send packet
        ostream.flush().unwrap();
    }
    // 3. check server response
    {
        let mut packet2: [u8; 8] = [0; 8];
        // read packet
        istream.read_exact(&mut packet2).unwrap();
        // check idx
        let pktid = u32::from_ne_bytes(packet2[..4].try_into().unwrap());
        if 2 != pktid {
            eprintln!("HANDSHAKE error, pktid: {} instead of {}", pktid, 2);
            return false;
        }
        // get remote iterface address
        let remote_addr = u32::from_ne_bytes(packet2[4..8].try_into().unwrap());
        if (local_addr & netmask == remote_addr & netmask) && (local_addr != remote_addr) {
            eprintln!("HANDSHAKE error, address: local {} remote {}", local_addr, remote_addr);
            return false;
        } else {
            // print server address
            println!("Server interface address: {}", IpAddr::from(remote_addr.to_ne_bytes()));
        }
    }
    // 4. send ok to server
    {
        // packet id: 3
        ostream.write(&(3 as u32).to_ne_bytes()).unwrap();
        // all zeros is ok!
        ostream.write(&(0 as u32).to_ne_bytes()).unwrap();
        // send packet
        ostream.flush().unwrap();
    }

    // SUCCESS
    true
}

fn handle_local2remote(iffile: std::fs::File, stream: TcpStream) -> thread::JoinHandle<()> {
    thread::spawn(
        move || {
            let mut buffer: [u8; 4096] = [0; 4096];
            let mut stream = BufWriter::with_capacity(64+4096, stream);
            // count how many packets are sent
            let mut counter: u64 = 0;
            let mut iffile = iffile;
            loop {
                // packet is always fully read (if possible):
                // this is a special case tied to virtual interface
                // internals
                let sz = match iffile.read(&mut buffer) {
                    Ok(0) => {
                        panic!("UNEXPECTED EMPTY PACKET!");
                    },
                    Ok(sz) => {
                        // new packet
                        counter += 1;
                        sz
                    },
                    Err(err) => {
                        eprintln!("Error creating cstring: {}", err);
                        std::process::exit(1)
                    }
                };
                // build packet
                // data packet: type 1
                stream.write(&(1 as u32).to_ne_bytes()).unwrap();
                // pkt length
                stream.write(&(sz as u32).to_ne_bytes()).unwrap();
                // counter
                stream.write(&counter.to_ne_bytes()).unwrap();
                // network packet
                stream.write(&buffer[..sz]).unwrap();
                // send packet
                stream.flush().unwrap();
            }
        }
    )
}
fn handle_remote2local(stream: TcpStream, iffile: std::fs::File) -> thread::JoinHandle<()> {
    thread::spawn(
        move || {
            let mut iffile = iffile;
            let mut buffer: [u8; 4096] = [0; 4096];
            let mut stream = BufReader::with_capacity(64+4096, stream);

            loop {
                // read packet type
                let mut pkt_type: [u8; 4] = [0; 4];
                stream.read_exact(&mut pkt_type).unwrap();
                let pkt_type = u32::from_ne_bytes(pkt_type);
                match pkt_type {
                    1 => {
                        let mut pkt_len: [u8; 4] = [0; 4];
                        stream.read_exact(&mut pkt_len).unwrap();
                        let pkt_len: u32 = u32::from_ne_bytes(pkt_len);
                        //println!("pkt_len = {}", pkt_len);
                        let mut counter: [u8; 8] = [0; 8];
                        stream.read_exact(&mut counter).unwrap();
                        let _counter = u64::from_ne_bytes(counter);
                        // counter is unused now
                        stream.read_exact(&mut buffer[0..(pkt_len as usize)]).unwrap();
                        // https://doc.rust-lang.org/std/fs/struct.File.html#method.write_all_at-1
                        match iffile.write_all(&buffer[0..(pkt_len as usize)]) {
                            Ok(()) => (),
                            Err(_) => eprintln!("Error writing pkt to virtual interface")
                        };
                        // it does not seem possible to flush virtual interface fd
                        //iffile.flush().unwrap();
                    },
                    _ => {
                        panic!("Unknown packet type: {} (only 1 valid)", pkt_type);
                    }
                }
            }
        })
}

// handle packet flow
// use 2 threads: [local->remote] and [remote->local]
fn handle_flow(stream: &mut TcpStream, iffile: &mut std::fs::File) -> () {
    let t1_hanle = handle_local2remote(iffile.try_clone().unwrap(), stream.try_clone().unwrap());
    let t2_hanle = handle_remote2local(stream.try_clone().unwrap(), iffile.try_clone().unwrap());
    t1_hanle.join().unwrap();
    t2_hanle.join().unwrap();
}


fn execute_server(ifname: String, ifaddr: IpAddr, netmask: u8, local: std::net::SocketAddr) {
    let mut iffile = initialize_tun_interface(&ifname, ifaddr, netmask);
    // wait for remote connection
    let listener = TcpListener::bind(local).unwrap();
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let h = handler_server_handshake(&mut stream, &ifaddr, netmask);
        if !h {
            eprintln!("Failed server handshake");
            std::process::exit(1)
        }
        // bring interface up
        tunif::set_interface_up(&iffile, &ifname);
        handle_flow(&mut stream, &mut iffile);
        eprintln!("End of server work!");
        std::process::exit(1)
    }
}

fn execute_client(ifname: String, ifaddr: IpAddr, netmask: u8, remote: std::net::SocketAddr) {
    let mut iffile = initialize_tun_interface(&ifname, ifaddr, netmask);
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
    let h = handler_client_handshake(&mut stream, &ifaddr, netmask);
    if !h {
        eprintln!("Failed client handshake");
        std::process::exit(1)
    }
    // bring interface up
    tunif::set_interface_up(&iffile, &ifname);
    handle_flow(&mut stream, &mut iffile);
}

fn run(args: Args) {
    // different behaviour in case of client or server
    match args {
        Args::Client { ifname, ifaddr, netmask, remote } => execute_client(ifname, ifaddr, netmask, remote),
        Args::Server { ifname, ifaddr, netmask, local } => execute_server(ifname, ifaddr, netmask, local),
    }
}

fn main() -> std::io::Result<()> {
    let args = parse_arg();
    run(args);

    Ok(())
}
