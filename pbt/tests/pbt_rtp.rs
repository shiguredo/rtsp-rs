use proptest::prelude::*;
use shiguredo_rtsp::rtp::{RtpExtension, RtpHeader, RtpPacket};

/// ペイロードタイプを生成 (0-127)
fn valid_payload_type() -> impl Strategy<Value = u8> {
    0..128u8
}

/// CSRC リストを生成 (0-15 items)
fn valid_csrc_list() -> impl Strategy<Value = Vec<u32>> {
    prop::collection::vec(any::<u32>(), 0..15)
}

/// ペイロードを生成
fn valid_payload() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..1024)
}

/// 拡張データを生成 (4バイト境界に揃える)
fn valid_extension_data() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..64).prop_map(|mut data| {
        // 4バイト境界に揃える
        while data.len() % 4 != 0 {
            data.push(0);
        }
        data
    })
}

/// RTP 拡張ヘッダーを生成
fn valid_extension() -> impl Strategy<Value = Option<RtpExtension>> {
    prop_oneof![
        Just(None),
        (any::<u16>(), valid_extension_data())
            .prop_map(|(profile, data)| { Some(RtpExtension { profile, data }) }),
    ]
}

/// パディングサイズを生成
fn valid_padding_size() -> impl Strategy<Value = u8> {
    prop_oneof![Just(0u8), 1..32u8,]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// RTP パケットの build/parse ラウンドトリップ (基本)
    #[test]
    fn test_rtp_packet_roundtrip_basic(
        payload_type in valid_payload_type(),
        sequence_number in any::<u16>(),
        timestamp in any::<u32>(),
        ssrc in any::<u32>(),
        payload in valid_payload(),
    ) {
        let header = RtpHeader::new(payload_type, sequence_number, timestamp, ssrc);
        let packet = RtpPacket::new(header, payload.clone());

        let encoded = packet.build();
        let decoded = RtpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.header.version, 2);
        prop_assert_eq!(decoded.header.payload_type, payload_type);
        prop_assert_eq!(decoded.header.sequence_number, sequence_number);
        prop_assert_eq!(decoded.header.timestamp, timestamp);
        prop_assert_eq!(decoded.header.ssrc, ssrc);
        prop_assert_eq!(decoded.payload, payload);
    }

    /// RTP パケットの build/parse ラウンドトリップ (マーカー付き)
    #[test]
    fn test_rtp_packet_roundtrip_with_marker(
        payload_type in valid_payload_type(),
        sequence_number in any::<u16>(),
        timestamp in any::<u32>(),
        ssrc in any::<u32>(),
        marker in any::<bool>(),
        payload in valid_payload(),
    ) {
        let mut header = RtpHeader::new(payload_type, sequence_number, timestamp, ssrc);
        header.marker = marker;
        let packet = RtpPacket::new(header, payload.clone());

        let encoded = packet.build();
        let decoded = RtpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.header.marker, marker);
        prop_assert_eq!(decoded.payload, payload);
    }

    /// RTP パケットの build/parse ラウンドトリップ (CSRC リスト付き)
    #[test]
    fn test_rtp_packet_roundtrip_with_csrc(
        payload_type in valid_payload_type(),
        sequence_number in any::<u16>(),
        timestamp in any::<u32>(),
        ssrc in any::<u32>(),
        csrc in valid_csrc_list(),
        payload in valid_payload(),
    ) {
        let mut header = RtpHeader::new(payload_type, sequence_number, timestamp, ssrc);
        header.csrc = csrc.clone();
        let packet = RtpPacket::new(header, payload.clone());

        let encoded = packet.build();
        let decoded = RtpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.header.csrc, csrc);
        prop_assert_eq!(decoded.payload, payload);
    }

    /// RTP パケットの build/parse ラウンドトリップ (拡張ヘッダー付き)
    #[test]
    fn test_rtp_packet_roundtrip_with_extension(
        payload_type in valid_payload_type(),
        sequence_number in any::<u16>(),
        timestamp in any::<u32>(),
        ssrc in any::<u32>(),
        extension in valid_extension(),
        payload in valid_payload(),
    ) {
        let header = RtpHeader::new(payload_type, sequence_number, timestamp, ssrc);
        let mut packet = RtpPacket::new(header, payload.clone());
        packet.extension = extension.clone();

        let encoded = packet.build();
        let decoded = RtpPacket::parse(&encoded).unwrap();

        match (&decoded.extension, &extension) {
            (Some(dec_ext), Some(orig_ext)) => {
                prop_assert_eq!(dec_ext.profile, orig_ext.profile);
                // データ長は4バイト境界に揃えられる
                prop_assert!(dec_ext.data.len() >= orig_ext.data.len());
            }
            (None, None) => {}
            _ => {
                prop_assert!(false, "extension mismatch");
            }
        }
        prop_assert_eq!(decoded.payload, payload);
    }

    /// RTP パケットの build/parse ラウンドトリップ (パディング付き)
    #[test]
    fn test_rtp_packet_roundtrip_with_padding(
        payload_type in valid_payload_type(),
        sequence_number in any::<u16>(),
        timestamp in any::<u32>(),
        ssrc in any::<u32>(),
        padding_size in valid_padding_size(),
        payload in valid_payload(),
    ) {
        let header = RtpHeader::new(payload_type, sequence_number, timestamp, ssrc);
        let mut packet = RtpPacket::new(header, payload.clone());
        packet.padding_size = padding_size;

        let encoded = packet.build();
        let decoded = RtpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.padding_size, padding_size);
        prop_assert_eq!(decoded.payload, payload);
    }

    /// RTP パケットの build/parse ラウンドトリップ (全オプション)
    #[test]
    fn test_rtp_packet_roundtrip_full(
        payload_type in valid_payload_type(),
        sequence_number in any::<u16>(),
        timestamp in any::<u32>(),
        ssrc in any::<u32>(),
        marker in any::<bool>(),
        csrc in valid_csrc_list(),
        extension in valid_extension(),
        padding_size in valid_padding_size(),
        payload in valid_payload(),
    ) {
        let mut header = RtpHeader::new(payload_type, sequence_number, timestamp, ssrc);
        header.marker = marker;
        header.csrc = csrc.clone();

        let mut packet = RtpPacket::new(header, payload.clone());
        packet.extension = extension;
        packet.padding_size = padding_size;

        let encoded = packet.build();
        let decoded = RtpPacket::parse(&encoded).unwrap();

        prop_assert_eq!(decoded.header.version, 2);
        prop_assert_eq!(decoded.header.payload_type, payload_type);
        prop_assert_eq!(decoded.header.sequence_number, sequence_number);
        prop_assert_eq!(decoded.header.timestamp, timestamp);
        prop_assert_eq!(decoded.header.ssrc, ssrc);
        prop_assert_eq!(decoded.header.marker, marker);
        prop_assert_eq!(decoded.header.csrc, csrc);
        prop_assert_eq!(decoded.padding_size, padding_size);
        prop_assert_eq!(decoded.payload, payload);
    }

    /// RTP パケットサイズの検証
    #[test]
    fn test_rtp_packet_size(
        payload_type in valid_payload_type(),
        sequence_number in any::<u16>(),
        timestamp in any::<u32>(),
        ssrc in any::<u32>(),
        csrc in valid_csrc_list(),
        payload in valid_payload(),
    ) {
        let mut header = RtpHeader::new(payload_type, sequence_number, timestamp, ssrc);
        header.csrc = csrc.clone();
        let packet = RtpPacket::new(header, payload.clone());

        let encoded = packet.build();
        let expected_size = 12 + csrc.len() * 4 + payload.len();

        prop_assert_eq!(encoded.len(), expected_size);
        prop_assert_eq!(packet.size(), expected_size);
    }
}

/// 不正なデータのパースが失敗することを確認
#[test]
fn test_rtp_parse_invalid_data() {
    // データが短すぎる
    assert!(RtpPacket::parse(&[]).is_err());
    assert!(RtpPacket::parse(&[0; 11]).is_err());

    // バージョンが違う
    let mut invalid_version = vec![0; 12];
    invalid_version[0] = 0b0000_0000; // version = 0
    assert!(RtpPacket::parse(&invalid_version).is_err());

    invalid_version[0] = 0b0100_0000; // version = 1
    assert!(RtpPacket::parse(&invalid_version).is_err());

    invalid_version[0] = 0b1100_0000; // version = 3
    assert!(RtpPacket::parse(&invalid_version).is_err());
}
