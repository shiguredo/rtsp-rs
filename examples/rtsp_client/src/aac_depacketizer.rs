//! RFC 3640 AAC RTP デパケタイザー
//!
//! AAC-hbr (High Bitrate) モードの RTP パケットから AAC フレームを抽出する。
//! AU Header Section をパースして各 AU のサイズを取得し、
//! 対応するデータを切り出す。

/// 抽出された AAC フレーム
pub struct AacFrame {
    /// 生の AAC フレームデータ
    pub data: Vec<u8>,
    /// RTP タイムスタンプ (複数フレーム時の個別タイムスタンプ)
    #[allow(dead_code)]
    pub timestamp: u32,
}

/// RFC 3640 AAC RTP デパケタイザー
///
/// SDP fmtp パラメータから取得した `sizeLength` と `indexLength` を使用して
/// AU Header Section をパースする。
pub struct AacDepacketizer {
    /// AU-size フィールドのビット長 (SDP fmtp から取得、通常 13)
    size_length: u8,
    /// AU-Index フィールドのビット長 (通常 3)
    index_length: u8,
}

impl AacDepacketizer {
    pub fn new(size_length: u8, index_length: u8) -> Self {
        Self {
            size_length,
            index_length,
        }
    }

    /// RTP ペイロードから AAC フレームを抽出する
    ///
    /// RFC 3640 Section 3.2.1 に従い、AU Header Section をパースして
    /// 各 AU のサイズを決定し、AU Data Section からフレームデータを切り出す。
    pub fn push(&mut self, payload: &[u8], timestamp: u32) -> Vec<AacFrame> {
        let mut frames = Vec::new();

        if payload.len() < 2 {
            return frames;
        }

        // AU Header Section (RFC 3640 Section 3.2.1)
        // 最初の 2 バイトは AU-headers-length (ビット単位)
        let au_headers_length_bits = u16::from_be_bytes([payload[0], payload[1]]) as usize;
        let au_header_bits = self.size_length as usize + self.index_length as usize;
        if au_header_bits == 0 {
            return frames;
        }

        let au_count = au_headers_length_bits / au_header_bits;
        // AU Header Section のバイトサイズ (2 バイトの長さフィールドを含む)
        let au_headers_bytes = 2 + au_headers_length_bits.div_ceil(8);

        if au_headers_bytes > payload.len() {
            return frames;
        }

        // 各 AU Header をパースして AU サイズを取得する
        let mut au_sizes = Vec::with_capacity(au_count);
        let header_data = &payload[2..au_headers_bytes];
        let mut bit_offset: usize = 0;

        for _ in 0..au_count {
            let au_size = read_bits(header_data, bit_offset, self.size_length as usize);
            bit_offset += self.size_length as usize;
            // AU-Index (最初の AU) または AU-Index-delta (後続の AU) をスキップ
            bit_offset += self.index_length as usize;
            au_sizes.push(au_size as usize);
        }

        // AU Data Section からフレームデータを切り出す
        let mut data_offset = au_headers_bytes;
        // AAC の 1 フレームあたりのサンプル数 (AAC-LC: 1024)
        let samples_per_frame: u32 = 1024;

        for (i, &au_size) in au_sizes.iter().enumerate() {
            if data_offset + au_size > payload.len() {
                break;
            }

            let frame_data = payload[data_offset..data_offset + au_size].to_vec();
            let frame_timestamp = timestamp + (i as u32) * samples_per_frame;

            frames.push(AacFrame {
                data: frame_data,
                timestamp: frame_timestamp,
            });

            data_offset += au_size;
        }

        frames
    }
}

/// ビットストリームから指定位置・指定ビット数の値を読み取る
fn read_bits(data: &[u8], bit_offset: usize, num_bits: usize) -> u32 {
    let mut value: u32 = 0;
    for i in 0..num_bits {
        let byte_index = (bit_offset + i) / 8;
        let bit_index = 7 - ((bit_offset + i) % 8);
        if byte_index < data.len() {
            value = (value << 1) | ((data[byte_index] >> bit_index) & 1) as u32;
        } else {
            value <<= 1;
        }
    }
    value
}
