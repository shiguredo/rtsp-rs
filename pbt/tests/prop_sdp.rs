use proptest::prelude::*;
use shiguredo_rtsp::sdp::{
    Sdp, SdpAttribute, SdpConnection, SdpMedia, SdpMediaBuilder, SdpOrigin, SdpTiming,
};

/// 有効なユーザー名を生成
fn valid_username() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("-".to_string()),
        prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_-]{0,20}")
            .expect("有効な正規表現なのでコンパイルに失敗しない想定"),
    ]
}

/// 有効なセッション ID を生成
fn valid_session_id() -> impl Strategy<Value = String> {
    prop::string::string_regex("[0-9]{1,15}")
        .expect("有効な正規表現なのでコンパイルに失敗しない想定")
}

/// 有効なセッションバージョンを生成
fn valid_session_version() -> impl Strategy<Value = String> {
    prop::string::string_regex("[0-9]{1,10}")
        .expect("有効な正規表現なのでコンパイルに失敗しない想定")
}

/// 有効な IP アドレスを生成
fn valid_ip_address() -> impl Strategy<Value = String> {
    (1..256u32, 0..256u32, 0..256u32, 1..255u32).prop_map(|(a, b, c, d)| format!("{a}.{b}.{c}.{d}"))
}

/// 有効なセッション名を生成 (空白を含まない)
fn valid_session_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_-]{1,50}")
        .expect("有効な正規表現なのでコンパイルに失敗しない想定")
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// Origin を生成
fn valid_origin() -> impl Strategy<Value = SdpOrigin> {
    (
        valid_username(),
        valid_session_id(),
        valid_session_version(),
        valid_ip_address(),
    )
        .prop_map(
            |(username, session_id, session_version, address)| SdpOrigin {
                username,
                session_id,
                session_version,
                net_type: "IN".to_string(),
                addr_type: "IP4".to_string(),
                address,
            },
        )
}

/// Connection を生成
fn valid_connection() -> impl Strategy<Value = SdpConnection> {
    valid_ip_address().prop_map(|address| SdpConnection {
        net_type: "IN".to_string(),
        addr_type: "IP4".to_string(),
        address,
    })
}

/// Timing を生成
fn valid_timing() -> impl Strategy<Value = SdpTiming> {
    (any::<u32>(), any::<u32>()).prop_map(|(start, stop)| SdpTiming {
        start: start as u64,
        stop: stop as u64,
    })
}

/// ポート番号を生成
fn valid_port() -> impl Strategy<Value = u16> {
    1024..65535u16
}

/// ペイロードタイプを生成
fn valid_payload_type() -> impl Strategy<Value = u8> {
    96..128u8
}

/// エンコーディング名を生成
fn valid_encoding() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("H264".to_string()),
        Just("H265".to_string()),
        Just("VP8".to_string()),
        Just("VP9".to_string()),
        Just("PCMU".to_string()),
        Just("PCMA".to_string()),
        Just("opus".to_string()),
    ]
}

/// クロックレートを生成
fn valid_clock_rate() -> impl Strategy<Value = u32> {
    prop_oneof![
        Just(8000u32),
        Just(16000u32),
        Just(44100u32),
        Just(48000u32),
        Just(90000u32),
    ]
}

/// メディアタイプを生成
fn valid_media_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("video".to_string()),
        Just("audio".to_string()),
        Just("application".to_string()),
    ]
}

/// プロトコルを生成
fn valid_protocol() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("RTP/AVP".to_string()),
        Just("RTP/SAVP".to_string()),
        Just("UDP".to_string()),
    ]
}

