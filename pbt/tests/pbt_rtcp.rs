use proptest::prelude::*;
use shiguredo_rtsp::rtcp::{
    RtcpApp, RtcpBye, RtcpPacket, RtcpReceiverReport, RtcpReportBlock, RtcpSdes, RtcpSdesChunk,
    RtcpSdesItem, RtcpSenderReport,
};

/// 有効な CNAME 文字列を生成
fn valid_cname() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9@._-]{1,50}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// 有効な理由文字列を生成
fn valid_reason() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 ._-]{0,50}").unwrap()
}

/// レポートブロックを生成
fn valid_report_block() -> impl Strategy<Value = RtcpReportBlock> {
    (
        any::<u32>(),     // ssrc
        any::<u8>(),      // fraction_lost
        0..0x00FFFFFFu32, // cumulative_lost (24 bits)
        any::<u32>(),     // highest_seq
        any::<u32>(),     // jitter
        any::<u32>(),     // last_sr
        any::<u32>(),     // delay_since_sr
    )
        .prop_map(
            |(
                ssrc,
                fraction_lost,
                cumulative_lost,
                highest_seq,
                jitter,
                last_sr,
                delay_since_sr,
            )| {
                RtcpReportBlock {
                    ssrc,
                    fraction_lost,
                    cumulative_lost,
                    highest_seq,
                    jitter,
                    last_sr,
                    delay_since_sr,
                }
            },
        )
}

/// Sender Report を生成
fn valid_sender_report() -> impl Strategy<Value = RtcpSenderReport> {
    (
        any::<u32>(),                                      // ssrc
        any::<u64>(),                                      // ntp_timestamp
        any::<u32>(),                                      // rtp_timestamp
        any::<u32>(),                                      // packet_count
        any::<u32>(),                                      // octet_count
        prop::collection::vec(valid_report_block(), 0..5), // reports (max 31, but limit for testing)
    )
        .prop_map(
            |(ssrc, ntp_timestamp, rtp_timestamp, packet_count, octet_count, reports)| {
                RtcpSenderReport {
                    ssrc,
                    ntp_timestamp,
                    rtp_timestamp,
                    packet_count,
                    octet_count,
                    reports,
                }
            },
        )
}

/// Receiver Report を生成
fn valid_receiver_report() -> impl Strategy<Value = RtcpReceiverReport> {
    (
        any::<u32>(),
        prop::collection::vec(valid_report_block(), 0..5),
    )
        .prop_map(|(ssrc, reports)| RtcpReceiverReport { ssrc, reports })
}

/// SDES Item を生成
fn valid_sdes_item() -> impl Strategy<Value = RtcpSdesItem> {
    prop_oneof![
        valid_cname().prop_map(RtcpSdesItem::Cname),
        valid_cname().prop_map(RtcpSdesItem::Name),
        valid_cname().prop_map(RtcpSdesItem::Email),
        valid_cname().prop_map(RtcpSdesItem::Tool),
    ]
}

/// SDES Chunk を生成
fn valid_sdes_chunk() -> impl Strategy<Value = RtcpSdesChunk> {
    (any::<u32>(), prop::collection::vec(valid_sdes_item(), 1..4))
        .prop_map(|(ssrc, items)| RtcpSdesChunk { ssrc, items })
}

/// SDES を生成
fn valid_sdes() -> impl Strategy<Value = RtcpSdes> {
    prop::collection::vec(valid_sdes_chunk(), 1..4).prop_map(|chunks| RtcpSdes { chunks })
}

/// BYE を生成
fn valid_bye() -> impl Strategy<Value = RtcpBye> {
    (
        prop::collection::vec(any::<u32>(), 1..5),
        prop::option::of(valid_reason()),
    )
        .prop_map(|(ssrcs, reason)| RtcpBye { ssrcs, reason })
}

