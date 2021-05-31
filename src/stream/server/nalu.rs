use bytes::{Bytes, Buf};

// Flv Data - Video Sequence_Header
// ------------------------| ----
// Version                 |   u8
// Profile Indication      |   u8
// Profile Compatability   |   u8
// Level Indication        |   u8
// Reserved                |   u6
// NALU Length             |   u2
// Reserved                |   u3
// SPS Count               |   u5
// SPS Length              |   u16
// SPS                     |   u[]
// PPS Count               |   u8
// PPS Length              |   u16
// PPS                     |   u[]
pub struct NaluConfig {
    pub version: u8,
    pub profile_indication: u8,
    pub profile_compatability: u8,
    pub level_indication: u8,
    pub nalu_size: u8,
    pub sps: Vec<Nalu>,
    pub pps: Vec<Nalu>,
}

impl NaluConfig {
    pub fn new() -> NaluConfig {
        NaluConfig {
            version: 0,
            profile_indication: 0,
            profile_compatability: 0,
            level_indication: 0,
            nalu_size: 0,
            sps: Vec::new(),
            pps: Vec::new(),
        }
    }

    pub fn set(&mut self, mut data: Bytes) {
        self.version = data.get_u8();
        self.profile_indication = data.get_u8();
        self.profile_compatability = data.get_u8();
        self.level_indication = data.get_u8();
        self.nalu_size = (data.get_u8() & 0b11) + 1;

        let sps_count = data.get_u8() & 0b11111;
        let mut sps = Vec::new();
        for _ in 0..sps_count {
            let sps_length = data.get_u16() as usize;
            let sps_temp = data.slice(..sps_length);
            data.advance(sps_length);
            sps.push(Nalu::read_unit(sps_temp));
        }

        let pps_count = data.get_u8();
        let mut pps = Vec::new();
        for _ in 0..pps_count {
            let pps_length = data.get_u16() as usize;
            let pps_temp = data.slice(..pps_length);
            data.advance(pps_length);
            pps.push(Nalu::read_unit(pps_temp));
        }

        self.sps = sps;
        self.pps = pps;
    }
}

// FLV Data Body
// ----------| --
// Nalu Type | u8
// RBSP      | []

// FLV Data Body Nalu Type
// -----| ---|
// F	| u1 |	forbidden zero bit, h.264 必須為零
// NRI	| u2 |	nal ref idc, 值0~3, I幀/sps/pps為3, P幀為2, B幀為0
// Type	| u5 |	参考下表
// -----------
// 0	  未使用
// 1	  非關鍵幀
// 2	  片分區A
// 3	  片分區B
// 4	  片分區C
// 5	  關鍵幀
// 6	  補充增強訊息單元(SEI)
// 7	  SPS序列參數集
// 8	  PPS圖像參數集
// 9	  分解符
// 10	  序列结束
// 11	  碼流结束
// 12	  填充
// 13~23  保留
// 24~31  未使用

pub struct Nalu {
    pub ref_idc: u8,
    pub unit_type: u8,
    pub data: Bytes, // RBSP
}

impl Nalu {
    const INTER_DELIMITER: &'static [u8] = &[0x00, 0x00, 0x01];
    const BEGIN_DELIMITER: &'static [u8] = &[0x00, 0x00, 0x00, 0x01];
    const NALU_DELIMITER: &'static [u8] = &[0x00, 0x00, 0x00, 0x01, 0x09, 0x00];

    fn to_vec(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.data.len() + 1);

        let header = (self.ref_idc << 5) | (self.unit_type);
        v.push(header);
        v.extend(self.data.clone());
        v
    }

    pub fn read(mut data: Bytes, nalu_size: u8) -> Vec<Nalu> {
        let nalu_size = nalu_size as usize;
        let mut nal_units = Vec::new();

        while data.has_remaining() {
            let nalu_length = data.get_uint(nalu_size) as usize;
            let nalu_data = data.slice(..nalu_length);
            let nal_unit = Nalu::read_unit(nalu_data);
            data.advance(nalu_length);
            nal_units.push(nal_unit);
        }

        nal_units
    }

    pub fn read_unit(mut data: Bytes) -> Nalu {
        let nalu = data.get_u8();
        let ref_idc = (nalu >> 5) & 0x03;
        let unit_type = nalu & 0x1f;

        Nalu { ref_idc, unit_type, data }
    }

    // 轉成es時有nalu header, 固定爲0x00000001（幀開始）或0x000001（幀中）
    // 轉成es時, pes和es之間需加入type=9的nalu, 關鍵幀前必須加入type=7和type=8的nalu, 這些nalu彼此相鄰。
    // Pes Header | nalu(0x09) | 隨便(u8) | nalu(其他) | 內容 | nalu(0x67) | sps | nalu(0x68) | pps | nalu(0x65) | keyframe |
    // Pes Header | nalu(0x09) | 隨便(u8) | nalu(其他) | 內容 | nalu(0x41) | 內容 |
    pub fn to_es_layer(nalu_config: &NaluConfig, data: Vec<Nalu>) -> Vec<u8> {
        let mut es = Vec::new();
        let mut is_delimit = false;
        let mut is_keyframe_delimit = false;

        for nalu in data {
            match nalu.unit_type {
                1 | 6 => {
                    if !is_delimit {
                        es.extend(Nalu::NALU_DELIMITER);
                        is_delimit = true;
                    }
                }
                5 => {
                    if !is_delimit {
                        es.extend(Nalu::NALU_DELIMITER);
                        is_delimit = true;
                    }

                    if !is_keyframe_delimit {
                        let nalu = nalu_config.sps.first().unwrap();
                        let sps: Vec<u8> = nalu.to_vec();
                        es.extend(Nalu::BEGIN_DELIMITER);
                        es.extend(sps);

                        let nalu = nalu_config.pps.first().unwrap();
                        let pps: Vec<u8> = nalu.to_vec();
                        es.extend(Nalu::BEGIN_DELIMITER);
                        es.extend(pps);

                        is_keyframe_delimit = true;
                    }
                }
                _ => continue,
            }

            es.extend(Self::INTER_DELIMITER);
            es.extend(nalu.to_vec());
        }
        es
    }
}
