//! A simple proxy server.
//!
//! todo Explain how it works

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
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, cache: Cache) -> Result<(), Error> {
    let mut buf = [0; BUF_SIZE];
    let mut headers = [httparse::EMPTY_HEADER; HEADERS_COUNT]; // TODO abstraction leak
    let _ = stream.read(&mut buf[..])?;

    let req = Request::parse(&buf, &mut headers)?;
    {
        let mut c = cache.lock().expect("todo msg");
        let target_resource = req.resource()?;
        if let Some(data) = c.get(&target_resource) {
            stream.write(data)?;
            stream.flush()?;
        } else {
            let authority = req.authority()?;
            let mut s = TcpStream::connect(&authority).unwrap();
            // https://tools.ietf.org/html/rfc2616#section-8.1.2
            // todo method
            let req = format!(
                "GET / HTTP/1.1\r\nHost: {}\r\nUser-Agent: MyProxy/7.58.0\r\nAccept: */*\r\nConnection: close\r\n\r\n",
                authority
            );
            s.write(req.as_bytes())?;
            s.flush()?;
            let mut caching_buf = Vec::new();
            loop {
                let mut buf = [0; 512];
                match s.read(&mut buf) {
                    Ok(0) => break,
                    Ok(i) => {
                        caching_buf.extend_from_slice(&buf[..i]);
                        stream.write(&buf[..i])?;
                    },
                    Err(e) => return Err(anyhow!(e))
                }
            }
            stream.flush()?;
            c.insert(target_resource, caching_buf);
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
