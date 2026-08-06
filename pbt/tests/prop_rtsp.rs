use proptest::prelude::*;
use shiguredo_rtsp::rtsp_connection::RtspTransport;
use shiguredo_rtsp::rtsp_method::RtspMethod;
use shiguredo_rtsp::rtsp_range::{NptRange, NptTime, RtspRange, SmpteRange, SmpteTime, SmpteType};

/// 有効な Transport プロトコルを生成
fn valid_transport_protocol() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("RTP/AVP".to_string()),
        Just("RTP/AVP/TCP".to_string()),
        Just("RTP/SAVP".to_string()),
    ]
}

/// 有効な Transport を生成
fn valid_transport() -> impl Strategy<Value = RtspTransport> {
    (
        valid_transport_protocol(),
        any::<bool>(),
        proptest::option::of((0..128u8, 0..128u8)),
        proptest::option::of((1024..65000u16, 1024..65000u16)),
        proptest::option::of((1024..65000u16, 1024..65000u16)),
        proptest::option::of(any::<u32>()),
        proptest::option::of(prop_oneof![
            Just("PLAY".to_string()),
            Just("RECORD".to_string()),
        ]),
        proptest::option::of(
            prop::string::string_regex("[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}")
                .expect("有効な正規表現なのでコンパイルに失敗しない想定"),
        ),
        proptest::option::of(
            prop::string::string_regex("[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}")
                .expect("有効な正規表現なのでコンパイルに失敗しない想定"),
        ),
        proptest::option::of(1..255u8),
    )
        .prop_map(
            |(
                protocol,
                unicast,
                interleaved,
                client_port,
                server_port,
                ssrc,
                mode,
                destination,
                source,
                ttl,
            )| {
                RtspTransport {
                    protocol,
                    unicast,
                    interleaved,
                    client_port,
                    server_port,
                    ssrc,
                    mode,
                    destination,
                    source,
                    ttl,
                    ..Default::default()
                }
            },
        )
}

/// 有効な SMPTE タイプを生成
fn valid_smpte_type() -> impl Strategy<Value = SmpteType> {
    prop_oneof![
        Just(SmpteType::Smpte),
        Just(SmpteType::Smpte30Drop),
        Just(SmpteType::Smpte25),
    ]
}

/// 有効な SMPTE 時刻を生成
fn valid_smpte_time() -> impl Strategy<Value = SmpteTime> {
    (0..24u8, 0..60u8, 0..60u8, 0..30u8).prop_map(|(h, m, s, f)| SmpteTime {
        hours: h,
        minutes: m,
        seconds: s,
        frames: f,
        subframes: None,
    })
}

