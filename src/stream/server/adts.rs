use bytes::{Bytes, Buf};

// Flv Data - Audio Sequence_Header
// ------------------------| ----
// Object Type             | u5
// Frequency Index         | u4
// Channel Configuration   | u4
// AOT Specific Config
// Frame Length Flag       | u1
// Depends On Core Coder   | u1
// Extension Flag          | u1
pub struct AdtsConfig {
    pub object_type: u8,
    pub sampling_frequency_index: u8,
    pub channel_configuration: u8,
}

impl AdtsConfig {
    pub fn new() -> AdtsConfig {
        AdtsConfig {
            object_type: 0,
            sampling_frequency_index: 0,
            channel_configuration: 0,
        }
    }

    pub fn set(&mut self, mut data: Bytes) {
        let byte0 = data.get_u8();
        let byte1 = data.get_u8();

        self.object_type = (byte0 & 0xF8) >> 3;
        self.sampling_frequency_index = ((byte0 & 0x07) << 1) | (byte1 >> 7);
        self.channel_configuration = (byte1 >> 3) & 0x0F;
    }
}

pub struct Adts {}

impl Adts {
    const SYNCWORD: &'static [u8] = &[0xff, 0xf1];

    // Syncword         	                u12     固定爲0xfff
    // Id               	                u1      0為MPEG-4, 1為MPEG-2
    // Layer 	                            u2      固定爲00
    // Protection Absent 	                u1      固定爲1
    // Profile 	                            u2      值: 0~3, 1為aac
    // Sampling Frequency Index 	        u4      表示採樣率, 0: 96000 Hz, 1: 88200 Hz, 2: 64000 Hz, 3：48000 Hz, 4: 44100 Hz, 5: 32000 Hz, 6: 24000 Hz, 7: 22050 Hz, 8: 16000 Hz, 9: 12000 Hz, 10: 11025 Hz, 11: 8000 Hz, 12: 7350 Hz
    // Private Bit 	                        u1      固定爲0
    // Channel Configuration 	            u3      值: 0~7, 1: 1 channel: front-center, 2: 2 channels: front-left, front-right, 3: 3 channels: front-center, front-left, front-right, 4: 4 channels: front-center, front-left, front-right, back-center
    // Original Copy 	                    u1      固定爲0
    // Home 	                            u1      固定爲0
    // Copyright Identification Bit 	    u1      固定爲0
    // Copyright Identification Start 	    u1      固定爲0
    // Aac Frame Length 	                u13     含adts header在內的數據總長度
    // Adts Buffer Fullness 	            u11     固定爲0x7ff
    // Number Of Raw Data Blocks In Frame 	u2      固定爲00
    pub fn to_es_layer(adts_config: &AdtsConfig, data: Vec<u8>) -> Vec<u8> {
        let mut es = Vec::with_capacity(7 + data.len());

        es.extend(Adts::SYNCWORD);

        let profile = 0x40;
        let sampling_frequency_index = adts_config.sampling_frequency_index << 2;
        let channel_configuration0 = (adts_config.channel_configuration & 0x07) >> 2;
        es.push(profile | sampling_frequency_index | channel_configuration0);

        let channel_configuration1 = (adts_config.channel_configuration & 0x03) << 6;
        let frame_length = (7 + data.len()) as u16;
        let frame_length0 = ((frame_length & 0x1FFF) >> 11) as u8;
        es.push(channel_configuration1 | frame_length0);

        let frame_length1 = ((frame_length & 0x7FF) << 5) as u16;
        let frame_length2 = frame_length1 | 0b0000_0000_0001_1111;
        es.extend(&[(frame_length2 >> 8) as u8, (frame_length2 & 0xff) as u8]);

        es.push(0b1111_1100);
        es.extend(data);

        es
    }
}
