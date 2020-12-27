//! A simple proxy server.
//!
//! todo Explain how it works
//! https://github.com/wlabatey/computer_networking_a_top_down_approach/blob/master/assignments/05_http_proxy/http_proxy.py as an example

#[macro_use]
extern crate anyhow;

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::{Mutex, Arc};
use std::collections::HashMap;

use anyhow::Error;

use http_utils::Request;
use cache::Cache;

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
    let mut cache = Cache::default();
    let tcp_server_socket = TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in tcp_server_socket.incoming() {
        let stream = stream?;
        handle_connection(stream, cache.clone())?;
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

fn handle_connection(mut stream: TcpStream, cache: Cache) -> Result<(), Error> {
    let mut buf = [0; BUF_SIZE];
    let mut headers = [httparse::EMPTY_HEADER; HEADERS_COUNT]; // TODO abstraction leak
    let _ = stream.read(&mut buf[..])?;

    let req = Request::parse(&buf, &mut headers)?;
    {
        let c = cache.lock().expect("todo msg");
        let target_resource = req.resource()?;
        if let Some(data) = c.get(&target_resource) {
            stream.write(data)?;
        } else {
            // TODO perform request to target resource and send received response to `stream`
            // let mut s = TcpStream::connect("www.google.com:80").unwrap();
            // let req = b"GET / HTTP/1.1\r\nHost: www.google.com\r\nUser-Agent: curl/7.58.0\r\nAccept: */*\r\n\r\n";
            // s.write(req).unwrap();
            // let mut read_buf = [0; 4096];
            // let _ = s.read(&mut read_buf).unwrap();
            // stream.write(&read_buf)?;
        }
    }
    Ok(())
}

// todo make it more generic
mod http_utils {
    use std::net::TcpStream;

    use url::Url;
    use httparse;
    use anyhow::Error;

    use crate::HEADERS_COUNT;
    use std::io::{Write, Read};

    pub(super) struct Request<'h, 'b>(httparse::Request<'h, 'b>);

    impl<'h, 'b> Request<'h, 'b> {
        pub(super) fn parse(data: &'b [u8], headers: &'h mut [httparse::Header<'b>; HEADERS_COUNT]) -> Result<Request<'h, 'b>, Error> {
            let mut r = httparse::Request::new(headers);
            let _ = r.parse(data)?;

            Ok(Request(r))
        }

        pub(super) fn authority(&self) -> Result<String, Error> {
            let Request(ref req) = self;
            if let Some(path) = req.path {
                let path = path.trim_start_matches("/"); // TODO abstraction leak
                let url = Url::parse(path)?;
                let ret = Self::authority_from_url(url)?;
                return Ok(ret);
            }
            Err(anyhow!("Invalid request: has not target resource"))
        }

        pub(super) fn resource(&self) -> Result<String, Error> {
            let Request(ref req) = self;
            if let Some(path) = req.path {
                let path = path.trim_start_matches("/"); // TODO abstraction leak
                let url = Url::parse(path)?;
                let ret = Self::resource_from_url(url)?;
                return Ok(ret);
            }
            Err(anyhow!("Invalid request: has not target resource"))
        }

        fn resource_from_url(url: Url) -> Result<String, Error> {
            if let (Some(host), Some(port), path) = (url.host_str(), url.port_or_known_default(), url.path()) {
                return Ok(format!("{}:{}{}", host, port, path));
            }
            Err(anyhow!("Invalid request: invalid target url"))
        }

        fn authority_from_url(url: Url) -> Result<String, Error> {
            if let (Some(host), Some(port)) = (url.host_str(), url.port_or_known_default()) {
                return Ok(format!("{}:{}", host, port));
            }
            Err(anyhow!("Invalid request: invalid target authority"))
        }
    }
}

mod cache {
    use std::sync::{Mutex, Arc};
    use std::collections::HashMap;

    pub(super) type Cache = Arc<Mutex<HashMap<String, Vec<u8>>>>;

}
