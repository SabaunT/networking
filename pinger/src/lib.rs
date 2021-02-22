//! Actually it's a kind of utils module for binaries

use std::net::{ToSocketAddrs, UdpSocket};
use std::time::Duration;

use anyhow::Error;

pub type PingBuf = [u8; 8];

pub const SERVER_ADDR: &str = "127.0.0.1:8000";
pub const CLIENT_ADDR: &str = "127.0.0.1:8001";

pub fn run<F: FnOnce() -> Result<(), Error>>(f: F) {
    std::process::exit(match f() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    });
}

pub fn new_udp_sock<T: ToSocketAddrs>(addr: T, read_timeout: Option<Duration>) -> Result<UdpSocket, Error> {
    let sock = UdpSocket::bind(addr).expect("port is used");
    sock.set_read_timeout(read_timeout)?;

    Ok(sock)
}

pub fn buf_to_data(buf: PingBuf) -> impl std::fmt::Display {
    to_u64(buf)
}

fn to_u64(buf: PingBuf) -> u64 {
    u64::from_be_bytes(buf)
}
