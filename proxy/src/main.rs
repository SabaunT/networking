//! A simple proxy server in accordance to explanation in "Computer networking. A Top-down approach" (Kurose & Ross).
//!
//! Proxy server stands in the middle of a client and original server interaction.
//! It improves the interaction by lowering response time:
//! 1. client performs sends http-request, trying to get some object
//! 2. request is handled by proxy server which checks whether requested object is in proxy's cache.
//! 3.1. If it's in the cache, then proxy server returns the object if it's valid against TTL. So, there is no need to connect to a remote server and
//! cache invalidation happens by demand.
//! 3.2. If it isn't in the cache or object isn't valid against TTL, the proxy server tries to get an object from the original server resending clients request.
//! The response from the original server is returned to the client.

#[macro_use]
extern crate anyhow;

use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Sender};
use std::thread;

use anyhow::Error;

use cache::Cache;
use http::{ProxyClient, ProxyServer};

mod http;
mod cache;

fn main() {
    loop {
        if let Err(e) = run() {
            eprintln!("An error occurred: {}", e);
        }
    }
}

fn run() -> Result<(), Error> {
    let (sender, receiver) = channel();
    let cache = Cache::default();
    let proxy_server_socket = TcpListener::bind("127.0.0.1:8080")?;

    {
        let s = sender.clone();
        thread::spawn(move || {
            for stream in proxy_server_socket.incoming() {
                match stream {
                    Ok(stream) => {
                        let c = cache.clone();
                        let s = sender.clone();
                        thread::spawn(|| handle_connection(stream, c, s));
                    }
                    Err(e) => s.send(Err(e.into())).expect("internal error: receiver end has hung up"),
                };
            }
        });
    }

    // Handling errors from threads
    for res in receiver {
        let _ = res?;
    }

    Ok(())
}

fn handle_connection(stream: TcpStream, cache: Cache, sender: Sender<Result<(), Error>>) {
    let res = handle_connection_impl(stream, cache.clone());
    let _ = sender.send(res).expect("internal error: receiver end has hung up");
}

fn handle_connection_impl(stream: TcpStream, cache: Cache) -> Result<(), Error> {
    let mut proxy_server = ProxyServer::from(stream);
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
