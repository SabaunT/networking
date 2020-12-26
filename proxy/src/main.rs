#[macro_use]
extern crate anyhow;

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::Mutex;
use std::collections::HashMap;

use anyhow::Error;

use http_utils::Request;

// Resource size implementing io::{Read, Write}
const BUF_SIZE: usize = 4096;
const HEADERS_COUNT: usize = 64;

fn main() {
    loop {
        if let Err(e) = run() {
            eprintln!("An error occurred: {}", e);
        }
    }
}

fn run() -> Result<(), Error> {
    // let mut cache: Mutex<HashMap<_, Vec<_>>> = Mutex::new(HashMap::new());
    let tcp_server_socket = TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in tcp_server_socket.incoming() {
        let stream = stream?;
        handle_connection(stream)?;
    //     let read = {
    //         let s = stream.read(&mut buf[..]).unwrap();
    //
    //         let mut headers = [EMPTY_HEADER; 64];
    //         let mut req = Request::new(&mut headers);
    //         let _ = req.parse(&buf).unwrap();
    //
    //
    //         let mut cache_l = cache.lock().unwrap();
    //         if let Some(val) = cache_l.get(req.path.unwrap()) {
    //             let _ = stream.write(&mut val.clone()).unwrap();
    //         } else {
    //             let url = &req.path.unwrap().trim_start_matches("/www.");
    //             let url = format!("{}:80", url);
    //             println!("{:?}", url);
    //             let mut ss = TcpStream::connect(url).unwrap();
    //             ss.write(&buf[..s]).unwrap();
    //             let mut read_buf = [0; 4096];
    //             let _ = ss.read(&mut read_buf).unwrap();
    //             cache_l.insert(req.path.unwrap().to_string(), read_buf.to_vec());
    //             let _ = stream.write(&mut read_buf).unwrap();
    //         }
    //     };
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<(), Error> {
    let mut buf = [0; BUF_SIZE];
    let mut headers = [httparse::EMPTY_HEADER; HEADERS_COUNT]; // TODO abstraction leak
    let _ = stream.read(&mut buf[..])?;

    let req = Request::parse(&buf, &mut headers)?;
    req.resource()
    // if resource in cache - return response value in bytes, otherwise send another request
}


// todo make it more generic
mod http_utils {
    use url::Url;
    use httparse;
    use anyhow::Error;
    use crate::HEADERS_COUNT;

    pub(super) struct Request<'h, 'b>(httparse::Request<'h, 'b>);

    impl<'h, 'b> Request<'h, 'b> {
        pub(super) fn parse(data: &'b [u8], headers: &'h mut [httparse::Header<'b>; HEADERS_COUNT]) -> Result<Request<'h, 'b>, Error> {
            let mut r = httparse::Request::new(headers);
            let _ = r.parse(data)?;

            Ok(Request(r))
        }

        pub(super) fn resource(&self) -> Result<(), Error> {
            if let Some(path) = self.0.path {
                let path = path.trim_start_matches("/"); // TODO abstraction leak
                let url = Url::parse(path)?;
                // client: curl http://127.0.0.1:8080/http://www.google.com
                // server:
                // println!("{:?}", url.scheme()); // "http"
                // println!("{:?}", url.path()); // "/"
                // println!("{:?}", url.host_str()); // Some("www.google.com")
                // println!("{:?}", url.port()); // None
                // println!("{:?}", url.port_or_known_default()); // Some(80)

                // return key for Cache
            }
            Ok(())
        }
    }
}
