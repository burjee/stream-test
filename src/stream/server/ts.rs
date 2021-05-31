use std::fs::File;
use mpeg2ts;
use mpeg2ts::{
    ts::{TsPacket, TsHeader, TsPayload, Pid, ContinuityCounter},
    pes::PesHeader,
};

pub struct TransportStream {
    video_continuity_counter: ContinuityCounter,
    audio_continuity_counter: ContinuityCounter,
    packets: Vec<TsPacket>,
}

impl TransportStream {
    const PAT_PID: u16 = 0;
    const PMT_PID: u16 = 256;
    const VIDEO_PID: u16 = 257;
    const AUDIO_PID: u16 = 258;
    const VIDEO_STREAM_ID: u8 = 224;
    const AUDIO_STREAM_ID: u8 = 192;

    pub fn new() -> TransportStream {
        TransportStream {
            video_continuity_counter: ContinuityCounter::new(),
            audio_continuity_counter: ContinuityCounter::new(),
            packets: Vec::new(),
        }
    }

    pub fn write_file(&mut self, filename: &str) {
        use mpeg2ts::ts::{TsPacketWriter, WriteTsPacket};

        let filename = format!("./video/{}", filename);
        let file = File::create(filename).unwrap();
        let packets: Vec<_> = self.packets.drain(..).collect();
        let mut writer = TsPacketWriter::new(file);

        writer.write_ts_packet(&TransportStream::default_pat()).unwrap();
        writer.write_ts_packet(&TransportStream::default_pmt()).unwrap();

        for packet in &packets {
            writer.write_ts_packet(packet).unwrap();
        }
    }

    pub fn push_video(&mut self, timestamp: u64, composition_time: u64, is_keyframe: bool, mut video: Vec<u8>) -> Result<(), ()> {
        use mpeg2ts::{
            ts::{AdaptationField, payload},
            es::StreamId,
        };

        let mut header = TransportStream::default_header(TransportStream::VIDEO_PID);
        header.continuity_counter = self.video_continuity_counter;

        let packet = {
            let data = {
                let bytes: Vec<u8> = if video.len() < 153 { video.drain(..).collect() } else { video.drain(..153).collect() };
                mpeg2ts::ts::payload::Bytes::new(&bytes[..]).unwrap()
            };

            let pcr = mpeg2ts::time::ClockReference::new(timestamp * 90).unwrap();

            let adaptation_field = if is_keyframe {
                Some(AdaptationField {
                    discontinuity_indicator: false,
                    random_access_indicator: true,
                    es_priority_indicator: false,
                    pcr: Some(pcr),
                    opcr: None,
                    splice_countdown: None,
                    transport_private_data: Vec::new(),
                    extension: None,
                })
            } else {
                None
            };

            let pts = mpeg2ts::time::Timestamp::new((timestamp + composition_time) * 90).unwrap();
            let dts = mpeg2ts::time::Timestamp::new(timestamp * 90).unwrap();

            TsPacket {
                header: header.clone(),
                adaptation_field,
                payload: Some(TsPayload::Pes(payload::Pes {
                    header: PesHeader {
                        stream_id: StreamId::new(TransportStream::VIDEO_STREAM_ID),
                        priority: false,
                        data_alignment_indicator: false,
                        copyright: false,
                        original_or_copy: false,
                        pts: Some(pts),
                        dts: Some(dts),
                        escr: None,
                    },
                    pes_packet_len: 0,
                    data,
                })),
            }
        };

        self.packets.push(packet);
        header.continuity_counter.increment();

        while video.len() > 0 {
            let raw = {
                let bytes: Vec<u8> = if video.len() < payload::Bytes::MAX_SIZE { video.drain(..).collect() } else { video.drain(..payload::Bytes::MAX_SIZE).collect() };
                mpeg2ts::ts::payload::Bytes::new(&bytes[..]).unwrap()
            };

            let packet = TsPacket {
                header: header.clone(),
                adaptation_field: None,
                payload: Some(TsPayload::Raw(raw)),
            };

            self.packets.push(packet);
            header.continuity_counter.increment();
        }

        self.video_continuity_counter = header.continuity_counter;

        Ok(())
    }

