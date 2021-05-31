use rml_rtmp::handshake::{Handshake, HandshakeProcessResult, PeerType};
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use super::server::{Server, ServerResult};
use super::PlayList;

pub struct Connection {
    socket: TcpStream,
    handshake: Handshake,
    handshake_completed: bool,
    server: Server,
}

impl Connection {
    const BUFFER_SIZE: usize = 4096;

    pub fn new(socket: TcpStream, playlist: Arc<Mutex<PlayList>>) {
        // let mut socket = socket.try_clone().unwrap();
        thread::spawn(|| {
            let mut connection = Connection {
                socket: socket,
                handshake: Handshake::new(PeerType::Server),
                handshake_completed: false,
                server: Server::new(playlist),
            };
            connection.start_socket_reader();
        });
    }

    fn start_socket_reader(&mut self) {
        let mut buffer = [0; Connection::BUFFER_SIZE];
        loop {
            let result = match self.socket.read(&mut buffer) {
                Ok(0) => {
                    self.server.end_stream();
                    return;
                }
                Ok(count) => {
                    if self.handshake_completed {
                        self.server.handle_bytes(&buffer[..count])
                    } else {
                        self.handshake(&buffer[..count])
                    }
                }
                Err(error) => {
                    println!("Error occurred reading from socket: {:?}", error);
                    self.server.end_stream();
                    return;
                }
            };

            let server_results = match result {
                Ok(results) => results,
                Err(error) => {
                    println!("Input caused the following server error: {}", error);
                    return;
                }
            };

            for result in server_results.into_iter() {
                match result {
                    ServerResult::Response { packet } => self.write(packet.bytes),
                    ServerResult::Disconnect => {
                        self.server.end_stream();
                        return;
                    }
                }
            }
        }
    }

    pub fn write(&mut self, bytes: Vec<u8>) {
        match self.socket.write(&bytes) {
            Ok(_) => (),
            Err(error) => {
                println!("Error writing to socket: {:?}", error);
            }
        }
    }

    fn handshake(&mut self, bytes: &[u8]) -> Result<Vec<ServerResult>, String> {
        let result = match self.handshake.process_bytes(bytes) {
            Ok(result) => result,
            Err(error) => {
                println!("Handshake error: {:?}", error);
                return Err(error.to_string());
            }
        };

        match result {
            HandshakeProcessResult::InProgress { response_bytes } => {
                if response_bytes.len() > 0 {
                    self.write(response_bytes);
                }
                Ok(vec![])
            }

            HandshakeProcessResult::Completed { response_bytes, remaining_bytes } => {
                println!("Handshake successful!");
                if response_bytes.len() > 0 {
                    self.write(response_bytes);
                }

                self.handshake_completed = true;
                self.server.handle_handshake_bytes(&remaining_bytes[..])
            }
        }
    }
}