/// セッションレベル属性を生成 (f64 を含まないもののみ)
fn valid_session_attribute() -> impl Strategy<Value = SdpAttribute> {
    prop_oneof![
        Just(SdpAttribute::Recvonly),
        Just(SdpAttribute::Sendrecv),
        Just(SdpAttribute::Sendonly),
        Just(SdpAttribute::Inactive),
        prop::string::string_regex("[a-z0-9-]{1,20}")
            .expect("有効な正規表現なのでコンパイルに失敗しない想定")
            .prop_map(SdpAttribute::Control),
        prop::string::string_regex("npt=[0-9]+-[0-9]*")
            .expect("有効な正規表現なのでコンパイルに失敗しない想定")
            .prop_map(SdpAttribute::Range),
        prop::string::string_regex("[a-zA-Z0-9_-]{1,30}")
            .expect("有効な正規表現なのでコンパイルに失敗しない想定")
            .prop_map(SdpAttribute::Tool),
    ]
}

/// メディアレベル属性を生成
fn valid_media_attribute() -> impl Strategy<Value = SdpAttribute> {
    prop_oneof![
        (valid_payload_type(), valid_encoding(), valid_clock_rate()).prop_map(|(pt, enc, rate)| {
            SdpAttribute::Rtpmap {
                payload_type: pt,
                encoding: enc,
                clock_rate: rate,
                encoding_params: None,
            }
        }),
        (
            valid_payload_type(),
            prop::string::string_regex("[a-zA-Z0-9=;-]{1,50}")
                .expect("有効な正規表現なのでコンパイルに失敗しない想定")
        )
            .prop_map(|(pt, params)| {
                SdpAttribute::Fmtp {
                    payload_type: pt,
                    parameters: params,
                }
            }),
        prop::string::string_regex("trackID=[0-9]{1,5}")
            .expect("有効な正規表現なのでコンパイルに失敗しない想定")
            .prop_map(SdpAttribute::Control),
    ]
}

