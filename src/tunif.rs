use std::ffi::{CStr, CString};
use std::fs::File;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::fd::{AsFd, AsRawFd};

use anyhow::{Result, bail};
use socket2::SockAddr;

#[inline(always)]
fn get_netmask(n: u8) -> u32 {
    assert!(n <= 32);
    0xFFFFFFFFu32.wrapping_shl(32 - n as u32)
}

struct Socket(i32);

impl Socket {
    fn new() -> Result<Self> {
        let socket = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
        if socket < 0 {
            bail!("Error creating socket")
        }
        Ok(Socket(socket))
    }

    fn ioctl(&self, request: u64, mut ifr: libc::ifreq) -> Result<()> {
        let tmp: *mut libc::ifreq = &raw mut ifr;
        let res = unsafe { libc::ioctl(self.0, request, tmp) };
        if res < 0 {
            let err = std::io::Error::last_os_error();
            bail!("Error setting interface address: {}", err);
        }
        Ok(())
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

unsafe fn ifr_create(ifname: &CStr, flags_fn: impl FnOnce(&mut libc::ifreq)) -> libc::ifreq {
    let mut ifr: libc::ifreq = unsafe { core::mem::zeroed() };
    for (i, &c) in ifname.to_bytes().iter().enumerate() {
        ifr.ifr_name[i] = c as i8;
    }
    ifr.ifr_name.last_mut().map(|c| *c = 0).unwrap();
    flags_fn(&mut ifr);
    ifr
}

fn set_interface_name(file: &File, ifname: &CStr) -> Result<()> {
    let ifr = unsafe {
        ifr_create(ifname, |ifr| {
            ifr.ifr_ifru.ifru_flags = (libc::IFF_TUN | libc::IFF_NO_PI) as i16;
        })
    };
    let res = unsafe { libc::ioctl(file.as_raw_fd(), libc::TUNSETIFF, &ifr) };
    if res < 0 {
        let err = std::io::Error::last_os_error();
        bail!("Error setting interface name: {}", err);
    }
    Ok(())
}

fn set_interface_address(socket: &Socket, ifname: &CStr, addr: &Ipv4Addr) -> Result<()> {
    let sockaddr: SockAddr = SocketAddrV4::new(*addr, 0).into();
    let tmp = unsafe { core::ptr::read(sockaddr.as_ptr()) };
    let ifr = unsafe { ifr_create(ifname, |ifr| ifr.ifr_ifru.ifru_addr = tmp) };
    socket.ioctl(libc::SIOCSIFADDR, ifr)?;
    Ok(())
}

fn set_subnet_mask(socket: &Socket, ifname: &CStr, netmask: u8) -> Result<()> {
    let mask = get_netmask(netmask).to_be_bytes();
    let ipv4addr = Ipv4Addr::new(mask[0], mask[1], mask[2], mask[3]);
    let sockaddr: SockAddr = SocketAddrV4::new(ipv4addr, 0).into();
    let tmp = unsafe { core::ptr::read(sockaddr.as_ptr()) };
    let ifr = unsafe { ifr_create(ifname, |ifr| ifr.ifr_ifru.ifru_netmask = tmp) };
    socket.ioctl(libc::SIOCSIFNETMASK, ifr)?;
    Ok(())
}

fn set_interface_up(socket: &Socket, ifname: &CStr) -> Result<()> {
    let ifr = unsafe {
        ifr_create(ifname, |ifr| {
            ifr.ifr_ifru.ifru_flags |= libc::IFF_UP as i16;
        })
    };
    socket.ioctl(libc::SIOCSIFFLAGS, ifr)?;
    Ok(())
}

fn set_interface_down(socket: &Socket, ifname: &CStr) -> Result<()> {
    let ifr = unsafe {
        ifr_create(ifname, |ifr| {
            ifr.ifr_ifru.ifru_flags &= !(libc::IFF_UP as i16);
        })
    };
    socket.ioctl(libc::SIOCSIFFLAGS, ifr)?;
    Ok(())
}

pub struct Iface {
    fd: File,
    name: CString,
    ip: Ipv4Addr,
    netmask: u8,
}

impl Iface {
    pub fn new(n: &str, ip: Ipv4Addr, netmask: u8) -> Result<Self> {
        if netmask > 32 {
            bail!("Netmask should be less than 32")
        }
        if n.len() > 16 {
            bail!("Interface name too long")
        }
        let fd = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")?;

        let name = CString::new(n)?;
        set_interface_name(&fd, &name)?;
        let socket = Socket::new()?;
        set_interface_address(&socket, &name, &ip)?;
        set_subnet_mask(&socket, &name, netmask)?;
        set_interface_up(&socket, &name)?;
        Ok(Iface {
            fd,
            name,
            ip,
            netmask,
        })
    }
}
impl AsRef<File> for Iface {
    fn as_ref(&self) -> &File {
        &self.fd
    }
}

impl AsFd for Iface {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl Drop for Iface {
    fn drop(&mut self) {
        let f = || -> Result<()> { set_interface_down(&Socket::new()?, &self.name) };
        f().unwrap();
    }
}
