// SDP の公開 API に対する単体テスト

use shiguredo_rtsp::sdp::{Sdp, SdpAttribute};

/// rtpmap 属性のラウンドトリップ
#[test]
fn test_sdp_rtpmap_roundtrip() {
    let sdp_text = r#"v=0
o=- 1234567890 1 IN IP4 127.0.0.1
s=Test
t=0 0
m=video 0 RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 profile-level-id=42e01f
"#;

    let parsed = Sdp::parse(sdp_text).expect("有効な SDP なのでパースに失敗しない想定");
    assert_eq!(parsed.media.len(), 1);
    assert_eq!(parsed.media[0].attributes.len(), 2);

    if let SdpAttribute::Rtpmap {
        payload_type,
        encoding,
        clock_rate,
        ..
    } = &parsed.media[0].attributes[0]
    {
        assert_eq!(*payload_type, 96);
        assert_eq!(encoding, "H264");
        assert_eq!(*clock_rate, 90000);
    } else {
        panic!("expected rtpmap");
    }

    // Re-serialize and parse again
    let text = parsed.to_string();
    let reparsed = Sdp::parse(&text).expect("有効な SDP なのでパースに失敗しない想定");

    assert_eq!(reparsed.media.len(), 1);
    assert_eq!(reparsed.media[0].attributes.len(), 2);
}

/// bandwidth のラウンドトリップ
#[test]
fn test_sdp_bandwidth_roundtrip() {
    let sdp_text = r#"v=0
o=- 1234567890 1 IN IP4 127.0.0.1
s=Test
b=AS:256
t=0 0
"#;

    let parsed = Sdp::parse(sdp_text).expect("有効な SDP なのでパースに失敗しない想定");
    assert_eq!(parsed.bandwidth.len(), 1);
    assert_eq!(parsed.bandwidth[0].bwtype, "AS");
    assert_eq!(parsed.bandwidth[0].bandwidth, 256);

    let text = parsed.to_string();
    let reparsed = Sdp::parse(&text).expect("有効な SDP なのでパースに失敗しない想定");
    assert_eq!(reparsed.bandwidth.len(), 1);
    assert_eq!(reparsed.bandwidth[0].bandwidth, 256);
}

/// メディアの num_ports のラウンドトリップ
#[test]
fn test_sdp_media_num_ports_roundtrip() {
    let sdp_text = r#"v=0
o=- 1234567890 1 IN IP4 127.0.0.1
s=Test
t=0 0
m=video 49170/2 RTP/AVP 96
"#;

    let parsed = Sdp::parse(sdp_text).expect("有効な SDP なのでパースに失敗しない想定");
    assert_eq!(parsed.media.len(), 1);
    assert_eq!(parsed.media[0].port, 49170);
    assert_eq!(parsed.media[0].num_ports, Some(2));

    let text = parsed.to_string();
    let reparsed = Sdp::parse(&text).expect("有効な SDP なのでパースに失敗しない想定");
    assert_eq!(reparsed.media[0].num_ports, Some(2));
}
