use std::convert::TryFrom;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

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
}

impl ProxyServer {
    pub(super) fn from(stream: TcpStream) -> Self {
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
                    if buf[..i].ends_with(REQ_END) {
                        break;
                    }
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

impl TryFrom<&[u8]> for Request {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
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