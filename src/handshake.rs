use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpStream};

const MAGIC: u32 = 0x12345678;

// INITIAL HANDSHAKE:
//      1. client send packet containing (ifaddr,netmask)
//      2. server check received packet from client
//      3. server sends its ifaddr
//      4. client double check server if properties and send OK to server
//      5. client can now bring interface UP
//      6. server receive Ok from client
//      7. server can now bring interface UP
//      8. server and client can now exchange packets
pub fn handler_server_handshake(
    stream: &mut TcpStream,
    ifaddr: &IpAddr,
    netmask: u8
) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    let ifaddr: &Ipv4Addr = match ifaddr {
        IpAddr::V4(addr) => &addr,
        _ => {
            eprintln!("Cannot accept IPv6");
            std::process::exit(1)
        }
    };
    // https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.try_clone
    let ostream = stream.try_clone().unwrap();
    // https://doc.rust-lang.org/std/io/struct.BufWriter.html#method.with_capacity
    let mut ostream = BufWriter::with_capacity(64, ostream);
    // read stream
    let istream = stream.try_clone().unwrap();
    let mut istream = BufReader::with_capacity(64, istream);

    // classic netmask
    let netmask: u32 = (!0) ^ ((1 << (32 - netmask)) - 1);
    // local addr
    let local_addr: u32 = u32::from_be_bytes(ifaddr.octets());
    // 2. parse first packet
    {
        let mut packet1: [u8; 16] = [0; 16];
        // https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact
        istream.read_exact(&mut packet1)?;
        // check magic
        // https://doc.rust-lang.org/std/primitive.slice.html#method.split_at
        let found_magick = u32::from_be_bytes(packet1[..4].try_into().unwrap());
        if MAGIC != found_magick {
            eprintln!(
                "HANDSHAKE error, magic: {} instead of {}",
                found_magick, MAGIC
            );
            return Ok(false);
        }
        // check packet id: should be 1
        let pktid = u32::from_be_bytes(packet1[4..8].try_into().unwrap());
        if 1 != pktid {
            eprintln!("HANDSHAKE error, pktid: {} instead of {}", pktid, 1);
            return Ok(false);
        }
        // get remote address and netmask
        let remote_addr = u32::from_be_bytes(packet1[8..12].try_into().unwrap());
        let remote_netmask = u32::from_be_bytes(packet1[12..16].try_into().unwrap());
        // check netmask
        if netmask != remote_netmask {
            eprintln!(
                "HANDSHAKE error, netmask: {} instead of {}",
                remote_netmask, netmask
            );
            return Ok(false);
        }
        // check addresse: should not be equals but in the same subnet
        if !((local_addr & netmask == remote_addr & netmask) && (local_addr != remote_addr)) {
            eprintln!(
                "HANDSHAKE error, address: local {:#08x} remote {:#08x}",
                local_addr, remote_addr
            );
            return Ok(false);
        }
    }
    // 3. send server ifaddr
    {
        // packet id: 2
        ostream.write(&(2 as u32).to_be_bytes())?;
        // server interface address
        ostream.write(&local_addr.to_be_bytes())?;
        // send packet
        ostream.flush()?;
    }
    // 5 check client response
    {
        let mut packet3: [u8; 8] = [0; 8];
        // read packet
        istream.read_exact(&mut packet3)?;
        let pktid = u32::from_be_bytes(packet3[..4].try_into().unwrap());
        if 3 != pktid {
            eprintln!("HANDSHAKE error, pktid: {} instead of {}", pktid, 3);
            return Ok(false);
        }
        let status = u32::from_be_bytes(packet3[4..8].try_into().unwrap());
        if status != 0 {
            eprintln!(
                "HANDSHAKE error, client status: {} instead of {}",
                status, 0
            );
            return Ok(false);
        }
    }

    // SUCCESS
    Ok(true)
}

pub fn handler_client_handshake(
    stream: &mut TcpStream,
    ifaddr: &IpAddr,
    netmask: u8
) -> std::result::Result<bool, Box<dyn std::error::Error>> {
    let ifaddr: &Ipv4Addr = match ifaddr {
        IpAddr::V4(addr) => &addr,
        _ => {
            eprintln!("Cannot accept IPv6");
            std::process::exit(1)
        }
    };
    // https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.try_clone
    let ostream = stream.try_clone().unwrap();
    // https://doc.rust-lang.org/std/io/struct.BufWriter.html#method.with_capacity
    let mut ostream = BufWriter::with_capacity(64, ostream);
    // read stream
    let istream = stream.try_clone().unwrap();
    let mut istream = BufReader::with_capacity(64, istream);

    // classic netmask
    let netmask: u32 = (!0) ^ ((1 << (32 - netmask)) - 1);
    // local addr
    let local_addr: u32 = u32::from_be_bytes(ifaddr.octets());
    // 1. send intial packet: 16 bytes
    {
        // insert magic
        ostream.write(&MAGIC.to_be_bytes())?;
        // packet id: 1
        ostream.write(&(1 as u32).to_be_bytes())?;
        // IPv4 address - already in network byte order
        // https://doc.rust-lang.org/std/net/struct.Ipv4Addr.html#method.octets
        ostream.write(&local_addr.to_be_bytes())?;
        // netmask
        ostream.write(&netmask.to_be_bytes())?;
        // send packet
        ostream.flush()?;
    }
    // 3. check server response
    {
        let mut packet2: [u8; 8] = [0; 8];
        // read packet
        istream.read_exact(&mut packet2)?;
        // check idx
        let pktid = u32::from_be_bytes(packet2[..4].try_into().unwrap());
        if 2 != pktid {
            eprintln!("HANDSHAKE error, pktid: {} instead of {}", pktid, 2);
            return Ok(false);
        }
        // get remote iterface address
        let remote_addr = u32::from_be_bytes(packet2[4..8].try_into().unwrap());
        if !((local_addr & netmask == remote_addr & netmask) && (local_addr != remote_addr)) {
            eprintln!(
                "HANDSHAKE error, address: local {:#08x} remote {:#08x}",
                local_addr, remote_addr
            );
            return Ok(false);
        } else {
            // print server address
            println!(
                "Server interface address: {}",
                IpAddr::from(remote_addr.to_be_bytes())
            );
        }
    }
    // 4. send ok to server
    {
        // packet id: 3
        ostream.write(&(3 as u32).to_be_bytes())?;
        // all zeros is ok!
        ostream.write(&(0 as u32).to_be_bytes())?;
        // send packet
        ostream.flush()?;
    }

    // SUCCESS
    Ok(true)
}