/// メディアを生成
fn valid_media() -> impl Strategy<Value = SdpMedia> {
    (
        valid_media_type(),
        valid_port(),
        valid_protocol(),
        prop::collection::vec(valid_payload_type().prop_map(|pt| pt.to_string()), 1..3),
        prop::collection::vec(valid_media_attribute(), 0..3),
    )
        .prop_map(
            |(media_type, port, protocol, formats, attributes)| SdpMedia {
                media_type,
                port,
                num_ports: None,
                protocol,
                formats,
                title: None,
                connection: None,
                bandwidth: Vec::new(),
                attributes,
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// SDP の parse/to_string ラウンドトリップ (基本)
    #[test]
    fn test_sdp_roundtrip_basic(
        origin in valid_origin(),
        session_name in valid_session_name(),
        timing in valid_timing(),
    ) {
        let sdp = Sdp {
            version: 0,
            origin: origin.clone(),
            session_name: session_name.clone(),
            session_info: None,
            uri: None,
            email: None,
            phone: None,
            connection: None,
            bandwidth: Vec::new(),
            timing: timing.clone(),
            attributes: Vec::new(),
            media: Vec::new(),
        };

        let text = sdp.to_string();
        let parsed = Sdp::parse(&text)
            .expect("有効な SDP なのでパースに失敗しない想定");

        prop_assert_eq!(parsed.version, 0);
        prop_assert_eq!(parsed.origin.username, origin.username);
        prop_assert_eq!(parsed.origin.session_id, origin.session_id);
        prop_assert_eq!(parsed.origin.address, origin.address);
        prop_assert_eq!(parsed.session_name, session_name);
        prop_assert_eq!(parsed.timing.start, timing.start);
        prop_assert_eq!(parsed.timing.stop, timing.stop);
    }

    /// SDP の parse/to_string ラウンドトリップ (connection 付き)
    #[test]
    fn test_sdp_roundtrip_with_connection(
        origin in valid_origin(),
        session_name in valid_session_name(),
        timing in valid_timing(),
        connection in valid_connection(),
    ) {
        let sdp = Sdp {
            version: 0,
            origin: origin.clone(),
            session_name: session_name.clone(),
            session_info: None,
            uri: None,
            email: None,
            phone: None,
            connection: Some(connection.clone()),
            bandwidth: Vec::new(),
            timing: timing.clone(),
            attributes: Vec::new(),
            media: Vec::new(),
        };

        let text = sdp.to_string();
        let parsed = Sdp::parse(&text)
            .expect("有効な SDP なのでパースに失敗しない想定");

        prop_assert!(parsed.connection.is_some());
        let parsed_conn = parsed
            .connection
            .expect("connection は設定済みなので取得できる想定");
        prop_assert_eq!(parsed_conn.net_type, connection.net_type);
        prop_assert_eq!(parsed_conn.addr_type, connection.addr_type);
        prop_assert_eq!(parsed_conn.address, connection.address);
    }

    /// SDP の parse/to_string ラウンドトリップ (メディア付き)
    #[test]
    fn test_sdp_roundtrip_with_media(
        origin in valid_origin(),
        session_name in valid_session_name(),
        timing in valid_timing(),
        media in prop::collection::vec(valid_media(), 1..3),
    ) {
        let sdp = Sdp {
            version: 0,
            origin: origin.clone(),
            session_name: session_name.clone(),
            session_info: None,
            uri: None,
            email: None,
            phone: None,
            connection: None,
            bandwidth: Vec::new(),
            timing: timing.clone(),
            attributes: Vec::new(),
            media: media.clone(),
        };

        let text = sdp.to_string();
        let parsed = Sdp::parse(&text)
            .expect("有効な SDP なのでパースに失敗しない想定");

        prop_assert_eq!(parsed.media.len(), media.len());
        for (orig, dec) in media.iter().zip(parsed.media.iter()) {
            prop_assert_eq!(&dec.media_type, &orig.media_type);
            prop_assert_eq!(dec.port, orig.port);
            prop_assert_eq!(&dec.protocol, &orig.protocol);
            prop_assert_eq!(dec.formats.len(), orig.formats.len());
        }
    }

    /// SDP の parse/to_string ラウンドトリップ (属性付き)
    #[test]
    fn test_sdp_roundtrip_with_attributes(
        origin in valid_origin(),
        session_name in valid_session_name(),
        timing in valid_timing(),
        attributes in prop::collection::vec(valid_session_attribute(), 0..3),
    ) {
        let sdp = Sdp {
            version: 0,
            origin: origin.clone(),
            session_name: session_name.clone(),
            session_info: None,
            uri: None,
            email: None,
            phone: None,
            connection: None,
            bandwidth: Vec::new(),
            timing: timing.clone(),
            attributes: attributes.clone(),
            media: Vec::new(),
        };

        let text = sdp.to_string();
        let parsed = Sdp::parse(&text)
            .expect("有効な SDP なのでパースに失敗しない想定");

        prop_assert_eq!(parsed.attributes.len(), attributes.len());
    }

    /// SdpBuilder の検証
    #[test]
    fn test_sdp_builder(
        session_id in valid_session_id(),
        address in valid_ip_address(),
        session_name in valid_session_name(),
    ) {
        let sdp = Sdp::builder()
            .origin_simple(&session_id, &address)
            .session_name(&session_name)
            .timing(0, 0)
            .build()
            // ビルダーで生成した SDP は必ず妥当な状態になる想定
            .expect("ビルダーで生成した SDP のビルドに失敗しない想定");

        prop_assert_eq!(sdp.version, 0);
        prop_assert_eq!(sdp.origin.session_id, session_id);
        prop_assert_eq!(sdp.origin.address, address);
        prop_assert_eq!(sdp.session_name, session_name);
    }

    /// SdpMediaBuilder の検証
    #[test]
    fn test_sdp_media_builder(
        port in valid_port(),
        payload_type in valid_payload_type(),
        encoding in valid_encoding(),
        clock_rate in valid_clock_rate(),
    ) {
        let media = SdpMediaBuilder::video(port)
            .format(&payload_type.to_string())
            .rtpmap(payload_type, &encoding, clock_rate)
            .control("trackID=1")
            .build();

        prop_assert_eq!(media.media_type, "video");
        prop_assert_eq!(media.port, port);
        prop_assert_eq!(media.protocol, "RTP/AVP");
        prop_assert_eq!(media.formats.len(), 1);
        prop_assert_eq!(media.attributes.len(), 2);
    }
}
