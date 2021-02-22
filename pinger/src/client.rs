use std::io::ErrorKind;
use std::time::{Duration, Instant};

use anyhow::Error;

use pinger;

fn main() {
    pinger::run(ping)
}

fn ping() -> Result<(), Error> {
    let udp_sock = pinger::new_udp_sock(pinger::CLIENT_ADDR, Some(Duration::from_secs(1)))?;
    udp_sock.connect(pinger::SERVER_ADDR)?;

    for num in 0u64..=10 {
        // send
        let sent_at = Instant::now();
        udp_sock.send(&num.to_be_bytes())?;

        // measure recv dur
        let mut buf = pinger::PingBuf::default();
        match udp_sock.recv(&mut buf) {
            Ok(_) => {
                let recv_dur = Instant::now().duration_since(sent_at);
                println!("Request number {} received after {:?}", num, recv_dur);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => println!("Request number {} timed out", num),
            Err(e) => return Err(e.into()),
        };
    }
    Ok(())
}
