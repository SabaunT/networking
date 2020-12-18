use std::net::TcpListener;
use std::io::Read;

fn main() {
    let tcp_server_socket = TcpListener::bind("127.0.0.1:8080").unwrap();
    for stream in tcp_server_socket.incoming() {
        let mut stream = stream.unwrap();
        let mut buf = [0; 4096];
        let read = {
            let s = stream.read(&mut buf[..]).unwrap();
            let read_bytes = &buf[..s];
            let str_content = String::from_utf8(read_bytes.to_vec()).unwrap();
            println!("{:?}", str_content);
            // parse str_content to get URL
            // check URL in hashmap
            // if url in hashmap - return data
            // otherwise connect to url and get return response (and save it)
        };
    }
}