/// APP を生成
fn valid_app() -> impl Strategy<Value = RtcpApp> {
    (
        0..32u8, // subtype (5 bits)
        any::<u32>(),
        prop::collection::vec(any::<u8>(), 4).prop_map(|v| {
            let mut name = [0u8; 4];
            name.copy_from_slice(&v);
            name
        }),
        prop::collection::vec(any::<u8>(), 0..64).prop_map(|mut data| {
            // 4バイト境界に揃える
            while data.len() % 4 != 0 {
                data.push(0);
            }
            data
        }),
    )
        .prop_map(|(subtype, ssrc, name, data)| RtcpApp {
            subtype,
            ssrc,
            name,
            data,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Sender Report の build/parse ラウンドトリップ
    #[test]
    fn test_rtcp_sender_report_roundtrip(sr in valid_sender_report()) {
        let packets = vec![RtcpPacket::SenderReport(sr.clone())];
        let encoded = RtcpPacket::build(&packets);
        let decoded = RtcpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.len(), 1);
        if let RtcpPacket::SenderReport(decoded_sr) = &decoded[0] {
            prop_assert_eq!(decoded_sr.ssrc, sr.ssrc);
            prop_assert_eq!(decoded_sr.ntp_timestamp, sr.ntp_timestamp);
            prop_assert_eq!(decoded_sr.rtp_timestamp, sr.rtp_timestamp);
            prop_assert_eq!(decoded_sr.packet_count, sr.packet_count);
            prop_assert_eq!(decoded_sr.octet_count, sr.octet_count);
            prop_assert_eq!(decoded_sr.reports.len(), sr.reports.len());
        } else {
            prop_assert!(false, "expected SenderReport");
        }
    }

    /// Receiver Report の build/parse ラウンドトリップ
    #[test]
    fn test_rtcp_receiver_report_roundtrip(rr in valid_receiver_report()) {
        let packets = vec![RtcpPacket::ReceiverReport(rr.clone())];
        let encoded = RtcpPacket::build(&packets);
        let decoded = RtcpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.len(), 1);
        if let RtcpPacket::ReceiverReport(decoded_rr) = &decoded[0] {
            prop_assert_eq!(decoded_rr.ssrc, rr.ssrc);
            prop_assert_eq!(decoded_rr.reports.len(), rr.reports.len());
        } else {
            prop_assert!(false, "expected ReceiverReport");
        }
    }

    /// SDES の build/parse ラウンドトリップ
    #[test]
    fn test_rtcp_sdes_roundtrip(sdes in valid_sdes()) {
        let packets = vec![RtcpPacket::SourceDescription(sdes.clone())];
        let encoded = RtcpPacket::build(&packets);
        let decoded = RtcpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.len(), 1);
        if let RtcpPacket::SourceDescription(decoded_sdes) = &decoded[0] {
            prop_assert_eq!(decoded_sdes.chunks.len(), sdes.chunks.len());
            for (orig_chunk, dec_chunk) in sdes.chunks.iter().zip(decoded_sdes.chunks.iter()) {
                prop_assert_eq!(dec_chunk.ssrc, orig_chunk.ssrc);
                prop_assert_eq!(dec_chunk.items.len(), orig_chunk.items.len());
            }
        } else {
            prop_assert!(false, "expected SDES");
        }
    }

    /// BYE の build/parse ラウンドトリップ
    #[test]
    fn test_rtcp_bye_roundtrip(bye in valid_bye()) {
        let packets = vec![RtcpPacket::Bye(bye.clone())];
        let encoded = RtcpPacket::build(&packets);
        let decoded = RtcpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.len(), 1);
        if let RtcpPacket::Bye(decoded_bye) = &decoded[0] {
            prop_assert_eq!(&decoded_bye.ssrcs, &bye.ssrcs);
            prop_assert_eq!(&decoded_bye.reason, &bye.reason);
        } else {
            prop_assert!(false, "expected BYE");
        }
    }

    /// APP の build/parse ラウンドトリップ
    #[test]
    fn test_rtcp_app_roundtrip(app in valid_app()) {
        let packets = vec![RtcpPacket::App(app.clone())];
        let encoded = RtcpPacket::build(&packets);
        let decoded = RtcpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.len(), 1);
        if let RtcpPacket::App(decoded_app) = &decoded[0] {
            prop_assert_eq!(decoded_app.subtype, app.subtype);
            prop_assert_eq!(decoded_app.ssrc, app.ssrc);
            prop_assert_eq!(decoded_app.name, app.name);
            // データは4バイト境界に揃えられるので、最低でも元のデータを含む
            prop_assert!(decoded_app.data.len() >= app.data.len());
        } else {
            prop_assert!(false, "expected APP");
        }
    }

    /// Compound packet の build/parse ラウンドトリップ
    #[test]
    fn test_rtcp_compound_packet_roundtrip(
        sr in valid_sender_report(),
        sdes in valid_sdes(),
    ) {
        let packets = vec![
            RtcpPacket::SenderReport(sr.clone()),
            RtcpPacket::SourceDescription(sdes.clone()),
        ];
        let encoded = RtcpPacket::build(&packets);
        let decoded = RtcpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.len(), 2);

        if let RtcpPacket::SenderReport(decoded_sr) = &decoded[0] {
            prop_assert_eq!(decoded_sr.ssrc, sr.ssrc);
        } else {
            prop_assert!(false, "expected SenderReport");
        }

        if let RtcpPacket::SourceDescription(decoded_sdes) = &decoded[1] {
            prop_assert_eq!(decoded_sdes.chunks.len(), sdes.chunks.len());
        } else {
            prop_assert!(false, "expected SDES");
        }
    }

    /// Report Block の検証
    #[test]
    fn test_rtcp_report_block_values(report in valid_report_block()) {
        let sr = RtcpSenderReport {
            ssrc: 0x12345678,
            ntp_timestamp: 0,
            rtp_timestamp: 0,
            packet_count: 0,
            octet_count: 0,
            reports: vec![report.clone()],
        };

        let packets = vec![RtcpPacket::SenderReport(sr)];
        let encoded = RtcpPacket::build(&packets);
        let decoded = RtcpPacket::parse(&encoded).unwrap();

        if let RtcpPacket::SenderReport(decoded_sr) = &decoded[0] {
            let decoded_report = &decoded_sr.reports[0];
            prop_assert_eq!(decoded_report.ssrc, report.ssrc);
            prop_assert_eq!(decoded_report.fraction_lost, report.fraction_lost);
            prop_assert_eq!(decoded_report.cumulative_lost, report.cumulative_lost);
            prop_assert_eq!(decoded_report.highest_seq, report.highest_seq);
            prop_assert_eq!(decoded_report.jitter, report.jitter);
            prop_assert_eq!(decoded_report.last_sr, report.last_sr);
            prop_assert_eq!(decoded_report.delay_since_sr, report.delay_since_sr);
        } else {
            prop_assert!(false, "expected SenderReport");
        }
    }
}

/// 不正なデータのパースが失敗することを確認
#[test]
fn test_rtcp_parse_invalid_data() {
    // データが短すぎる
    assert!(RtcpPacket::parse(&[]).unwrap().is_empty());
    assert!(RtcpPacket::parse(&[0, 0, 0]).unwrap().is_empty());

    // バージョンが違う
    let mut invalid_version = vec![0; 8];
    invalid_version[0] = 0b0000_0000; // version = 0
    invalid_version[1] = 200; // SR
    invalid_version[2] = 0;
    invalid_version[3] = 1; // length = 1 word
    assert!(RtcpPacket::parse(&invalid_version).is_err());
}
