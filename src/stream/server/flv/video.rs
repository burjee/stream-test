use bytes::{Bytes, Buf};

// FLV Data - Normal
// Field                | Type
// -------------------- | ---
// Frame Type           | u4
// Codec ID             | u4
// AVC Packet Type      | u8
// Composition Time     | i24
// Body                 | [u8]
pub struct FlvVideo {
    pub is_keyframe: bool,
    pub is_sequence_header: bool,
    pub composition_time: u64,
    pub data: Bytes,
}

impl FlvVideo {
    pub fn read(mut data: Bytes) -> FlvVideo {
        let byte0 = data.get_u8();
        let byte1 = data.get_u8();

        let is_keyframe = (byte0 >> 4) == 1;
        let is_sequence_header = byte1 == 0;
        let composition_time = data.get_uint(3);

        FlvVideo {
            is_keyframe,
            is_sequence_header,
            composition_time,
            data,
        }
    }
}
