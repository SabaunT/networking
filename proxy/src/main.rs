//! A simple proxy server.
//!
//! todo Explain how it works

// TODO
// 1. ProxyStream logic to HttpProxy struct, which can perform server tasks (reading requests) and client tasks (sending requests and reading response)

#[macro_use]
extern crate anyhow;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::Error;

use cache::Cache;
use http::{ProxyListener, ProxyStream};

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
    let mut cache = Cache::default();
    let proxy_server_socket = ProxyListener::bind("127.0.0.1:8080")?;

    for stream in proxy_server_socket.incoming() {
        // todo multithread
        let stream = stream?;
        handle_connection(stream, cache.clone())?;
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, cache: Cache) -> Result<(), Error> {
    let mut client_buf = [0; REQ_SIZE];
    let _ = stream.read(&mut client_buf[..])?;

    let client_req = http::Request::try_from(client_buf.as_ref())?;
    {
        let mut c = cache.lock().expect("todo msg");
        let target_url = client_req.target_url();

        if let Some(data) = c.get(&target_url) {
            stream.write(data)?;
            stream.flush()?;
        } else {
            let mut s = ProxyStream::connect(client_req.authority())?;
            s.send_req(&client_req)?;
            let caching_buf = s.read_resp()?;

            stream.write(&caching_buf)?;
            stream.flush()?;

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

    pub(super) struct ProxyStream(TcpStream);

    impl ProxyStream {
        pub(super) fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self, Error> {
            TcpStream::connect(addr).map(ProxyStream).map_err(|e| e.into())
        }

        pub(super) fn send_req(&mut self, req: &Request) -> Result<(), Error> {
            let ProxyStream(ref mut stream) = self;
            let ser_req = req.serialize(); // todo use builder
            stream.write(&ser_req)?;
            stream.flush()?;
            Ok(())
        }

        pub(super) fn read_resp(&mut self) -> Result<Vec<u8>, Error> {
            let ProxyStream(ref mut stream) = self;
            let mut ret = Vec::new();
            loop {
                let mut buf = [0; 512];
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(i) => {
                        ret.extend_from_slice(&buf[..i]);
                    }
                    Err(e) => return Err(anyhow!(e)),
                }
            }
            Ok(ret)
        }
    }

    impl Deref for ProxyStream {
        type Target = TcpStream;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    pub(super) struct ProxyListener(TcpListener);

    impl ProxyListener {
        pub(super) fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, Error> {
            TcpListener::bind(addr).map(ProxyListener).map_err(|e| e.into())
        }
    }

    impl Deref for ProxyListener {
        type Target = TcpListener;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[derive(Default)]
    pub(super) struct RequestBuilder {
        method: Option<String>,
        host: Option<String>,
        path: Option<String>,
        port: Option<u16>,
        // headers
    }

    pub(super) struct Request {
        pub(super) method: String,
        pub(super) host: String,
        pub(super) path: String,
        pub(super) port: u16,
        // headers
    }

    // impl RequestBuilder {
    //     pub(super) fn new() -> Self {
    //         Self::default()
    //     }
    //
    //     pub(super) fn build(self) -> Result<Request, Error> {
    //         let RequestBuilder { method, host, path, port } = self;
    //         if let (Some(method), Some(host), Some(path), Some(port)) = (method, host, path, port) {
    //             return Ok( Request { method, host, path, port} )
    //         }
    //         return Err(anyhow!("Can't build request: not all fields are set enough data"));
    //     }
    //
    //     pub(super) fn method(self, m: String) -> Self {
    //         let RequestBuilder { method: _, host, path, port } = self;
    //         Self { method: Some(m), host, path, port }
    //     }
    //
    //     pub(super) fn host(self, h: String) -> Self {
    //         let RequestBuilder { method, host: _, path, port } = self;
    //         Self { method, host: Some(h), path, port }
    //     }
    //
    //     pub(super) fn path(self, p: String) -> Self {
    //         let RequestBuilder { method, host, path: _, port } = self;
    //         Self { method, host, path: Some(p), port }
    //     }
    //
    //     pub(super) fn port(self, p: u16) -> Self {
    //         let RequestBuilder { method, host, path, port: _ } = self;
    //         Self { method, host, path, port: Some(p) }
    //     }
    // }

    impl Request {
        // pub(super) fn builder() -> RequestBuilder {
        //     RequestBuilder::new()
        // }

        pub(super) fn target_url(&self) -> String {
            format!("{}:{}{}", self.host, self.port, self.path)
        }

        pub(super) fn authority(&self) -> String {
            format!("{}:{}", self.host, self.port)
        }

        pub(super) fn serialize(&self) -> Vec<u8> {
            // Closing connection due to https://tools.ietf.org/html/rfc2616#section-8.1.2
            let req = format!(
                "{} {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: MyProxy/7.58.0\r\nAccept: */*\r\nConnection: close\r\n\r\n",
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
            if let Some(p) = r.path {
                let p = p.trim_start_matches("/");
                let url = Url::parse(p)?;
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
