mod adts;
mod flv;
mod nalu;
mod ts;

use rml_rtmp::chunk_io::Packet;
use rml_rtmp::sessions::{ServerSession, ServerSessionConfig, ServerSessionEvent, ServerSessionResult};
use rml_rtmp::time::RtmpTimestamp;
use std::sync::{Arc, Mutex};
use std::{fs, thread};
use bytes::Bytes;
use ts::TransportStream;
use flv::{Flv, DataType};
use nalu::{Nalu, NaluConfig};
use adts::{Adts, AdtsConfig};
use super::PlayList;

pub enum ServerResult {
    Disconnect,
    Response { packet: Packet },
}

pub struct Server {
    flv: Flv,
    ts: TransportStream,
    video_config: NaluConfig,
    audio_config: AdtsConfig,
    has_keyframe: bool,
    session: Option<ServerSession>,
    playlist: Arc<Mutex<PlayList>>,
    next_write: u32,
}

impl Server {
    const WRITE_DURATION: u32 = 2000;

    pub fn new(playlist: Arc<Mutex<PlayList>>) -> Server {
        Server {
            flv: Flv::new(),
            ts: TransportStream::new(),
            video_config: NaluConfig::new(),
            audio_config: AdtsConfig::new(),
            has_keyframe: false,
            session: None,
            playlist,
            next_write: Server::WRITE_DURATION,
        }
    }

    pub fn handle_handshake_bytes(&mut self, bytes: &[u8]) -> Result<Vec<ServerResult>, String> {
        let mut server_results = Vec::new();
        let config = ServerSessionConfig::new();
        let (session, initial_results) = match ServerSession::new(config) {
            Ok(results) => results,
            Err(error) => return Err(error.to_string()),
        };

        self.session = Some(session);
        self.handle_session_results(initial_results, &mut server_results);
        match self.handle_bytes(bytes) {
            Ok(results) => server_results.extend(results),
            Err(error) => {
                println!("Handshake bytes the following server error: {}", error);
                return Err(error.to_string());
            }
        }
        Ok(server_results)
    }

    pub fn handle_bytes(&mut self, bytes: &[u8]) -> Result<Vec<ServerResult>, String> {
        let mut server_results = Vec::new();
        let session_results = match self.session.as_mut().unwrap().handle_input(bytes) {
            Ok(results) => results,
            Err(error) => return Err(error.to_string()),
        };

        self.handle_session_results(session_results, &mut server_results);
        Ok(server_results)
    }

    fn handle_session_results(&mut self, session_results: Vec<ServerSessionResult>, server_results: &mut Vec<ServerResult>) {
        for result in session_results {
            match result {
                ServerSessionResult::OutboundResponse(packet) => server_results.push(ServerResult::Response { packet }),
                ServerSessionResult::RaisedEvent(event) => self.handle_event(event, server_results),
                r => println!("Server result received: {:?}", r),
            }
        }
    }

    fn handle_event(&mut self, event: ServerSessionEvent, server_results: &mut Vec<ServerResult>) {
        match event {
            ServerSessionEvent::ConnectionRequested { request_id, app_name } => {
                self.handle_connection_requested(request_id, app_name, server_results);
            }
            ServerSessionEvent::PublishStreamRequested {
                request_id,
                app_name,
                stream_key,
                mode: _,
            } => {
                self.handle_publish_requested(request_id, app_name, stream_key, server_results);
            }
            ServerSessionEvent::VideoDataReceived {
                app_name: _,
                stream_key: _,
                data,
                timestamp,
            } => {
                self.handle_video(timestamp, data);
            }
            ServerSessionEvent::AudioDataReceived {
                app_name: _,
                stream_key: _,
                data,
                timestamp,
            } => {
                self.handle_audio(timestamp, data);
            }
            _ => (), // println!("Event raised {:?}", event),
        }
    }

    fn handle_connection_requested(&mut self, request_id: u32, app_name: String, server_results: &mut Vec<ServerResult>) {
        println!("Connection requested connection to app '{}'", app_name);

        let accept_result = self.session.as_mut().unwrap().accept_request(request_id);
        match accept_result {
            Ok(results) => self.handle_session_results(results, server_results),
            Err(error) => {
                println!("Error occurred accepting connection request: {:?}", error);
                server_results.push(ServerResult::Disconnect);
            }
        }
    }

    fn handle_publish_requested(&mut self, request_id: u32, app_name: String, stream_key: String, server_results: &mut Vec<ServerResult>) {
        println!("Publish requested on app '{}' and stream key '{}'", app_name, stream_key);
        // self.flv.init_file(String::from("./video.flv"));

        {
            let mut playlist = self.playlist.lock().unwrap();
            if playlist.live {
                server_results.push(ServerResult::Disconnect);
                return;
            }
            playlist.reset();
        }

        fs::remove_dir_all("./video").unwrap();
        fs::create_dir_all("./video").unwrap();

        let accept_result = self.session.as_mut().unwrap().accept_request(request_id);
        match accept_result {
            Ok(results) => self.handle_session_results(results, server_results),
            Err(error) => {
                println!("Error occurred accepting publish request: {:?}", error);
                server_results.push(ServerResult::Disconnect)
            }
        }
    }

    fn handle_video(&mut self, timestamp: RtmpTimestamp, data: Bytes) {
        let video = Flv::read_video(data.clone());
        if video.is_keyframe {
            self.has_keyframe = true;
        }
        if !(self.has_keyframe || video.is_sequence_header) {
            return;
        }
        // self.flv.push(DataType::Video, timestamp.value, video.is_keyframe, data.clone());

        if video.is_sequence_header {
            self.video_config.set(video.data.clone());
            return;
        }

        if video.is_keyframe {
            if timestamp.value > self.next_write {
                let mut playlist = self.playlist.lock().unwrap();
                let filename = format!("{}.ts", timestamp.value);
                self.ts.write_file(&filename);
                self.next_write = timestamp.value + Server::WRITE_DURATION;
                playlist.push(timestamp.value, filename, false);
            }
        }

        let nalu = Nalu::read(video.data, self.video_config.nalu_size);
        let es = Nalu::to_es_layer(&self.video_config, nalu);
        self.ts.push_video(timestamp.value as u64, video.composition_time, video.is_keyframe, es).unwrap();
    }

    fn handle_audio(&mut self, timestamp: RtmpTimestamp, data: Bytes) {
        let audio = Flv::read_audio(data.clone());
        if !(self.has_keyframe || audio.is_sequence_header) {
            return;
        }
        // self.flv.push(DataType::Audio, timestamp.value, false, data.clone());

        if audio.is_sequence_header {
            self.audio_config.set(audio.data.clone());
            return;
        }

        let es = Adts::to_es_layer(&self.audio_config, audio.data.to_vec());
        self.ts.push_audio(timestamp.value as u64, es);
    }

    pub fn end_stream(&mut self) {
        // self.flv.write_file();
        self.ts.write_file("0.ts");

        let duration = {
            let mut playlist = self.playlist.lock().unwrap();
            playlist.push(0, "0.ts".to_string(), true) * 1000 + 1000
        };

        let playlist = self.playlist.clone();
        thread::spawn(move || {
            let duration = std::time::Duration::from_millis(duration);
            std::thread::sleep(duration);
            playlist.lock().unwrap().live = false;
        });
    }
}
