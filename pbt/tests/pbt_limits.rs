use proptest::prelude::*;
use shiguredo_http11::{
    DecoderLimits, Request, RequestDecoder, Response, ResponseDecoder, encode_request,
    encode_response,
};
use shiguredo_rtsp::{RtspClientConnection, RtspConnectionLimits};

/// 小さな制限を持つデコーダーを作成
fn request_decoder_with_small_limits() -> RequestDecoder {
    RequestDecoder::with_limits(DecoderLimits {
        max_buffer_size: 1024,
        max_headers_count: 5,
        max_header_line_size: 128,
        max_body_size: 256,
        ..Default::default()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// バッファサイズ制限を超えるとエラーになることを確認
    #[test]
    fn test_buffer_size_limit_exceeded(
        data in prop::collection::vec(any::<u8>(), 2000..3000)
    ) {
        let mut decoder = request_decoder_with_small_limits();
        let result = decoder.feed(&data);
        prop_assert!(result.is_err());
    }

    /// バッファサイズ制限内ならエラーにならないことを確認
    #[test]
    fn test_buffer_size_within_limit(
        data in prop::collection::vec(any::<u8>(), 0..500)
    ) {
        let mut decoder = request_decoder_with_small_limits();
        let result = decoder.feed(&data);
        prop_assert!(result.is_ok());
    }

    /// ヘッダー数制限を超えるとエラーになることを確認
    #[test]
    fn test_header_count_limit_exceeded(
        header_count in 10usize..20usize
    ) {
        let mut decoder = request_decoder_with_small_limits();

        let mut request = "OPTIONS rtsp://example.com RTSP/1.0\r\n".to_string();
        for i in 0..header_count {
            request.push_str(&format!("Header{}: value{}\r\n", i, i));
        }
        request.push_str("\r\n");

        decoder.feed(request.as_bytes()).unwrap();
        let result = decoder.decode();
        prop_assert!(result.is_err());
    }

    /// ヘッダー数制限内ならパースできることを確認
    #[test]
    fn test_header_count_within_limit(
        header_count in 1usize..5usize
    ) {
        let mut decoder = request_decoder_with_small_limits();

        let mut request = "OPTIONS rtsp://example.com RTSP/1.0\r\n".to_string();
        for i in 0..header_count {
            request.push_str(&format!("H{}: v{}\r\n", i, i));
        }
        request.push_str("\r\n");

        decoder.feed(request.as_bytes()).unwrap();
        let result = decoder.decode();
        prop_assert!(result.is_ok());
        prop_assert!(result.unwrap().is_some());
    }

    /// ボディサイズ制限を超えるとエラーになることを確認
    #[test]
    fn test_body_size_limit_exceeded(
        body_size in 500usize..1000usize
    ) {
        let mut decoder = request_decoder_with_small_limits();

        let request = format!(
            "POST rtsp://example.com RTSP/1.0\r\nContent-Length: {}\r\n\r\n",
            body_size
        );

        decoder.feed(request.as_bytes()).unwrap();
        let result = decoder.decode();
        prop_assert!(result.is_err());
    }

    /// ボディサイズ制限内ならパースできることを確認
    #[test]
    fn test_body_size_within_limit(
        body_size in 1usize..100usize
    ) {
        let mut decoder = request_decoder_with_small_limits();

        let body = vec![b'x'; body_size];
        let request = format!(
            "POST rtsp://example.com RTSP/1.0\r\nContent-Length: {}\r\n\r\n",
            body_size
        );

        decoder.feed(request.as_bytes()).unwrap();
        decoder.feed(&body).unwrap();
        let result = decoder.decode();
        prop_assert!(result.is_ok());
        let req = result.unwrap().unwrap();
        prop_assert_eq!(req.body.len(), body_size);
    }

    /// ヘッダー行サイズ制限を超えるとエラーになることを確認
    #[test]
    fn test_header_line_size_limit_exceeded(
        value_len in 200usize..500usize
    ) {
        let mut decoder = request_decoder_with_small_limits();

        let value: String = (0..value_len).map(|_| 'x').collect();
        let request = format!(
            "OPTIONS rtsp://example.com RTSP/1.0\r\nLongHeader: {}\r\n\r\n",
            value
        );

        decoder.feed(request.as_bytes()).unwrap();
        let result = decoder.decode();
        prop_assert!(result.is_err());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Interleaved フレームサイズ制限を超えるとエラー (クライアント)
    #[test]
    fn test_interleaved_frame_size_limit_client(
        frame_size in 2000u16..10000u16
    ) {
        let limits = RtspConnectionLimits {
            max_interleaved_frame_size: 1000,  // Small limit for testing
            ..Default::default()
        };
        let mut conn = RtspClientConnection::with_limits(limits);

        // Interleaved frame header: $ + channel + length (big-endian)
        let mut frame = vec![b'$', 0];
        frame.extend_from_slice(&frame_size.to_be_bytes());

        let result = conn.feed_recv_buf(&frame);
        prop_assert!(result.is_err());
    }

    /// Interleaved フレームサイズ制限内なら受け入れ
    #[test]
    fn test_interleaved_frame_within_limit(
        frame_size in 12u16..1000u16
    ) {
        let limits = RtspConnectionLimits {
            max_interleaved_frame_size: 64 * 1024,
            ..Default::default()
        };
        let mut conn = RtspClientConnection::with_limits(limits);

        // Build a valid RTP packet
        let mut rtp_data = vec![
            0x80, 0x60,  // V=2, P=0, X=0, CC=0, M=0, PT=96
            0x00, 0x01,  // Sequence number
            0x00, 0x00, 0x00, 0x00,  // Timestamp
            0x12, 0x34, 0x56, 0x78,  // SSRC
        ];
        // Pad to requested size
        while rtp_data.len() < frame_size as usize {
            rtp_data.push(0);
        }
        rtp_data.truncate(frame_size as usize);

        // Build interleaved frame
        let mut frame = vec![b'$', 0];  // channel 0 = RTP
        frame.extend_from_slice(&(rtp_data.len() as u16).to_be_bytes());
        frame.extend_from_slice(&rtp_data);

        let result = conn.feed_recv_buf(&frame);
        prop_assert!(result.is_ok());
    }
}

// HTTP エンコード/デコードの制限付きラウンドトリップ
proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    #[test]
    fn test_request_roundtrip_with_limits(
        method in "[A-Z]{3,10}",
        uri in "rtsp://[a-z]+/[a-z]+",
        header_count in 0usize..3usize,
        body_size in 0usize..100usize
    ) {
        let mut request = Request::with_version(&method, &uri, "RTSP/1.0");
        for i in 0..header_count {
            request = request.header(&format!("H{}", i), &format!("v{}", i));
        }
        if body_size > 0 {
            request = request.body(vec![b'x'; body_size]);
        }

        let encoded = encode_request(&request).unwrap();

        // デフォルト制限でデコードできることを確認
        let mut decoder = RequestDecoder::new();
        decoder.feed(&encoded).unwrap();
        let decoded = decoder.decode().unwrap();
        prop_assert!(decoded.is_some());
    }

    #[test]
    fn test_response_roundtrip_with_limits(
        status_code in prop::sample::select(vec![200u16, 301, 400, 404, 500]),
        header_count in 0usize..3usize,
        body_size in 0usize..100usize
    ) {
        let mut response = Response::with_version("RTSP/1.0", status_code, "OK");
        for i in 0..header_count {
            response = response.header(&format!("H{}", i), &format!("v{}", i));
        }
        if body_size > 0 {
            response = response.body(vec![b'x'; body_size]);
        }

        let encoded = encode_response(&response).unwrap();

        // デフォルト制限でデコードできることを確認
        let mut decoder = ResponseDecoder::new();
        decoder.feed(&encoded).unwrap();
        let decoded = decoder.decode().unwrap();
        prop_assert!(decoded.is_some());
    }
}
