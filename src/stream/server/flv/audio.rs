use bytes::{Bytes, Buf};

// Sound Format         | u4    10 = AAC
// Sound Rate           | u2    AAC: always 3
// Sound Size           | u1
// Sound Type           | u1    AAC: always 1
// AAC Packet Type      | u8    0 = sequence header
// Data                 | [u8]
pub struct FlvAudio {
    pub is_sequence_header: bool,
    pub data: Bytes,
}

impl FlvAudio {
    pub fn read(mut data: Bytes) -> FlvAudio {
        let header = data.get_u16();
        let is_sequence_header = (header & 0xff) == 0;

        FlvAudio { is_sequence_header, data }
    }
}
