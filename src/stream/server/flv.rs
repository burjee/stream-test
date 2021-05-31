mod audio;
mod video;

use std::io::prelude::*;
use std::io::BufWriter;
use std::fs::OpenOptions;
use bytes::Bytes;
use video::FlvVideo;
use audio::FlvAudio;

pub enum DataType {
    Video,
    Audio,
}

// https://www.adobe.com/content/dam/acom/en/devnet/flv/video_file_format_spec_v10.pdf
pub struct Flv {
    bytes: Vec<u8>,
    file_path: String,
}

impl Flv {
    const HEADER: &'static [u8] = b"FLV\x01\x05\x00\x00\x00\x09";

    pub fn read_video(data: Bytes) -> FlvVideo {
        FlvVideo::read(data)
    }

    pub fn read_audio(data: Bytes) -> FlvAudio {
        FlvAudio::read(data)
    }

    pub fn new() -> Flv {
        Flv { bytes: vec![], file_path: String::from("") }
    }

    pub fn init_file(&mut self, file_path: String) {
        self.file_path = file_path;
        self.bytes.extend(Flv::HEADER);
        self.bytes.extend(b"\x00\x00\x00\x00"); // pre_tag_size
    }

    pub fn push(&mut self, data_type: DataType, timestamp: u32, is_keyframe: bool, data: Bytes) {
        if is_keyframe {
            self.write_file();
        }

        let data_type = get_data_type(data_type);
        let data_len = data.len();
        let len_byte0 = (data_len >> 16) as u8;
        let len_byte1 = ((data_len >> 8) & 0xff) as u8;
        let len_byte2 = (data_len & 0xff) as u8;

        let time_byte0 = (timestamp >> 16) as u8;
        let time_byte1 = ((timestamp >> 8) & 0xff) as u8;
        let time_byte2 = (timestamp & 0xff) as u8;

        let tag = vec![data_type, len_byte0, len_byte1, len_byte2, time_byte0, time_byte1, time_byte2, 0, 0, 0, 0];
        let pre_tag_size = tag.len() + data.len();
        let tag_size_byte0 = (pre_tag_size >> 24) as u8;
        let tag_size_byte1 = ((pre_tag_size >> 16) & 0xff) as u8;
        let tag_size_byte2 = ((pre_tag_size >> 8) & 0xff) as u8;
        let tag_size_byte3 = (pre_tag_size & 0xff) as u8;

        let pre_tag_size = vec![tag_size_byte0, tag_size_byte1, tag_size_byte2, tag_size_byte3];

        self.bytes.extend(&tag[..]);
        self.bytes.extend(&data[..]);
        self.bytes.extend(&pre_tag_size[..]);
    }

    pub fn write_file(&mut self) {
        let file = OpenOptions::new().create(true).write(true).append(true).open(&self.file_path).unwrap();
        let mut buf = BufWriter::new(file);

        buf.write_all(&self.bytes[..]).unwrap();
        buf.flush().unwrap();
        self.bytes.clear();
    }
}

fn get_data_type(data_type: DataType) -> u8 {
    match data_type {
        DataType::Video => 0x09,
        DataType::Audio => 0x08,
    }
}

// --------------------
// Flv File:
// --------------------
// Flv Header
// Previous Tag Size 0
// Tag 1
// Previous Tag Size 1
// ...
// Tag N
// Previous Tag Size N

// ------------------------
// Flv Tag
// ------------------| --- |
// Tag Type          | u8  | 0x08=audio 0x09=video
// Data Size         | u24 | Data欄位的長度
// Timestamp         | u24 | 時間戳記(毫秒) 第一個tag的相對值
// TimestampExtended | u8  | 時間戳記延伸 使時間戳記欄位變為U32 為第一個Byte
// StreamID          | u24 | 始終為 0
// Data              | []  | 資料內容
