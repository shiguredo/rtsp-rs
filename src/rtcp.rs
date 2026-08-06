use crate::buf::{ByteSliceExt, VecExt};
use crate::error::Error;

/// RTCP パケット種別
pub const RTCP_PT_SR: u8 = 200;
pub const RTCP_PT_RR: u8 = 201;
pub const RTCP_PT_SDES: u8 = 202;
pub const RTCP_PT_BYE: u8 = 203;
pub const RTCP_PT_APP: u8 = 204;

/// RTCP パケット (RFC 3550)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtcpPacket {
    /// Sender Report (PT=200)
    SenderReport(RtcpSenderReport),
    /// Receiver Report (PT=201)
    ReceiverReport(RtcpReceiverReport),
    /// Source Description (PT=202)
    SourceDescription(RtcpSdes),
    /// Goodbye (PT=203)
    Bye(RtcpBye),
    /// Application-Defined (PT=204)
    App(RtcpApp),
    /// Unknown packet type
    Unknown { payload_type: u8, data: Vec<u8> },
}

/// Sender Report (SR)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpSenderReport {
    pub ssrc: u32,
    pub ntp_timestamp: u64,
    pub rtp_timestamp: u32,
    pub packet_count: u32,
    pub octet_count: u32,
    pub reports: Vec<RtcpReportBlock>,
}

/// Receiver Report (RR)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpReceiverReport {
    pub ssrc: u32,
    pub reports: Vec<RtcpReportBlock>,
}

/// Report Block (共通)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpReportBlock {
    pub ssrc: u32,
    pub fraction_lost: u8,
    pub cumulative_lost: u32, // 24 bits
    pub highest_seq: u32,
    pub jitter: u32,
    pub last_sr: u32,
    pub delay_since_sr: u32,
}

/// Source Description (SDES)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpSdes {
    pub chunks: Vec<RtcpSdesChunk>,
}

/// SDES Chunk
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpSdesChunk {
    pub ssrc: u32,
    pub items: Vec<RtcpSdesItem>,
}

/// SDES Item Type
pub const SDES_CNAME: u8 = 1;
pub const SDES_NAME: u8 = 2;
pub const SDES_EMAIL: u8 = 3;
pub const SDES_PHONE: u8 = 4;
pub const SDES_LOC: u8 = 5;
pub const SDES_TOOL: u8 = 6;
pub const SDES_NOTE: u8 = 7;
pub const SDES_PRIV: u8 = 8;

/// SDES Item
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtcpSdesItem {
    Cname(String),
    Name(String),
    Email(String),
    Phone(String),
    Loc(String),
    Tool(String),
    Note(String),
    Priv { prefix: String, value: Vec<u8> },
    Unknown { item_type: u8, data: Vec<u8> },
}

impl RtcpSdesItem {
    pub fn item_type(&self) -> u8 {
        match self {
            RtcpSdesItem::Cname(_) => SDES_CNAME,
            RtcpSdesItem::Name(_) => SDES_NAME,
            RtcpSdesItem::Email(_) => SDES_EMAIL,
            RtcpSdesItem::Phone(_) => SDES_PHONE,
            RtcpSdesItem::Loc(_) => SDES_LOC,
            RtcpSdesItem::Tool(_) => SDES_TOOL,
            RtcpSdesItem::Note(_) => SDES_NOTE,
            RtcpSdesItem::Priv { .. } => SDES_PRIV,
            RtcpSdesItem::Unknown { item_type, .. } => *item_type,
        }
    }
}

/// Goodbye (BYE)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpBye {
    pub ssrcs: Vec<u32>,
    pub reason: Option<String>,
}

/// Application-Defined (APP)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpApp {
    pub subtype: u8,
    pub ssrc: u32,
    pub name: [u8; 4],
    pub data: Vec<u8>,
}

