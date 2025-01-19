use anyhow::{Result, bail};
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
    netmask: u8,
) -> Result<()> {
    let ifaddr: &Ipv4Addr = match ifaddr {
        IpAddr::V4(addr) => addr,
        _ => {
            bail!("Cannot accept IPv6");
        }
    };
    // https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.try_clone
    let ostream = stream.try_clone()?;
    // https://doc.rust-lang.org/std/io/struct.BufWriter.html#method.with_capacity
    let mut ostream = BufWriter::with_capacity(64, ostream);
    // read stream
    let istream = stream.try_clone()?;
    let mut istream = BufReader::with_capacity(64, istream);

    // classic netmask
    let netmask = 0xFFFF_FFFFu32.wrapping_shl(32 - netmask as u32);
    // local addr
    let local_addr: u32 = u32::from_be_bytes(ifaddr.octets());
    // 2. parse first packet
    parse_first_packet(&mut istream, netmask, local_addr)?;
    // 3. send server ifaddr
    send_server_ifaddr(&mut ostream, local_addr)?;
    // 5 check client response
    check_client_response(&mut istream)?;

    Ok(())
}

fn check_client_response(istream: &mut BufReader<TcpStream>) -> Result<()> {
    let mut packet: [u8; 8] = [0; 8];
    istream.read_exact(&mut packet)?;
    let scan = packet.as_slice();
    let (&pktid, scan): (&[u8; 4], _) = scan.split_first_chunk().unwrap();
    let pktid = u32::from_be_bytes(pktid);
    if 3 != pktid {
        bail!("HANDSHAKE error, pktid: {} instead of {}", pktid, 3);
    }
    let (&status, _): (&[u8; 4], _) = scan.split_first_chunk().unwrap();
    let status = u32::from_be_bytes(status);
    if status != 0 {
        bail!(
            "HANDSHAKE error, client status: {} instead of {}",
            status,
            0
        );
    }
    Ok(())
}

fn send_server_ifaddr(ostream: &mut BufWriter<TcpStream>, local_addr: u32) -> Result<()> {
    ostream.write_all(&2_u32.to_be_bytes())?;
    ostream.write_all(&local_addr.to_be_bytes())?;
    ostream.flush()?;
    Ok(())
}

fn parse_first_packet(
    istream: &mut BufReader<TcpStream>,
    netmask: u32,
    local_addr: u32,
) -> Result<()> {
    let mut packet: [u8; 16] = [0; 16];
    istream.read_exact(&mut packet)?;
    let scan = packet.as_slice();
    let (&found_magic, scan): (&[u8; 4], _) = scan.split_first_chunk().unwrap();
    let found_magic = u32::from_be_bytes(found_magic);
    if MAGIC != found_magic {
        bail!(
            "HANDSHAKE error, magic: {} instead of {}",
            found_magic,
            MAGIC
        );
    }
    let (&pktid, scan): (&[u8; 4], _) = scan.split_first_chunk().unwrap();
    let pktid = u32::from_be_bytes(pktid);
    if 1 != pktid {
        bail!("HANDSHAKE error, pktid: {} instead of {}", pktid, 1);
    }
    let (&remote_addr, scan): (&[u8; 4], _) = scan.split_first_chunk().unwrap();
    let remote_addr = u32::from_be_bytes(remote_addr);
    let (&remote_netmask, _): (&[u8; 4], _) = scan.split_first_chunk().unwrap();
    let remote_netmask = u32::from_be_bytes(remote_netmask);
    if netmask != remote_netmask {
        bail!(
            "HANDSHAKE error, netmask: {} instead of {}",
            remote_netmask,
            netmask
        );
    }
    if !((local_addr & netmask == remote_addr & netmask) && (local_addr != remote_addr)) {
        bail!(
            "HANDSHAKE error, address: local {:#08x} remote {:#08x}",
            local_addr,
            remote_addr
        );
    }
    Ok(())
}

pub fn handler_client_handshake(
    stream: &mut TcpStream,
    ifaddr: &IpAddr,
    netmask: u8,
) -> Result<()> {
    let IpAddr::V4(ifaddr) = ifaddr else {
        bail!("Cannot accept IPv6");
    };
    // https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.try_clone
    let ostream = stream.try_clone()?;
    // https://doc.rust-lang.org/std/io/struct.BufWriter.html#method.with_capacity
    let mut ostream = BufWriter::with_capacity(64, ostream);
    // read stream
    let istream = stream.try_clone()?;
    let mut istream = BufReader::with_capacity(64, istream);

    // classic netmask
    let netmask = 0xFFFF_FFFFu32.wrapping_shl(32 - netmask as u32);
    // local addr
    let local_addr: u32 = u32::from_be_bytes(ifaddr.octets());
    // 1. send intial packet: 16 bytes
    send_initial_packet(&mut ostream, netmask, local_addr)?;
    // 3. check server response
    check_server_response(&mut istream, netmask, local_addr)?;
    // 4. send ok to server
    send_ok_to_server(&mut ostream)?;
    // SUCCESS
    Ok(())
}

fn send_ok_to_server(ostream: &mut BufWriter<TcpStream>) -> Result<(), anyhow::Error> {
    ostream.write_all(&3_u32.to_be_bytes())?;
    ostream.write_all(&0_u32.to_be_bytes())?;
    ostream.flush()?;
    Ok(())
}

fn check_server_response(
    istream: &mut BufReader<TcpStream>,
    netmask: u32,
    local_addr: u32,
) -> Result<(), anyhow::Error> {
    let mut packet: [u8; 8] = [0; 8];
    istream.read_exact(&mut packet)?;
    let scan = packet.as_slice();
    let (&pktid, scan): (&[u8; 4], _) = scan.split_first_chunk().unwrap();
    let pktid = u32::from_be_bytes(pktid);
    if pktid != 2 {
        bail!("HANDSHAKE error, pktid: {} instead of {}", pktid, 2);
    }
    let (&remote_addr, _): (&[u8; 4], _) = scan.split_first_chunk().unwrap();
    let remote_addr = u32::from_be_bytes(remote_addr);
    if !((local_addr & netmask == remote_addr & netmask) && (local_addr != remote_addr)) {
        bail!(
            "HANDSHAKE error, address: local {:#08x} remote {:#08x}",
            local_addr,
            remote_addr
        );
    } else {
        println!(
            "Server interface address: {}",
            IpAddr::from(remote_addr.to_be_bytes())
        );
    }
    Ok(())
}

fn send_initial_packet(
    ostream: &mut BufWriter<TcpStream>,
    netmask: u32,
    local_addr: u32,
) -> Result<(), anyhow::Error> {
    ostream.write_all(&MAGIC.to_be_bytes())?;
    ostream.write_all(&1_u32.to_be_bytes())?;
    ostream.write_all(&local_addr.to_be_bytes())?;
    ostream.write_all(&netmask.to_be_bytes())?;
    ostream.flush()?;
    Ok(())
}
