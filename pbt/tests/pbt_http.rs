use proptest::prelude::*;
use shiguredo_http11::{
    Request, RequestDecoder, Response, ResponseDecoder, encode_request, encode_response,
};

/// 有効な HTTP トークン文字列を生成 (制御文字やセパレータを除く)
fn valid_token() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_-]{0,30}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// 有効な URI 文字列を生成
fn valid_uri() -> impl Strategy<Value = String> {
    prop::string::string_regex("rtsp://[a-z0-9.-]+/[a-zA-Z0-9/_.-]*")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// 有効なバージョン文字列を生成
///
/// HTTP/1.1 はエンコード時に Host ヘッダーが必須なので除外する
fn valid_version() -> impl Strategy<Value = String> {
    prop_oneof![Just("RTSP/1.0".to_string()), Just("RTSP/2.0".to_string()),]
}

/// 有効なヘッダー名を生成
fn valid_header_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[A-Za-z][A-Za-z0-9-]{0,20}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// 有効なヘッダー値を生成 (CRLF を含まない)
fn valid_header_value() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 ,;:=/_.-]{0,100}").unwrap()
}

/// ヘッダーリストを生成 (Content-Length, Transfer-Encoding を除外)
fn valid_headers() -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::vec((valid_header_name(), valid_header_value()), 0..5).prop_filter(
        "no Content-Length or Transfer-Encoding",
        |headers| {
            headers.iter().all(|(name, _)| {
                !name.eq_ignore_ascii_case("Content-Length")
                    && !name.eq_ignore_ascii_case("Transfer-Encoding")
            })
        },
    )
}

/// ボディを持てるステータスコードを生成 (1xx, 204, 304 を除外)
fn valid_status_code_with_body() -> impl Strategy<Value = u16> {
    prop_oneof![
        Just(200u16),
        Just(201u16),
        Just(301u16),
        Just(400u16),
        Just(401u16),
        Just(404u16),
        Just(500u16),
    ]
}

/// Reason phrase を生成
fn valid_reason_phrase() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("OK".to_string()),
        Just("Not Found".to_string()),
        Just("Bad Request".to_string()),
        Just("Internal Server Error".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// HTTP リクエストの encode/decode ラウンドトリップ (ボディなし)
    #[test]
    fn test_http_request_roundtrip_no_body(
        method in valid_token(),
        uri in valid_uri(),
        version in valid_version(),
        headers in valid_headers(),
    ) {
        let request = Request {
            method: method.clone(),
            uri: uri.clone(),
            version: version.clone(),
            headers: headers.clone(),
            body: vec![],
        };

        let encoded = encode_request(&request).unwrap();
        let mut decoder = RequestDecoder::new();
        decoder.feed(&encoded).unwrap();

        let decoded = decoder.decode().unwrap().unwrap();

        prop_assert_eq!(&decoded.method, &method);
        prop_assert_eq!(&decoded.uri, &uri);
        prop_assert_eq!(&decoded.version, &version);
        // ボディなしの場合、エンコーダーは Content-Length を追加しない
        prop_assert_eq!(decoded.headers.len(), headers.len());
        prop_assert!(decoded.body.is_empty());
    }

    /// HTTP リクエストの encode/decode ラウンドトリップ (ボディあり)
    #[test]
    fn test_http_request_roundtrip_with_body(
        method in valid_token(),
        uri in valid_uri(),
        version in valid_version(),
        headers in valid_headers(),
        body in prop::collection::vec(any::<u8>(), 1..256),
    ) {
        let request = Request {
            method: method.clone(),
            uri: uri.clone(),
            version: version.clone(),
            headers: headers.clone(),
            body: body.clone(),
        };

        let encoded = encode_request(&request).unwrap();
        let mut decoder = RequestDecoder::new();
        decoder.feed(&encoded).unwrap();

        let decoded = decoder.decode().unwrap().unwrap();

        prop_assert_eq!(&decoded.method, &method);
        prop_assert_eq!(&decoded.uri, &uri);
        prop_assert_eq!(&decoded.version, &version);
        prop_assert_eq!(&decoded.body, &body);
        // エンコーダーが Content-Length を自動追加するため +1
        prop_assert_eq!(decoded.headers.len(), headers.len() + 1);
    }

    /// HTTP レスポンスの encode/decode ラウンドトリップ (ボディなし)
    #[test]
    fn test_http_response_roundtrip_no_body(
        version in valid_version(),
        status_code in valid_status_code_with_body(),
        reason_phrase in valid_reason_phrase(),
        headers in valid_headers(),
    ) {
        let response = Response {
            version: version.clone(),
            status_code,
            reason_phrase: reason_phrase.clone(),
            headers: headers.clone(),
            body: vec![],
            omit_body: false,
        };

        let encoded = encode_response(&response).unwrap();
        let mut decoder = ResponseDecoder::new();
        decoder.feed(&encoded).unwrap();

        let decoded = decoder.decode().unwrap().unwrap();

        prop_assert_eq!(&decoded.version, &version);
        prop_assert_eq!(decoded.status_code, status_code);
        prop_assert_eq!(&decoded.reason_phrase, &reason_phrase);
        // エンコーダーが Content-Length: 0 を自動追加するため +1
        prop_assert_eq!(decoded.headers.len(), headers.len() + 1);
        prop_assert!(decoded.body.is_empty());
    }

    /// HTTP レスポンスの encode/decode ラウンドトリップ (ボディあり)
    #[test]
    fn test_http_response_roundtrip_with_body(
        version in valid_version(),
        status_code in valid_status_code_with_body(),
        reason_phrase in valid_reason_phrase(),
        headers in valid_headers(),
        body in prop::collection::vec(any::<u8>(), 1..256),
    ) {
        let response = Response {
            version: version.clone(),
            status_code,
            reason_phrase: reason_phrase.clone(),
            headers: headers.clone(),
            body: body.clone(),
            omit_body: false,
        };

        let encoded = encode_response(&response).unwrap();
        let mut decoder = ResponseDecoder::new();
        decoder.feed(&encoded).unwrap();

        let decoded = decoder.decode().unwrap().unwrap();

        prop_assert_eq!(&decoded.version, &version);
        prop_assert_eq!(decoded.status_code, status_code);
        prop_assert_eq!(&decoded.reason_phrase, &reason_phrase);
        prop_assert_eq!(&decoded.body, &body);
        // エンコーダーが Content-Length を自動追加するため +1
        prop_assert_eq!(decoded.headers.len(), headers.len() + 1);
    }

    /// 分割されたデータでもパースできることを確認
    #[test]
    fn test_http_request_chunked_feed(
        method in valid_token(),
        uri in valid_uri(),
    ) {
        let request = Request::with_version(&method, &uri, "RTSP/1.0");
        let encoded = encode_request(&request).unwrap();

        let mut decoder = RequestDecoder::new();

        // 1バイトずつ feed
        for (i, byte) in encoded.iter().enumerate() {
            decoder.feed(&[*byte]).unwrap();
            let result = decoder.decode();

            if i < encoded.len() - 1 {
                // まだ完了していない
                prop_assert!(result.unwrap().is_none());
            } else {
                // 最後のバイトで完了
                let decoded = result.unwrap().unwrap();
                prop_assert_eq!(&decoded.method, &method);
                prop_assert_eq!(&decoded.uri, &uri);
            }
        }
    }
}