/// 有効な拡張メソッド名を生成
fn valid_extension_method() -> impl Strategy<Value = String> {
    prop::string::string_regex("[A-Z][A-Z_]{2,20}")
        .expect("有効な正規表現なのでコンパイルに失敗しない想定")
        .prop_filter("not a standard method", |s| {
            !matches!(
                s.as_str(),
                "OPTIONS"
                    | "DESCRIBE"
                    | "ANNOUNCE"
                    | "SETUP"
                    | "PLAY"
                    | "PAUSE"
                    | "TEARDOWN"
                    | "GET_PARAMETER"
                    | "SET_PARAMETER"
                    | "REDIRECT"
                    | "RECORD"
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Transport ヘッダーのラウンドトリップ
    #[test]
    fn test_transport_roundtrip(transport in valid_transport()) {
        let header = transport.to_header();
        let parsed = RtspTransport::parse(&header);

        prop_assert_eq!(parsed.protocol, transport.protocol);
        prop_assert_eq!(parsed.unicast, transport.unicast);
        prop_assert_eq!(parsed.interleaved, transport.interleaved);
        prop_assert_eq!(parsed.client_port, transport.client_port);
        prop_assert_eq!(parsed.server_port, transport.server_port);
        prop_assert_eq!(parsed.ssrc, transport.ssrc);
        prop_assert_eq!(parsed.mode, transport.mode);
        prop_assert_eq!(parsed.destination, transport.destination);
        prop_assert_eq!(parsed.source, transport.source);
        prop_assert_eq!(parsed.ttl, transport.ttl);
    }

    /// 複数 Transport ヘッダーのパース
    #[test]
    fn test_transport_parse_multiple(
        t1 in valid_transport(),
        t2 in valid_transport(),
    ) {
        let header = format!("{}, {}", t1.to_header(), t2.to_header());
        let parsed = RtspTransport::parse_multiple(&header);

        prop_assert_eq!(parsed.len(), 2);
        prop_assert_eq!(&parsed[0].protocol, &t1.protocol);
        prop_assert_eq!(&parsed[1].protocol, &t2.protocol);
    }

    /// SMPTE タイプ保持のラウンドトリップ
    #[test]
    fn test_smpte_type_roundtrip(
        smpte_type in valid_smpte_type(),
        start in valid_smpte_time(),
    ) {
        let range = RtspRange::Smpte(SmpteRange {
            smpte_type,
            start,
            end: None,
        });

        let text = range.to_string();
        let parsed = RtspRange::parse(&text)
            .expect("有効な Range なのでパースに失敗しない想定");

        if let RtspRange::Smpte(smpte) = parsed {
            prop_assert_eq!(smpte.smpte_type, smpte_type);
        } else {
            prop_assert!(false, "expected Smpte range");
        }
    }

    /// NPT レンジのラウンドトリップ
    #[test]
    fn test_npt_roundtrip(seconds in 0.0f64..86400.0) {
        let range = RtspRange::Npt(NptRange {
            start: NptTime::Seconds(seconds),
            end: None,
        });

        let text = range.to_string();
        let parsed = RtspRange::parse(&text)
            .expect("有効な Range なのでパースに失敗しない想定");

        if let RtspRange::Npt(npt) = parsed {
            if let NptTime::Seconds(s) = npt.start {
                prop_assert!((s - seconds).abs() < 0.001);
            } else {
                prop_assert!(false, "expected Seconds");
            }
        } else {
            prop_assert!(false, "expected Npt range");
        }
    }

    /// Extension メソッドのラウンドトリップ
    #[test]
    fn test_extension_method_roundtrip(name in valid_extension_method()) {
        let method: RtspMethod = name
            .parse()
            .expect("有効なメソッド名なのでパースに失敗しない想定");

        if let RtspMethod::Extension(ref ext_name) = method {
            prop_assert_eq!(ext_name, &name);
            prop_assert_eq!(method.as_str(), name.as_str());
            prop_assert_eq!(method.to_string(), name);
        } else {
            prop_assert!(false, "expected Extension method");
        }
    }

    /// 標準メソッドは大文字小文字を区別する
    #[test]
    fn test_standard_method_case_sensitive(
        method_str in prop_oneof![
            Just("OPTIONS"),
            Just("DESCRIBE"),
            Just("ANNOUNCE"),
            Just("SETUP"),
            Just("PLAY"),
            Just("PAUSE"),
            Just("TEARDOWN"),
            Just("GET_PARAMETER"),
            Just("SET_PARAMETER"),
            Just("REDIRECT"),
            Just("RECORD"),
        ],
    ) {
        let method: RtspMethod = method_str
            .parse()
            .expect("有効なメソッド名なのでパースに失敗しない想定");
        // 標準メソッドは Extension にならない
        prop_assert!(!matches!(method, RtspMethod::Extension(_)));

        // 小文字にすると Extension になる
        let lower: RtspMethod = method_str
            .to_lowercase()
            .parse()
            .expect("有効なメソッド名なのでパースに失敗しない想定");
        prop_assert!(matches!(lower, RtspMethod::Extension(_)));
    }
}
