// Contains code for handling signal related operations

use ctrlc;
// https://docs.rs/nix/latest/nix/sys/signal/struct.SigSet.html
use anyhow::{Result, bail};
use nix::sys::signal::{SigSet, SigmaskHow, Signal};
use std::fs::File;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
const THREAD_NAME: &str = "sigthread";

// Signal thread created?
// https://doc.rust-lang.org/std/keyword.static.html
static STARTED: AtomicBool = AtomicBool::new(false);
// signal handling enabled? Or termination should be forced?
static HANDLE_SIGNAL: AtomicBool = AtomicBool::new(false);

// should handle interrupt or let the process terminate?
pub fn handle_interrupt(flag: bool) {
    HANDLE_SIGNAL.store(flag, Ordering::Relaxed);
}

/// Spawn thread charged of handling SIGINT, disable SIGINT on caller thread
/// WARNING! Must be called at most once!
//
// internally call handle_interrupt(true)
pub fn spawn_sig_handler() -> Result<File> {
    loop {
        let old = STARTED.load(Ordering::Relaxed);
        if old {
            bail!(
                "Cannot call {} multiple times!",
                "signals::spawn_sig_handler"
            );
        }
        if STARTED
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            break;
        }
    }

    // https://docs.rs/nix/0.28.0/nix/poll/struct.PollFd.html#method.new
    // https://docs.rs/nix/latest/nix/unistd/fn.pipe.html
    // syscall is not expected to fail, if so must panic!

    let (r, w) = nix::unistd::pipe().unwrap();
    let mut w: File = w.into();
    // set handler
    ctrlc::set_handler(move || {
        // terminate is signal must not be handled
        if !HANDLE_SIGNAL.load(Ordering::Relaxed) {
            std::process::exit(1);
        }
        // handle signal - should never fail
        w.write_all(&([1] as [u8; 1])).unwrap();
    })?;
    // set handler thread
    // https://doc.rust-lang.org/std/thread/fn.spawn.html
    // note: as the returned handler is dropped the thread is
    // automatically detatched
    // Use Builder to set thread name
    //  https://doc.rust-lang.org/std/thread/struct.Builder.html
    //  https://doc.rust-lang.org/std/thread/index.html#naming-threads
    thread::Builder::new()
        .name(THREAD_NAME.to_string())
        .spawn(|| {
            loop {
                // run forever
                nix::unistd::pause();
            }
            // https://doc.rust-lang.org/std/result/enum.Result.html#method.expect
        })?;
    // block sigint in main thread
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGINT);
    // if fails to block signal should panic
    nix::sys::signal::pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&mask), None).unwrap();
    // handle signals
    handle_interrupt(true);
    // spawned thread remains the only able to handle SIGINT
    Ok(r.into())
}

// consume data waiting inside the
pub fn consume_sigpipe(sigfile: &mut std::fs::File) {
    let mut buf = [0; 8];
    // if fail to read from pipe bad error occurs!
    let _ = sigfile.read(&mut buf).unwrap();
}
