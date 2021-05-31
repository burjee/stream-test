use slab::Slab;
use std::collections::HashSet;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use websocket::sync::Server;
use websocket::OwnedMessage;
use super::playlist::PlayList;

pub enum ServerMessage {
    Off,
    Live,
}

struct ClientMessage {
    pub id: usize,
    pub message: OwnedMessage,
}

pub struct ChatServer {}

impl ChatServer {
    pub fn start(playlist: Arc<Mutex<PlayList>>) {
        let address = "0.0.0.0:4343";
        let server = Server::bind(address).unwrap();
        let (tx, rx) = mpsc::channel();
        let connections_map = Arc::new(Mutex::new(Slab::new()));
        let connections = Arc::new(Mutex::new(HashSet::new()));
        handle_message(connections_map.clone(), connections.clone(), rx);
        handle_status(playlist.clone(), connections_map.clone(), connections.clone());

        thread::spawn(move || {
            for request in server.filter_map(Result::ok) {
                if !request.protocols().contains(&String::from("yo-websocket")) {
                    request.reject().unwrap();
                    continue;
                }

                let client = request.use_protocol("yo-websocket").accept().unwrap();
                let (mut receiver, sender) = client.split().unwrap();
                let mut map = connections_map.lock().unwrap();
                let mut ids = connections.lock().unwrap();
                let id = map.insert(sender);
                ids.insert(id);

                let tx_copy = mpsc::Sender::clone(&tx);
                thread::spawn(move || {
                    for message in receiver.incoming_messages() {
                        match message {
                            Err(_) => return,
                            Ok(message) => tx_copy.send(ClientMessage { id, message }).unwrap(),
                        }
                    }
                });
                println!("new chat connection!");
            }
        });
        println!("chat server on ws://{}", address);
    }
}

type Sender = websocket::sender::Writer<std::net::TcpStream>;

fn handle_status(playlist: Arc<Mutex<PlayList>>, connections_map: Arc<Mutex<Slab<Sender>>>, connections: Arc<Mutex<HashSet<usize>>>) {
    thread::spawn(move || {
        let rx = {
            let playlist = playlist.lock().unwrap();
            playlist.rx.clone()
        };
        loop {
            match rx.lock().unwrap().recv() {
                Ok(server_message) => match server_message {
                    ServerMessage::Live => {
                        let mut map = connections_map.lock().unwrap();
                        let ids = connections.lock().unwrap();
                        for &id in &*ids {
                            let sender = map.get_mut(id).unwrap();
                            let message = OwnedMessage::Text(String::from("server@;live"));
                            sender.send_message(&message).unwrap();
                        }
                        println!("live!");
                    }
                    ServerMessage::Off => println!("off!"),
                },
                Err(mpsc::RecvError) => {
                    println!("chat status channel closed!");
                    return;
                }
            }
        }
    });
}

fn handle_message(connections_map: Arc<Mutex<Slab<Sender>>>, connections: Arc<Mutex<HashSet<usize>>>, rx: mpsc::Receiver<ClientMessage>) {
    thread::spawn(move || loop {
        match rx.recv() {
            Err(mpsc::RecvError) => {
                println!("chat channel closed!");
                return;
            }
            Ok(client_message) => {
                let mut map = connections_map.lock().unwrap();
                let mut ids = connections.lock().unwrap();
                match client_message.message {
                    OwnedMessage::Close(_) => {
                        let message = OwnedMessage::Close(None);
                        let sender = map.get_mut(client_message.id).unwrap();
                        sender.send_message(&message).unwrap();
                        map.remove(client_message.id);
                        ids.remove(&client_message.id);
                        println!("chat client disconnected!");
                    }
                    OwnedMessage::Ping(ping) => {
                        let message = OwnedMessage::Pong(ping);
                        let sender = map.get_mut(client_message.id).unwrap();
                        sender.send_message(&message).unwrap();
                    }
                    OwnedMessage::Text(message) => {
                        for &id in &*ids {
                            let sender = map.get_mut(id).unwrap();
                            sender.send_message(&OwnedMessage::from(format!("client@;{}", message))).unwrap()
                        }
                    }
                    _ => {
                        println!("unhandle message");
                    }
                }
            }
        }
    });
}
