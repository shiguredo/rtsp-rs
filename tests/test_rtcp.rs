// RTCP の公開 API に対する単体テスト

use shiguredo_rtsp::rtcp::RtcpPacket;

/// 不正なデータのパースが失敗することを確認
#[test]
fn test_rtcp_parse_invalid_data() {
    // データが短すぎる場合は空のパケットリストとして解釈される
    assert!(
        RtcpPacket::parse(&[])
            .expect("空入力はパースに失敗しない想定")
            .is_empty()
    );
    assert!(
        RtcpPacket::parse(&[0, 0, 0])
            .expect("短い入力はパースに失敗しない想定")
            .is_empty()
    );

    // バージョンが違う
    let mut invalid_version = vec![0; 8];
    invalid_version[0] = 0b0000_0000; // version = 0
    invalid_version[1] = 200; // SR
    invalid_version[2] = 0;
    invalid_version[3] = 1; // length = 1 word
    assert!(RtcpPacket::parse(&invalid_version).is_err());
}
