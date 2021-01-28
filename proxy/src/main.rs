//! A simple proxy server.
//!
//! todo Explain how it works

#[macro_use]
extern crate anyhow;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::Error;

use cache::Cache;
use http::{ProxyServer, ProxyClient};

fn main() {
    loop {
        if let Err(e) = run() {
            eprintln!("An error occurred: {}", e);
        }
    }
}

fn run() -> Result<(), Error> {
    let mut cache = Cache::default();
    let proxy_server_socket = TcpListener::bind("127.0.0.1:8080")?;

    for stream in proxy_server_socket.incoming() {
        // todo multithread
        let stream = stream?;
        handle_connection(stream, cache.clone())?;
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, cache: Cache) -> Result<(), Error> {
    let mut proxy_server = ProxyServer::from_stream(stream);
    let req = proxy_server.read_req()?;

    {
        let mut c = cache.lock().expect("todo msg");
        let target_url = req.target_url();

        if let Some(data) = c.get(&target_url) {
            proxy_server.send_resp(data)?;
        } else {
            let mut proxy_client = ProxyClient::connect(req.authority())?;
            proxy_client.send_req(&req.serialize())?;
            let caching_buf = proxy_client.read_resp()?;

            proxy_server.send_resp(&caching_buf)?;

            c.insert(target_url, caching_buf);
        }
    }
    Ok(())
}

mod http {
    use std::convert::TryFrom;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream, ToSocketAddrs};
    use std::ops::Deref;

    use anyhow::Error;
    use httparse;
    use url::Url;

    const REQ_END: &[u8; 4] = b"\r\n\r\n";

    pub(super) struct ProxyServer(TcpStream);
    pub(super) struct ProxyClient(TcpStream);

    pub(super) struct Request {
        pub(super) method: String,
        pub(super) host: String,
        pub(super) path: String,
        pub(super) port: u16,
        // headers
    }

    impl ProxyServer {
        pub(super) fn from_stream(stream: TcpStream) -> Self {
            Self(stream)
        }

        pub(super) fn send_resp(&mut self, resp: &[u8]) -> Result<(), Error> {
            let ProxyServer(ref mut stream) = self;
            stream.write(&resp)?;
            stream.flush().map(|_| ()).map_err(|e| e.into())
        }

        pub(super) fn read_req(&mut self) -> Result<Request, Error> {
            let ProxyServer(ref mut stream) = self;
            let mut req_bytes = Vec::new();
            loop {
                let mut buf = [0; 512];
                match stream.read(&mut buf) {
                    Ok(i) => {
                        req_bytes.extend_from_slice(&buf[..i]);
                        if buf[..i].ends_with(REQ_END) { break; }
                    }
                    Err(e) => return Err(anyhow!(e)),
                }
            }
            Request::try_from(req_bytes.as_slice())
        }
    }

    impl ProxyClient {
        pub(super) fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self, Error> {
            TcpStream::connect(addr).map(ProxyClient).map_err(|e| e.into())
        }

        pub(super) fn send_req(&mut self, req: &[u8]) -> Result<(), Error> {
            let ProxyClient(ref mut stream) = self;
            stream.write(req)?;
            stream.flush().map(|_| ()).map_err(|e| e.into())
        }

        pub(super) fn read_resp(&mut self) -> Result<Vec<u8>, Error> {
            let ProxyClient(ref mut stream) = self;
            let mut resp_bytes = Vec::new();
            loop {
                let mut buf = [0; 512];
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(i) => {
                        resp_bytes.extend_from_slice(&buf[..i]);
                    }
                    Err(e) => return Err(anyhow!(e)),
                }
            }
            Ok(resp_bytes)
        }
    }

    impl Request {

        pub(super) fn target_url(&self) -> String {
            format!("{}:{}{}", self.host, self.port, self.path)
        }

        pub(super) fn authority(&self) -> String {
            format!("{}:{}", self.host, self.port)
        }

        pub(super) fn serialize(&self) -> Vec<u8> {
            // Closing connection due to https://tools.ietf.org/html/rfc2616#section-8.1.2
            let req = format!(
                "{} {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: SabaunTProxy/0.0.1\r\nAccept: */*\r\nConnection: close\r\n\r\n",
                self.method, self.path, self.host
            );
            req.as_bytes().into()
        }
    }

    // todo clean-up
    impl TryFrom<&[u8]> for Request {
        type Error = Error;

        fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
            // todo hide it in some mode under HttpParser trait
            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut r = httparse::Request::new(&mut headers);
            r.parse(data)?;

            let method = r.method.map(ToString::to_string).ok_or(anyhow!("Invalid request: no http method"))?;
            if let Some(path) = r.path {
                let path = path.trim_start_matches("/");
                let url = Url::parse(path)?;
                let host = url.host_str().map(ToString::to_string).ok_or(anyhow!("Invalid request: no resource url"))?;
                let path = url.path().to_string();
                let port = url.port_or_known_default().ok_or(anyhow!("Invalid request: url scheme default port"))?;
                return Ok(Request { method, host, path, port });
            }
            return Err(anyhow!("Invalid request: request has not requesting resource"));
        }
    }
}

mod cache {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    pub(super) type Cache = Arc<Mutex<HashMap<String, Vec<u8>>>>;
}
