// RTP の公開 API に対する単体テスト

use shiguredo_rtsp::rtp::RtpPacket;

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
