use std::sync::{mpsc, Arc, Mutex};
use super::chat::ServerMessage;

pub struct PlayList {
    pub sequence: usize,
    pub m3u8: String,
    pub ts: Vec<(u32, String)>,
    pub timestamp: Vec<u32>,
    pub live: bool,
    pub tx: mpsc::Sender<ServerMessage>,
    pub rx: Arc<Mutex<mpsc::Receiver<ServerMessage>>>,
}

impl PlayList {
    const COUNT: usize = 2;

    pub fn new() -> PlayList {
        let (tx, rx) = mpsc::channel();

        PlayList {
            sequence: 0,
            m3u8: String::from(""),
            ts: vec![],
            timestamp: vec![0],
            live: false,
            tx,
            rx: Arc::new(Mutex::new(rx)),
        }
    }

    pub fn push(&mut self, timestamp: u32, filename: String, end: bool) -> u64 {
        let mut timestamp = timestamp / 1000 + 1;
        let mut duration = timestamp;
        if let Some(t) = self.timestamp.last() {
            if end {
                if let Some(d) = self.ts.last() {
                    timestamp = d.0 + t;
                    duration = d.0;
                }
            } else {
                duration = timestamp - t;
            }
        }

        self.ts.push((duration, filename));
        self.timestamp.push(timestamp);
        self.update(end);
        duration as u64
    }

    pub fn update(&mut self, end: bool) {
        if self.ts.len() >= PlayList::COUNT {
            if self.ts.len() == PlayList::COUNT + 1 && !end {
                self.ts.remove(0);
                self.timestamp.remove(0);
            }
            let mut target_duration = 0;
            let mut list = String::from("");
            for ts in &self.ts {
                list = format!("{}#EXTINF:{}.0000\r\n", list, ts.0);
                list = format!("{}http://127.0.0.1:1337/{}\r\n", list, ts.1);
                target_duration = if target_duration <= ts.0 { ts.0 + 1 } else { target_duration }
            }

            let mut m3u8 = String::from("");
            m3u8 = format!("{}#EXTM3U\r\n", m3u8);
            m3u8 = format!("{}#EXT-X-VERSION:3\r\n", m3u8);
            m3u8 = format!("{}#EXT-X-TARGETDURATION:{}\r\n", m3u8, target_duration);
            m3u8 = format!("{}#EXT-X-MEDIA-SEQUENCE:{}\r\n", m3u8, self.sequence);
            m3u8 = format!("{}{}", m3u8, list);
            if end {
                m3u8 = format!("{}#EXT-X-ENDLIST\r\n", m3u8);
            }
            self.m3u8 = m3u8;
            if self.sequence == 0 {
                self.live = true;
                self.tx.send(ServerMessage::Live).unwrap();
            }
            if end {
                self.tx.send(ServerMessage::Off).unwrap();
            }
            self.sequence += 1;
        }
    }

    pub fn reset(&mut self) {
        self.sequence = 0;
        self.m3u8 = String::from("");
        self.ts.clear();
        self.timestamp = vec![0];
    }
}