    pub fn push_audio(&mut self, timestamp: u64, mut audio: Vec<u8>) {
        use mpeg2ts::{ts::payload, es::StreamId};

        let data = {
            let bytes: Vec<u8> = if audio.len() < 153 { audio.drain(..).collect() } else { audio.drain(..153).collect() };
            mpeg2ts::ts::payload::Bytes::new(&bytes[..]).unwrap()
        };

        let mut header = TransportStream::default_header(TransportStream::AUDIO_PID);
        header.continuity_counter = self.audio_continuity_counter;

        let packet = TsPacket {
            header: header.clone(),
            adaptation_field: None,
            payload: Some(TsPayload::Pes(payload::Pes {
                header: PesHeader {
                    stream_id: StreamId::new(TransportStream::AUDIO_STREAM_ID),
                    priority: false,
                    data_alignment_indicator: false,
                    copyright: false,
                    original_or_copy: false,
                    pts: Some(mpeg2ts::time::Timestamp::new(timestamp * 90).unwrap()),
                    dts: None,
                    escr: None,
                },
                pes_packet_len: 0,
                data,
            })),
        };

        self.packets.push(packet);
        header.continuity_counter.increment();

        while audio.len() > 0 {
            let raw = {
                let bytes: Vec<u8> = if audio.len() < payload::Bytes::MAX_SIZE { audio.drain(..).collect() } else { audio.drain(..payload::Bytes::MAX_SIZE).collect() };
                mpeg2ts::ts::payload::Bytes::new(&bytes[..]).unwrap()
            };

            let packet = TsPacket {
                header: header.clone(),
                adaptation_field: None,
                payload: Some(TsPayload::Raw(raw)),
            };

            self.packets.push(packet);
            header.continuity_counter.increment();
        }

        self.audio_continuity_counter = header.continuity_counter;
    }

    pub fn default_header(pid: u16) -> TsHeader {
        use mpeg2ts::ts::TransportScramblingControl;

        TsHeader {
            transport_error_indicator: false,
            transport_priority: false,
            pid: Pid::new(pid).unwrap(),
            transport_scrambling_control: TransportScramblingControl::NotScrambled,
            continuity_counter: ContinuityCounter::new(),
        }
    }

    pub fn default_pat() -> TsPacket {
        use mpeg2ts::ts::{VersionNumber, payload::Pat, ProgramAssociation};

        TsPacket {
            header: TransportStream::default_header(TransportStream::PAT_PID),
            adaptation_field: None,
            payload: Some(TsPayload::Pat(Pat {
                transport_stream_id: 1,
                version_number: VersionNumber::default(),
                table: vec![ProgramAssociation {
                    program_num: 1,
                    program_map_pid: Pid::new(TransportStream::PMT_PID).unwrap(),
                }],
            })),
        }
    }

    pub fn default_pmt() -> TsPacket {
        use mpeg2ts::{
            ts::{VersionNumber, payload::Pmt, EsInfo},
            es::StreamType,
        };

        TsPacket {
            header: TransportStream::default_header(TransportStream::PMT_PID),
            adaptation_field: None,
            payload: Some(TsPayload::Pmt(Pmt {
                program_num: 1,
                pcr_pid: Some(Pid::new(TransportStream::VIDEO_PID).unwrap()),
                version_number: VersionNumber::default(),
                table: vec![
                    EsInfo {
                        stream_type: StreamType::H264,
                        elementary_pid: Pid::new(TransportStream::VIDEO_PID).unwrap(),
                        descriptors: vec![],
                    },
                    EsInfo {
                        stream_type: StreamType::AdtsAac,
                        elementary_pid: Pid::new(TransportStream::AUDIO_PID).unwrap(),
                        descriptors: vec![],
                    },
                ],
            })),
        }
    }
}
