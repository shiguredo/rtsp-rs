use crate::buf::{ByteSliceExt, VecExt};
use crate::error::Error;

/// RTP パケットヘッダー (RFC 3550)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpHeader {
    /// バージョン (2 bits, 常に 2)
    pub version: u8,
    /// パディング (1 bit)
    pub padding: bool,
    /// 拡張 (1 bit)
    pub extension: bool,
    /// CSRC カウント (4 bits)
    pub csrc_count: u8,
    /// マーカー (1 bit)
    pub marker: bool,
    /// ペイロードタイプ (7 bits)
    pub payload_type: u8,
    /// シーケンス番号 (16 bits)
    pub sequence_number: u16,
    /// タイムスタンプ (32 bits)
    pub timestamp: u32,
    /// SSRC (32 bits)
    pub ssrc: u32,
    /// CSRC リスト (0-15 items)
    pub csrc: Vec<u32>,
}

impl Default for RtpHeader {
    fn default() -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type: 0,
            sequence_number: 0,
            timestamp: 0,
            ssrc: 0,
            csrc: Vec::new(),
        }
    }
}

impl RtpHeader {
    pub fn new(payload_type: u8, sequence_number: u16, timestamp: u32, ssrc: u32) -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            csrc: Vec::new(),
        }
    }

    /// ヘッダーサイズを取得 (バイト)
    pub fn size(&self) -> usize {
        12 + self.csrc.len() * 4
    }
}

/// RTP 拡張ヘッダー
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpExtension {
    /// プロファイル固有識別子 (16 bits)
    pub profile: u16,
    /// 拡張データ
    pub data: Vec<u8>,
}

/// RTP パケット
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpPacket {
    pub header: RtpHeader,
    pub extension: Option<RtpExtension>,
    pub payload: Vec<u8>,
    pub padding_size: u8,
}

impl RtpPacket {
    pub fn new(header: RtpHeader, payload: Vec<u8>) -> Self {
        Self {
            header,
            extension: None,
            payload,
            padding_size: 0,
        }
    }

    /// バイト列からパース
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 12 {
            return Err(Error::insufficient_buffer());
        }

        let mut buf = data;

        // First byte: V(2) P(1) X(1) CC(4)
        let first = buf.read_u8()?;
        let version = (first >> 6) & 0x03;
        if version != 2 {
            return Err(Error::invalid_data(format!(
                "unsupported RTP version: {}",
                version
            )));
        }
        let padding = (first >> 5) & 0x01 == 1;
        let extension = (first >> 4) & 0x01 == 1;
        let csrc_count = first & 0x0F;

        // Second byte: M(1) PT(7)
        let second = buf.read_u8()?;
        let marker = (second >> 7) & 0x01 == 1;
        let payload_type = second & 0x7F;

        let sequence_number = buf.read_u16()?;
        let timestamp = buf.read_u32()?;
        let ssrc = buf.read_u32()?;

        // CSRC list
        // csrc_count は入力データ由来のため、容量を事前確保せず Vec::new() で安全に積む
        let mut csrc = Vec::new();
        for _ in 0..csrc_count {
            csrc.push(buf.read_u32()?);
        }

        // Extension header
        let ext = if extension {
            if buf.len() < 4 {
                return Err(Error::insufficient_buffer());
            }
            let profile = buf.read_u16()?;
            let length = buf.read_u16()? as usize * 4;
            if buf.len() < length {
                return Err(Error::insufficient_buffer());
            }
            let ext_data = buf.read_bytes(length)?;
            Some(RtpExtension {
                profile,
                data: ext_data,
            })
        } else {
            None
        };

        // Padding
        let padding_size = if padding {
            if buf.is_empty() {
                return Err(Error::invalid_data("padding indicated but no data"));
            }
            buf[buf.len() - 1]
        } else {
            0
        };

        // Payload (excluding padding)
        let payload_len = if padding {
            buf.len().saturating_sub(padding_size as usize)
        } else {
            buf.len()
        };
        let payload = buf[..payload_len].to_vec();

