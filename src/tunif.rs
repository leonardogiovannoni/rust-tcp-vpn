pub mod wrapper {
    extern "C" {
        pub fn set_interface_name(if_fd: cty::c_int, ifname: *const cty::c_char);
        pub fn set_interface_address(
            if_fd: cty::c_int,
            ifname: *const cty::c_char,
            addr: *const cty::c_char,
            netmask: cty::c_int,
        );
        pub fn set_interface_up(if_fd: cty::c_int, ifname: *const cty::c_char);
        pub fn set_interface_down(if_fd: cty::c_int, ifname: *const cty::c_char);
    }
}

use std::net::IpAddr;
use std::os::fd::AsRawFd;

pub fn set_interface_name(iffile: &std::fs::File, ifname: &str) {
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

pub fn set_interface_address(iffile: &std::fs::File, ifname: &str, addr: &IpAddr, netmask: i32) {
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
        IpAddr::V4(_) => addr.to_string(),
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

pub fn set_interface_up(iffile: &std::fs::File, ifname: &str) {
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

pub fn set_interface_down(iffile: &std::fs::File, ifname: &str) {
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

const DEV_FILE: &str = "/dev/net/tun";

// inizialize virtual interface but do not bring it up
pub fn initialize_tun_interface(ifname: &str, ifaddr: IpAddr, netmask: u8) -> std::fs::File {
    // open virtual device
    let iffile = match std::fs::File::options()
        .read(true)
        .write(true)
        .open(DEV_FILE)
    {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Error opening file {}: {}", DEV_FILE, err);
            std::process::exit(1)
        }
    };
    // set interface name
    set_interface_name(&iffile, &ifname);
    // set interface ip
    set_interface_address(&iffile, &ifname, &ifaddr, netmask as i32);
    // return file handler
    iffile
}
