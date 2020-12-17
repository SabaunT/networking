//! Actually it's a kind of utils module for binaries

use std::net::{ToSocketAddrs, UdpSocket};
use std::time::Duration;

pub type AnyError = Box<dyn std::error::Error>;
pub type PengBuf = [u8; 8];

pub const SERVER_ADDR: &str = "127.0.0.1:8000";
pub const CLIENT_ADDR: &str = "127.0.0.1:8001";

pub fn run<F: FnOnce() -> Result<(), AnyError>>(f: F) {
    std::process::exit(match f() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    });
}

pub fn new_udp_sock<T: ToSocketAddrs>(addr: T, read_timeout: Option<Duration>) -> Result<UdpSocket, AnyError> {
    let sock = UdpSocket::bind(addr).expect("port is used");
    sock.set_read_timeout(read_timeout)?;

    Ok(sock)
}

pub fn buf_to_data(buf: PengBuf) -> impl std::fmt::Display {
    to_u64(buf)
}

fn to_u64(buf: PengBuf) -> u64 {
    u64::from_be_bytes(buf)
}
