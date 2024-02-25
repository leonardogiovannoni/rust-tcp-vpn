
// Contains code for handling signal related operations

use ctrlc;
// https://docs.rs/nix/latest/nix/sys/signal/struct.SigSet.html
use nix::sys::signal::{SigmaskHow, Signal, SigSet};
use std::fs::File;
use std::io::{Read, Write};
use std::thread;

const THREAD_NAME: &str = "sigthread";

/// Spawn thread charged of handling SIGINT, disable SIGINT on caller thread
/// WARNING! Must be called at most once!
pub fn spawn_sig_handler() -> std::fs::File {
    // https://docs.rs/nix/0.28.0/nix/poll/struct.PollFd.html#method.new
    // https://docs.rs/nix/latest/nix/unistd/fn.pipe.html
    let (r, w) = nix::unistd::pipe().unwrap();
    let mut w: File = w.into();
    // set handler
    ctrlc::set_handler(move || {
        w.write(&([1] as [u8; 1])).unwrap();
    })
    .expect("Error setting Ctrl-C handler");
    // set handler thread
    // https://doc.rust-lang.org/std/thread/fn.spawn.html
    // note: as the returned handler is dropped the thread is
    // automatically detatched
    // Use Builder to set thread name
    //  https://doc.rust-lang.org/std/thread/struct.Builder.html
    //  https://doc.rust-lang.org/std/thread/index.html#naming-threads
    thread::Builder::new().name(THREAD_NAME.to_string()).spawn(|| {
        loop {
            // run forever
            nix::unistd::pause();
        }
    // https://doc.rust-lang.org/std/result/enum.Result.html#method.expect
    }).expect(&format!("Error spawning thread '{}'", THREAD_NAME));
    // block sigint in main thread
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGINT);
    nix::sys::signal::pthread_sigmask(
        SigmaskHow::SIG_BLOCK,
        Some(&mask),
        None
    ).unwrap();
    // spawned thread remains the only able to handle SIGINT
    let r: File = r.into();
    r
}

// consume data waiting inside the 
pub fn consume_sigpipe(sigfile: &mut std::fs::File) {
    let mut buf : [u8; 8] = [0; 8];
    sigfile.read(&mut buf[..]).unwrap();
}
