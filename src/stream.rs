mod connection;
mod server;

use std::sync::{Arc, Mutex};
use std::net::TcpListener;
use std::thread;
use connection::Connection;
use super::playlist::PlayList;

pub struct StreamServer {}

impl StreamServer {
    pub fn start(playlist: Arc<Mutex<PlayList>>) {
        let address = "0.0.0.0:1935";
        let listener = TcpListener::bind(address).unwrap();
        println!("stream server on rtmp://{}", address);

        thread::spawn(move || {
            for stream in listener.incoming() {
                Connection::new(stream.unwrap(), playlist.clone());
                println!("new stream connection!");
            }
        });
    }
}
