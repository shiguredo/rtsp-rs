# RTSP クライアントサンプル

RTSP サーバーに接続してメディアストリームを受信するサンプルです。

## 機能

- TCP interleaved (RTP/AVP/TCP) によるストリーム受信
- SDP パースによる複数メディアトラック対応
- H.264 / AAC の RTP デパケタイズ
- MP4 録画 ([shiguredo_mp4](https://github.com/shiguredo/mp4-rs))
- Digest 認証
- 映像表示 (オプション、`video-display` feature)
  - [shiguredo_openh264](https://github.com/shiguredo/openh264-rs) による H.264 デコード
  - [raw_player](https://github.com/shiguredo/raw-player-rs) (SDL3) による映像表示
- RTP/RTCP パケットカウントのログ出力

## 対応 RTSP メソッド

- OPTIONS
- DESCRIBE
- SETUP
- PLAY
- TEARDOWN

## 引数

| 引数 | 必須 | 説明 |
|---|---|---|
| `<rtsp-url>` | 必須 | 接続先の RTSP URL |
| `[output.mp4]` | - | MP4 録画ファイルのパス |

## 実行方法

```bash
# 基本
cargo run -p rtsp_client -- rtsp://127.0.0.1:8554/test

# MP4 録画
cargo run -p rtsp_client -- rtsp://127.0.0.1:8554/test output.mp4

# Digest 認証付き
cargo run -p rtsp_client -- rtsp://admin:password@192.168.1.100:554/stream1

# 映像表示を有効にする (OpenH264 + SDL3 が必要)
cargo run -p rtsp_client --features video-display -- rtsp://127.0.0.1:8554/test
```
