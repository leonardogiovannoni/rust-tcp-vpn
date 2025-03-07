use crate::tunif::Iface;
use anyhow::{Result, anyhow, bail};
use nix::poll::PollFd;
use nix::poll::PollFlags;
use nix::poll::PollTimeout;
use nix::poll::poll;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::os::fd::AsFd;

enum Status {
    // continue
    Continue,
    // regular exit
    ExitOk,
}

fn send_exit_pkt(stream: &mut impl std::io::Write, exit_reason: u32) -> Result<()> {
    // build packet
    // exit packet: type 2
    stream.write_all(&2_u32.to_be_bytes())?;
    // exit reason, only 0 in currently valid
    stream.write_all(&exit_reason.to_be_bytes())?;
    // send packet
    stream.flush()?;
    Ok(())
}

fn handle_local2remote_pkt(
    iffile: &mut Iface,
    stream: &mut impl std::io::Write,
    counter: &mut u64,
    buffer: &mut [u8],
) -> Result<Status> {
    let sz = iffile.as_ref().read(buffer)?;
    if sz == 0 {
        bail!("UNEXPECTED EMPTY PACKET from Virtual interface!");
    }
    *counter += 1;
    // build packet
    // data packet: type 1
    stream.write_all(&1_u32.to_be_bytes())?;
    // pkt length
    stream.write_all(&(sz as u32).to_be_bytes())?;
    // counter
    stream.write_all(&counter.to_be_bytes())?;
    // network packet
    stream.write_all(&buffer[..sz])?;
    // send packet
    stream.flush()?;
    // Everything Ok, continue
    Ok(Status::Continue)
}

fn handle_remote2local_pkt(
    iffile: &mut Iface,
    stream: &mut impl std::io::BufRead,
    buffer: &mut [u8],
) -> Result<Status> {
    // read packet type
    let mut pkt_type: [u8; 4] = [0; 4];
    stream.read_exact(&mut pkt_type)?;
    let pkt_type = u32::from_be_bytes(pkt_type);
    match pkt_type {
        1 => {
            let mut pkt_len: [u8; 4] = [0; 4];
            stream.read_exact(&mut pkt_len)?;
            let pkt_len: u32 = u32::from_be_bytes(pkt_len);
            let mut counter: [u8; 8] = [0; 8];
            stream.read_exact(&mut counter)?;
            let _counter = u64::from_be_bytes(counter);
            // counter is unused now
            stream.read_exact(&mut buffer[0..(pkt_len as usize)])?;
            // https://doc.rust-lang.org/std/fs/struct.File.html#method.write_all_at-1
            iffile.as_ref().write_all(&buffer[0..(pkt_len as usize)])?;
            Ok(Status::Continue)
        }
        2 => {
            let mut exit_reason: [u8; 4] = [0; 4];
            stream.read_exact(&mut exit_reason)?;
            let exit_reason: u32 = u32::from_be_bytes(exit_reason);
            if exit_reason != 0 {
                bail!("Unknown exit reason code {} in VPN protocol", exit_reason);
            } else {
                // terminate VPN protocol
                Ok(Status::ExitOk)
            }
        }
        _ => {
            bail!("Unknown packet type: {} (only 1 valid)", pkt_type);
        }
    }
}

// https://docs.rs/nix/0.28.0/nix/poll/struct.PollFd.html
// sigfile has been generated by crate::signals::spawn_sig_handler
// and is filled with new data everytime a signal is received
//
// Return true if exits because received exit packet from remote
// endpoint (or in case of remote stream error), return false if
// it exits because of local signal
//
// Return Err in case of other errors
pub fn handle_flow(
    stream: &mut TcpStream,
    iffile: &mut Iface,
    sigfile: &mut std::fs::File,
) -> Result<bool> {
    let mut buffer = [0; 4096];
    // split both socket ends
    let mut ostream = BufWriter::with_capacity(64 + 4096, stream.try_clone()?);
    let mut istream = BufReader::with_capacity(64 + 4096, stream.try_clone()?);
    // count how many packets are sent?
    let mut counter = 0;

    loop {
        let mut fds = [sigfile.as_fd(), stream.as_fd(), iffile.as_fd()]
            .map(|fd| PollFd::new(fd, PollFlags::POLLIN));
        // https://docs.rs/nix/0.28.0/nix/poll/fn.poll.html
        let ret = poll(&mut fds, PollTimeout::NONE)?;
        if ret <= 0 {
            bail!("Non positive nix::poll::poll");
        }
        let [pipe_fd, tcp_fd, if_fd] = fds;
        let b = pipe_fd
            .any()
            .ok_or(anyhow!("ERROR: pipe_fd.any() returned None!"))?;
        if b {
            // consume pending signal data
            crate::signals::consume_sigpipe(sigfile);
            let exit_reason = 0; // normal exit
            if let Err(err) = send_exit_pkt(&mut ostream, exit_reason) {
                bail!(
                    "Anomalous error occurred while sending exit packet: {}",
                    err
                );
            }
            return Ok(false);
        }
        // check tcp connection
        let if_flag = if_fd
            .any()
            .ok_or(anyhow!("ERROR: if_fd.any() returned None!"))?;
        let tcp_flag = tcp_fd
            .any()
            .ok_or(anyhow!("ERROR: tcp_fd.any() returned None!"))?;
        if tcp_flag {
            loop {
                if let Status::ExitOk = handle_remote2local_pkt(iffile, &mut istream, &mut buffer)?
                {
                    // remote endpoint exited
                    println!("Remote exit!");
                    return Ok(true);
                }
                // https://doc.rust-lang.org/std/io/struct.BufReader.html#method.buffer
                if istream.buffer().is_empty() {
                    if if_flag {
                        handle_local2remote_pkt(iffile, &mut ostream, &mut counter, &mut buffer)?;
                    }
                    break;
                }
            }
        }
    }
}
