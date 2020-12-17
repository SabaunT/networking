use rand::Rng;

use peng;

fn main() {
    peng::run(serve_peng)
}

fn serve_peng() -> Result<(), peng::AnyError> {
    let udp_sock = peng::new_udp_sock(peng::SERVER_ADDR, None)?;

    println!("Ready for UDP packets");

    let mut rng = rand::thread_rng();
    let mut incrementor = 0u64;
    loop {
        let mut buf = peng::PengBuf::default();
        let (_, addr) = udp_sock.recv_from(&mut buf[..]).expect("internal error: read timeout is met");
        println!("received from {}: {}", addr, peng::buf_to_data(buf));

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
