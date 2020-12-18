use rand::Rng;

use pinger;

fn main() {
    pinger::run(serve_ping)
}

fn serve_ping() -> Result<(), pinger::AnyError> {
    let udp_sock = pinger::new_udp_sock(pinger::SERVER_ADDR, None)?;

    println!("Ready for UDP packets");

    let mut rng = rand::thread_rng();
    let mut incrementor = 0u64;
    loop {
        let mut buf = pinger::PingBuf::default();
        let (_, addr) = udp_sock.recv_from(&mut buf[..]).expect("internal error: read timeout is met");
        println!("received from {}: {}", addr, pinger::buf_to_data(buf));

        // just drop
        if rng.gen_range(0, 10) > 6 {
            continue;
        }

        udp_sock.send_to(&incrementor.to_be_bytes(), addr)?;
        incrementor += 1;
        if incrementor == 30 {
            break;
        }
    }

    Ok(())
}