impl RtcpPacket {
    /// 複数の RTCP パケットをパース (compound packet)
    pub fn parse(data: &[u8]) -> Result<Vec<Self>, Error> {
        let mut packets = Vec::new();
        let mut buf = data;

        while buf.len() >= 4 {
            let packet = Self::parse_one(&mut buf)?;
            packets.push(packet);
        }

        Ok(packets)
    }

    fn parse_one(buf: &mut &[u8]) -> Result<Self, Error> {
        if buf.len() < 4 {
            return Err(Error::insufficient_buffer());
        }

        // Common header
        let first = buf.read_u8()?;
        let version = (first >> 6) & 0x03;
        if version != 2 {
            return Err(Error::invalid_data(format!(
                "unsupported RTCP version: {}",
                version
            )));
        }
        let _padding = (first >> 5) & 0x01 == 1;
        let count = first & 0x1F;

        let payload_type = buf.read_u8()?;
        let length_words = buf.read_u16()? as usize;
        let length_bytes = length_words * 4;

        if buf.len() < length_bytes {
            return Err(Error::insufficient_buffer());
        }

        let packet_data = buf.read_bytes(length_bytes)?;
        let mut packet_buf: &[u8] = &packet_data;

        match payload_type {
            RTCP_PT_SR => {
                let ssrc = packet_buf.read_u32()?;
                let ntp_timestamp = packet_buf.read_u64()?;
                let rtp_timestamp = packet_buf.read_u32()?;
                let packet_count = packet_buf.read_u32()?;
                let octet_count = packet_buf.read_u32()?;

                // count は入力データ由来のため、容量を事前確保せず Vec::new() で安全に積む
                let mut reports = Vec::new();
                for _ in 0..count {
                    reports.push(Self::parse_report_block(&mut packet_buf)?);
                }

                Ok(RtcpPacket::SenderReport(RtcpSenderReport {
                    ssrc,
                    ntp_timestamp,
                    rtp_timestamp,
                    packet_count,
                    octet_count,
                    reports,
                }))
            }
            RTCP_PT_RR => {
                let ssrc = packet_buf.read_u32()?;

                // count は入力データ由来のため、容量を事前確保せず Vec::new() で安全に積む
                let mut reports = Vec::new();
                for _ in 0..count {
                    reports.push(Self::parse_report_block(&mut packet_buf)?);
                }

                Ok(RtcpPacket::ReceiverReport(RtcpReceiverReport {
                    ssrc,
                    reports,
                }))
            }
            RTCP_PT_SDES => {
                // count は入力データ由来のため、容量を事前確保せず Vec::new() で安全に積む
                let mut chunks = Vec::new();
                for _ in 0..count {
                    if packet_buf.len() < 4 {
                        break;
                    }
                    let chunk_start = packet_buf.len();
                    let ssrc = packet_buf.read_u32()?;
                    let mut items = Vec::new();

                    loop {
                        if packet_buf.is_empty() {
                            break;
                        }
                        let item_type = packet_buf.read_u8()?;
                        if item_type == 0 {
                            // Skip padding to 32-bit boundary
                            // Calculate how many bytes we've read in this chunk
                            let bytes_read = chunk_start - packet_buf.len();
                            let padding = (4 - (bytes_read % 4)) % 4;
                            for _ in 0..padding {
                                if !packet_buf.is_empty() {
                                    packet_buf.read_u8()?;
                                }
                            }
                            break;
                        }
                        let item_len = packet_buf.read_u8()? as usize;
                        if packet_buf.len() < item_len {
                            break;
                        }
                        let item_data = packet_buf.read_bytes(item_len)?;

                        let item = match item_type {
                            SDES_CNAME => {
                                RtcpSdesItem::Cname(String::from_utf8_lossy(&item_data).to_string())
                            }
                            SDES_NAME => {
                                RtcpSdesItem::Name(String::from_utf8_lossy(&item_data).to_string())
                            }
                            SDES_EMAIL => {
                                RtcpSdesItem::Email(String::from_utf8_lossy(&item_data).to_string())
                            }
                            SDES_PHONE => {
                                RtcpSdesItem::Phone(String::from_utf8_lossy(&item_data).to_string())
                            }
                            SDES_LOC => {
                                RtcpSdesItem::Loc(String::from_utf8_lossy(&item_data).to_string())
                            }
                            SDES_TOOL => {
                                RtcpSdesItem::Tool(String::from_utf8_lossy(&item_data).to_string())
                            }
                            SDES_NOTE => {
                                RtcpSdesItem::Note(String::from_utf8_lossy(&item_data).to_string())
                            }
                            SDES_PRIV => {
                                if !item_data.is_empty() {
                                    let prefix_len = item_data[0] as usize;
                                    if item_data.len() > prefix_len {
                                        RtcpSdesItem::Priv {
                                            prefix: String::from_utf8_lossy(
                                                &item_data[1..1 + prefix_len],
                                            )
                                            .to_string(),
                                            value: item_data[1 + prefix_len..].to_vec(),
                                        }
                                    } else {
                                        RtcpSdesItem::Unknown {
                                            item_type,
                                            data: item_data,
                                        }
                                    }
                                } else {
                                    RtcpSdesItem::Priv {
                                        prefix: String::new(),
                                        value: Vec::new(),
                                    }
                                }
                            }
                            _ => RtcpSdesItem::Unknown {
                                item_type,
                                data: item_data,
                            },
                        };
                        items.push(item);
                    }

                    chunks.push(RtcpSdesChunk { ssrc, items });
                }

                Ok(RtcpPacket::SourceDescription(RtcpSdes { chunks }))
            }
            RTCP_PT_BYE => {
                // count は入力データ由来のため、容量を事前確保せず Vec::new() で安全に積む
                let mut ssrcs = Vec::new();
                for _ in 0..count {
                    if packet_buf.len() < 4 {
                        break;
                    }
                    ssrcs.push(packet_buf.read_u32()?);
                }

                let reason = if !packet_buf.is_empty() {
                    let reason_len = packet_buf.read_u8()? as usize;
                    if packet_buf.len() >= reason_len {
                        Some(
                            String::from_utf8_lossy(&packet_buf.read_bytes(reason_len)?)
                                .to_string(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                };

                Ok(RtcpPacket::Bye(RtcpBye { ssrcs, reason }))
            }
            RTCP_PT_APP => {
                let ssrc = packet_buf.read_u32()?;
                let mut name = [0u8; 4];
                if packet_buf.len() >= 4 {
                    name.copy_from_slice(&packet_buf.read_bytes(4)?);
                }
                let data = packet_buf.to_vec();

                Ok(RtcpPacket::App(RtcpApp {
                    subtype: count,
                    ssrc,
                    name,
                    data,
                }))
            }
            _ => Ok(RtcpPacket::Unknown {
                payload_type,
                data: packet_data,
            }),
        }
    }

    fn parse_report_block(buf: &mut &[u8]) -> Result<RtcpReportBlock, Error> {
        let ssrc = buf.read_u32()?;
        let fraction_and_lost = buf.read_u32()?;
        let fraction_lost = (fraction_and_lost >> 24) as u8;
        let cumulative_lost = fraction_and_lost & 0x00FFFFFF;
        let highest_seq = buf.read_u32()?;
        let jitter = buf.read_u32()?;
        let last_sr = buf.read_u32()?;
        let delay_since_sr = buf.read_u32()?;

        Ok(RtcpReportBlock {
            ssrc,
            fraction_lost,
            cumulative_lost,
            highest_seq,
            jitter,
            last_sr,
            delay_since_sr,
        })
    }

    /// RTCP パケットをエンコード
    pub fn build(packets: &[Self]) -> Vec<u8> {
        let mut buf = Vec::new();
        for packet in packets {
            packet.build_one(&mut buf);
        }
        buf
    }

    fn build_one(&self, buf: &mut Vec<u8>) {
        match self {
            RtcpPacket::SenderReport(sr) => {
                Self::write_header(
                    buf,
                    RTCP_PT_SR,
                    sr.reports.len() as u8,
                    6 + sr.reports.len() * 6,
                );
                buf.write_u32(sr.ssrc);
                buf.write_u64(sr.ntp_timestamp);
                buf.write_u32(sr.rtp_timestamp);
                buf.write_u32(sr.packet_count);
                buf.write_u32(sr.octet_count);
                for report in &sr.reports {
                    Self::write_report_block(buf, report);
                }
            }
            RtcpPacket::ReceiverReport(rr) => {
                Self::write_header(
                    buf,
                    RTCP_PT_RR,
                    rr.reports.len() as u8,
                    1 + rr.reports.len() * 6,
                );
                buf.write_u32(rr.ssrc);
                for report in &rr.reports {
                    Self::write_report_block(buf, report);
                }
            }
            RtcpPacket::SourceDescription(sdes) => {
                let mut sdes_buf = Vec::new();
                for chunk in &sdes.chunks {
                    sdes_buf.write_u32(chunk.ssrc);
                    for item in &chunk.items {
                        sdes_buf.write_u8(item.item_type());
                        match item {
                            RtcpSdesItem::Cname(s)
                            | RtcpSdesItem::Name(s)
                            | RtcpSdesItem::Email(s)
                            | RtcpSdesItem::Phone(s)
                            | RtcpSdesItem::Loc(s)
                            | RtcpSdesItem::Tool(s)
                            | RtcpSdesItem::Note(s) => {
                                sdes_buf.write_u8(s.len() as u8);
                                sdes_buf.write_bytes(s.as_bytes());
                            }
                            RtcpSdesItem::Priv { prefix, value } => {
                                let total_len = 1 + prefix.len() + value.len();
                                sdes_buf.write_u8(total_len as u8);
                                sdes_buf.write_u8(prefix.len() as u8);
                                sdes_buf.write_bytes(prefix.as_bytes());
                                sdes_buf.write_bytes(value);
                            }
                            RtcpSdesItem::Unknown { data, .. } => {
                                sdes_buf.write_u8(data.len() as u8);
                                sdes_buf.write_bytes(data);
                            }
                        }
                    }
                    sdes_buf.write_u8(0); // End of items
                    // Pad to 32-bit boundary
                    while sdes_buf.len() % 4 != 0 {
                        sdes_buf.write_u8(0);
                    }
                }
                let length_words = sdes_buf.len() / 4;
                Self::write_header(buf, RTCP_PT_SDES, sdes.chunks.len() as u8, length_words);
                buf.write_bytes(&sdes_buf);
            }
            RtcpPacket::Bye(bye) => {
                let mut bye_buf = Vec::new();
                for ssrc in &bye.ssrcs {
                    bye_buf.write_u32(*ssrc);
                }
                if let Some(ref reason) = bye.reason {
                    bye_buf.write_u8(reason.len() as u8);
                    bye_buf.write_bytes(reason.as_bytes());
                    // Pad to 32-bit boundary
                    while (bye_buf.len()) % 4 != 0 {
                        bye_buf.write_u8(0);
                    }
                }
                let length_words = bye_buf.len() / 4;
                Self::write_header(buf, RTCP_PT_BYE, bye.ssrcs.len() as u8, length_words);
                buf.write_bytes(&bye_buf);
            }
            RtcpPacket::App(app) => {
                let total_len = 2 + app.data.len().div_ceil(4);
                Self::write_header(buf, RTCP_PT_APP, app.subtype, total_len);
                buf.write_u32(app.ssrc);
                buf.write_bytes(&app.name);
                buf.write_bytes(&app.data);
                // Pad to 32-bit boundary
                while !buf.len().is_multiple_of(4) {
                    buf.write_u8(0);
                }
            }
            RtcpPacket::Unknown { payload_type, data } => {
                let length_words = data.len() / 4;
                Self::write_header(buf, *payload_type, 0, length_words);
                buf.write_bytes(data);
            }
        }
    }

    fn write_header(buf: &mut Vec<u8>, payload_type: u8, count: u8, length_words: usize) {
        let first = (2 << 6) | (count & 0x1F);
        buf.write_u8(first);
        buf.write_u8(payload_type);
        buf.write_u16(length_words as u16);
    }

    fn write_report_block(buf: &mut Vec<u8>, report: &RtcpReportBlock) {
        buf.write_u32(report.ssrc);
        let fraction_and_lost =
            ((report.fraction_lost as u32) << 24) | (report.cumulative_lost & 0x00FFFFFF);
        buf.write_u32(fraction_and_lost);
        buf.write_u32(report.highest_seq);
        buf.write_u32(report.jitter);
        buf.write_u32(report.last_sr);
        buf.write_u32(report.delay_since_sr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtcp_sr_parse_and_build() {
        let sr = RtcpSenderReport {
            ssrc: 0x12345678,
            ntp_timestamp: 0x0102030405060708,
            rtp_timestamp: 12345,
            packet_count: 100,
            octet_count: 5000,
            reports: vec![],
        };

        let packets = vec![RtcpPacket::SenderReport(sr.clone())];
        let encoded = RtcpPacket::build(&packets);
        // ビルダーで生成したデータは必ずパースできる想定
        let decoded = RtcpPacket::parse(&encoded).expect("SR のパースに失敗しない想定");

        assert_eq!(decoded.len(), 1);
        if let RtcpPacket::SenderReport(decoded_sr) = &decoded[0] {
            assert_eq!(decoded_sr.ssrc, sr.ssrc);
            assert_eq!(decoded_sr.ntp_timestamp, sr.ntp_timestamp);
            assert_eq!(decoded_sr.rtp_timestamp, sr.rtp_timestamp);
            assert_eq!(decoded_sr.packet_count, sr.packet_count);
            assert_eq!(decoded_sr.octet_count, sr.octet_count);
        } else {
            panic!("expected SenderReport");
        }
    }

    #[test]
    fn test_rtcp_sdes_parse_and_build() {
        let sdes = RtcpSdes {
            chunks: vec![RtcpSdesChunk {
                ssrc: 0xDEADBEEF,
                items: vec![RtcpSdesItem::Cname("test@example.com".to_string())],
            }],
        };

        let packets = vec![RtcpPacket::SourceDescription(sdes)];
        let encoded = RtcpPacket::build(&packets);
        // ビルダーで生成したデータは必ずパースできる想定
        let decoded = RtcpPacket::parse(&encoded).expect("SDES のパースに失敗しない想定");

        assert_eq!(decoded.len(), 1);
        if let RtcpPacket::SourceDescription(decoded_sdes) = &decoded[0] {
            assert_eq!(decoded_sdes.chunks.len(), 1);
            assert_eq!(decoded_sdes.chunks[0].ssrc, 0xDEADBEEF);
            if let RtcpSdesItem::Cname(cname) = &decoded_sdes.chunks[0].items[0] {
                assert_eq!(cname, "test@example.com");
            } else {
                panic!("expected CNAME");
            }
        } else {
            panic!("expected SDES");
        }
    }

    #[test]
    fn test_rtcp_bye() {
        let bye = RtcpBye {
            ssrcs: vec![0x11111111, 0x22222222],
            reason: Some("Goodbye".to_string()),
        };

        let packets = vec![RtcpPacket::Bye(bye)];
        let encoded = RtcpPacket::build(&packets);
        // ビルダーで生成したデータは必ずパースできる想定
        let decoded = RtcpPacket::parse(&encoded).expect("BYE のパースに失敗しない想定");

        assert_eq!(decoded.len(), 1);
        if let RtcpPacket::Bye(decoded_bye) = &decoded[0] {
            assert_eq!(decoded_bye.ssrcs.len(), 2);
            assert_eq!(decoded_bye.ssrcs[0], 0x11111111);
            assert_eq!(decoded_bye.ssrcs[1], 0x22222222);
            assert_eq!(decoded_bye.reason, Some("Goodbye".to_string()));
        } else {
            panic!("expected BYE");
        }
    }
}
