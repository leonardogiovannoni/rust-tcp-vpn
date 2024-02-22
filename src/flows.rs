
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::thread;


pub fn handle_local2remote(iffile: std::fs::File, stream: TcpStream) -> thread::JoinHandle<()> {
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

pub fn handle_remote2local(stream: TcpStream, iffile: std::fs::File) -> thread::JoinHandle<()> {
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
pub fn handle_flow(stream: &mut TcpStream, iffile: &mut std::fs::File) -> () {
    let t1_hanle = handle_local2remote(iffile.try_clone().unwrap(), stream.try_clone().unwrap());
    let t2_hanle = handle_remote2local(stream.try_clone().unwrap(), iffile.try_clone().unwrap());
    t1_hanle.join().unwrap();
    t2_hanle.join().unwrap();
}
