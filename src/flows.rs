
use std::io::{BufReader, BufWriter, Read, Write};
use std::fs::File;
use std::net::TcpStream;
use std::thread;
use ctrlc;



fn handle_local2remote_pkt(iffile: &mut std::fs::File, stream: &mut impl std::io::Write, counter: &mut u64, buffer: &mut [u8]) {
    // packet is always fully read (if possible):
    // this is a special case tied to virtual interface
    // internals
    let sz = match iffile.read(buffer) {
        Ok(0) => {
            panic!("UNEXPECTED EMPTY PACKET!");
        },
        Ok(sz) => {
            // new packet
            *counter += 1;
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


fn handle_remote2local_pkt(iffile: &mut std::fs::File, stream: &mut impl std::io::BufRead, buffer: &mut [u8]) {
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


// https://docs.rs/nix/0.28.0/nix/poll/struct.PollFd.html
pub fn handle_flow(stream: &mut TcpStream, iffile: &mut std::fs::File) -> () {

    // https://docs.rs/nix/0.28.0/nix/poll/struct.PollFd.html#method.new
    //      let (r, w) = pipe().unwrap();
    //      let pfd = PollFd::new(r.as_fd(), PollFlags::POLLIN);
    //      let mut fds = [pfd];
    //      poll(&mut fds, PollTimeout::NONE).unwrap();
    //      let mut buf = [0u8; 80];
    //      read(r.as_raw_fd(), &mut buf[..]);

    // https://docs.rs/nix/latest/nix/unistd/fn.pipe.html
    let (r, w) = nix::unistd::pipe().unwrap();
    let mut w:File = w.into();
    ctrlc::set_handler(move || {
        eprintln!("HANDLER!");
        w.write(&([1] as [u8; 1])).unwrap();
    }).expect("Error setting Ctrl-C handler");

    // buffer
    let mut buffer: [u8; 4096] = [0; 4096];
    // split both socket ends
    let mut ostream = BufWriter::with_capacity(64+4096, stream.try_clone().unwrap());
    let mut istream = BufReader::with_capacity(64+4096, stream.try_clone().unwrap());    
    // count how many packets are sent?
    let mut counter: u64 = 0;

    loop {
        use nix::poll::PollFlags;
        use nix::poll::PollFd;
        use nix::poll::PollTimeout;
        use std::os::fd::AsFd;

        
        // to pool pipe read end
        let pipe_fd = PollFd::new(r.as_fd(), PollFlags::POLLIN);
        // to pool tcp stream
        let tcp_fd = PollFd::new(stream.as_fd(), PollFlags::POLLIN);
        // to pool interface
        let if_fd = PollFd::new(iffile.as_fd(), PollFlags::POLLIN);
        // prepare input 
        let mut fds = [pipe_fd, tcp_fd, if_fd];
        // https://docs.rs/nix/0.28.0/nix/poll/fn.poll.html
        let ret = nix::poll::poll(&mut fds, PollTimeout::NONE).unwrap();
        if ret <= 0 {
            panic!("Non positive nix::poll::poll");
        }
        let [pipe_fd, tcp_fd, if_fd] = fds;
        if pipe_fd.any().unwrap() {
            eprintln!("CTRL+C");
            // TODO: send exit packet
            break;
        }
        // check tcp connection
        let if_flag = if_fd.any().unwrap();
        let tcp_flag = tcp_fd.any().unwrap();
        // https://doc.rust-lang.org/std/mem/fn.drop.html
        // std::mem::drop(if_fd);
        // std::mem::drop(tcp_fd);
        // check interface
        if tcp_flag {
            handle_remote2local_pkt(iffile, &mut istream, &mut buffer);
        }
        if if_flag {
            handle_local2remote_pkt(iffile, &mut ostream, &mut counter, &mut buffer);
        }
    }
}