        Ok(Self {
            header: RtpHeader {
                version,
                padding,
                extension,
                csrc_count,
                marker,
                payload_type,
                sequence_number,
                timestamp,
                ssrc,
                csrc,
            },
            extension: ext,
            payload,
            padding_size,
        })
    }

    /// バイト列にエンコード
    pub fn build(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // First byte: V(2) P(1) X(1) CC(4)
        let first = ((self.header.version & 0x03) << 6)
            | (u8::from(self.header.padding || self.padding_size > 0) << 5)
            | (u8::from(self.extension.is_some()) << 4)
            | (self.header.csrc.len() as u8 & 0x0F);
        buf.write_u8(first);

        // Second byte: M(1) PT(7)
        let second = (u8::from(self.header.marker) << 7) | (self.header.payload_type & 0x7F);
        buf.write_u8(second);

        buf.write_u16(self.header.sequence_number);
        buf.write_u32(self.header.timestamp);
        buf.write_u32(self.header.ssrc);

        // CSRC list
        for csrc in &self.header.csrc {
            buf.write_u32(*csrc);
        }

        // Extension header
        if let Some(ref ext) = self.extension {
            buf.write_u16(ext.profile);
            let length_words = ext.data.len().div_ceil(4);
            buf.write_u16(length_words as u16);
            buf.write_bytes(&ext.data);
            // Pad to 32-bit boundary
            let padding_needed = length_words * 4 - ext.data.len();
            for _ in 0..padding_needed {
                buf.write_u8(0);
            }
        }

        // Payload
        buf.write_bytes(&self.payload);

        // Padding
        if self.padding_size > 0 {
            for _ in 0..(self.padding_size - 1) {
                buf.write_u8(0);
            }
            buf.write_u8(self.padding_size);
        }

        buf
    }

    /// パケット全体のサイズ
    pub fn size(&self) -> usize {
        let mut size = self.header.size();
        if let Some(ref ext) = self.extension {
            size += 4 + ext.data.len().div_ceil(4) * 4;
        }
        size += self.payload.len();
        size += self.padding_size as usize;
        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtp_parse_and_build() {
        let header = RtpHeader::new(96, 1234, 90000, 0x12345678);
        let packet = RtpPacket::new(header, vec![0x01, 0x02, 0x03, 0x04]);

        let encoded = packet.build();
        // ビルダーで生成したデータは必ずパースできる想定
        let decoded = RtpPacket::parse(&encoded).expect("RTP のパースに失敗しない想定");

        assert_eq!(decoded.header.version, 2);
        assert_eq!(decoded.header.payload_type, 96);
        assert_eq!(decoded.header.sequence_number, 1234);
        assert_eq!(decoded.header.timestamp, 90000);
        assert_eq!(decoded.header.ssrc, 0x12345678);
        assert_eq!(decoded.payload, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_rtp_with_marker() {
        let mut header = RtpHeader::new(96, 100, 12345, 0xDEADBEEF);
        header.marker = true;
        let packet = RtpPacket::new(header, vec![0xFF]);

        let encoded = packet.build();
        // ビルダーで生成したデータは必ずパースできる想定
        let decoded = RtpPacket::parse(&encoded).expect("RTP のパースに失敗しない想定");

        assert!(decoded.header.marker);
    }

    #[test]
    fn test_rtp_with_csrc() {
        let mut header = RtpHeader::new(96, 100, 12345, 0xDEADBEEF);
        header.csrc = vec![0x11111111, 0x22222222];
        let packet = RtpPacket::new(header, vec![0xAA]);

        let encoded = packet.build();
        // ビルダーで生成したデータは必ずパースできる想定
        let decoded = RtpPacket::parse(&encoded).expect("RTP のパースに失敗しない想定");

        assert_eq!(decoded.header.csrc.len(), 2);
        assert_eq!(decoded.header.csrc[0], 0x11111111);
        assert_eq!(decoded.header.csrc[1], 0x22222222);
    }
}
