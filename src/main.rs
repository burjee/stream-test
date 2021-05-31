mod chat;
mod media;
mod playlist;
mod stream;

use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let playlist = Arc::new(Mutex::new(playlist::PlayList::new()));
    stream::StreamServer::start(playlist.clone());
    chat::ChatServer::start(playlist.clone());
    media::MediaServer::start(playlist.clone()).await;
}
