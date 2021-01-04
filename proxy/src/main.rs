//! A simple proxy server.
//!
//! todo Explain how it works

#[macro_use]
extern crate anyhow;

use std::convert::TryFrom;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::{Mutex, Arc};
use std::collections::HashMap;

use anyhow::Error;

use cache::Cache;

// Client request to proxy server meta - data
const REQ_SIZE: usize = 4096;

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
    let mut buf = [0; REQ_SIZE];
    let _ = stream.read(&mut buf[..])?;

    let req = http::Request::try_from(&buf)?;
    {
        let mut c = cache.lock().expect("todo msg");
        let target_url = req.target_url().ok_or(anyhow!("Invalid request: invalid target url"))?;
        let mut ret_bytes = None;

        if let Some(data) = c.get(&target_url) {
            ret_bytes = Some(data);
        } else {
            let authority = req.target_url()?;
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

        // stream.write() and stream.flush()
    }
    Ok(())
}

mod http {
    use std::net::TcpStream;
    use std::convert::TryFrom;

    use anyhow::Error;
    use url::Url;
    use httparse;

    pub(super) struct RequestBuilder(Request);

    pub(super) struct Request {
        pub(super) method: Option<String>,
        pub(super) host: Option<String>,
        pub(super) path: Option<String>,
        pub(super) port: Option<u16>,
        // headers
    }

    impl Request {
        pub(super) fn target_url(&self) -> Option<String> {
            if let (Some(h), Some(port), Some(path)) = (&self.host, self.port, &self.path) {
                let ret = format!("{}:{}{}", h, port, path);
                return Some(ret)
            }
            None
        }
    }

    // todo clean-up
    impl TryFrom<&[u8]> for Request {
        type Error = Error;

        fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
            // todo hide it in some mode under HttpParser trait
            let r = {
                let mut headers = [httparse::EMPTY_HEADER; 64];
                let mut r = httparse::Request::new(&headers);
                r.parse(data)?;
                r
            };
            let method = r.method.map(ToString::to_string);
            let (host, path, port) = {
                r.path.map(|p| {
                    let p = p.trim_start_matches("/");
                    let url = Url::parse(p)?;
                    let host = url.host_str().map(ToString::to_string);
                    let path = Some(url.path().to_string());
                    let port = url.port_or_known_default();
                    (host, path, port)
                }).ok_or(anyhow!("Invalid request: request has not requesting resource"))?
            };
            Ok(Request {
                method,
                host,
                path,
                port
            })
        }
    }
}

mod cache {
    use std::sync::{Mutex, Arc};
    use std::collections::HashMap;

    pub(super) type Cache = Arc<Mutex<HashMap<String, Vec<u8>>>>;

}
