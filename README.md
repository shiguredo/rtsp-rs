# rtsp-rs

## About Shiguredo's open source software

We will not respond to PRs or issues that have not been discussed on Discord. Also, Discord is only available in Japanese.

Please read <https://github.com/shiguredo/oss> before use.

## 時雨堂のオープンソースソフトウェアについて

利用前に <https://github.com/shiguredo/oss> をお読みください。

## 概要

Rust で実装された Sans I/O な RTSP 1.0 クライアントライブラリです。

## 特徴

- Sans I/O
  - <https://sans-io.readthedocs.io/index.html>
- RTP/RTCP パケットのパース/生成
- SDP (RFC 8866) のパース/生成
- Interleaved (RTP/RTCP over TCP) 対応
- Digest 認証 (RFC 2617 / RFC 7235 / RFC 9110)

## 使い方

```rust
use shiguredo_rtsp::RtspClientConnection;

let mut conn = RtspClientConnection::new();

// OPTIONS リクエストを送信
conn.send_options("rtsp://example.com/media")?;

// 送信バッファを取得して TCP で送信
let data = conn.send_buf();
// stream.write_all(data)...
let len = data.len();
conn.advance_send_buf(len);

// 受信データをフィード
// conn.feed_recv_buf(&received_data)?;

// イベントを処理
while let Some(event) = conn.next_event() {
    // event を処理...
}
```

### RTP パケット

```rust
use shiguredo_rtsp::rtp::{RtpHeader, RtpPacket};

// RTP パケットを作成してエンコード
let header = RtpHeader::new(96, 1234, 90000, 0x12345678);
let packet = RtpPacket::new(header, vec![0x01, 0x02, 0x03]);
let bytes = packet.build();

// RTP パケットをデコード
let decoded = RtpPacket::parse(&bytes).unwrap();
```

### RTCP パケット

```rust
use shiguredo_rtsp::rtcp::{RtcpPacket, RtcpSenderReport};

// RTCP パケットをデコード
// let packets = RtcpPacket::parse(&received_data)?;

// RTCP パケットをエンコード
// let bytes = RtcpPacket::build(&packets);
```

### Digest 認証 (RFC 2617)

```rust
use shiguredo_rtsp::auth::{DigestCredentials, build_authorization};
use shiguredo_http11::auth::DigestChallenge;

let credentials = DigestCredentials {
    username: "admin".to_string(),
    password: "secret".to_string(),
};

// サーバーからの WWW-Authenticate ヘッダーをパースする
let challenge = DigestChallenge::parse(www_authenticate_value).unwrap();

// Authorization ヘッダー値を生成する
let auth_value = build_authorization(&credentials, &challenge, "DESCRIBE", "rtsp://example.com/media");
```

## RTSP 1.0

このライブラリが対応している RTSP 1.0 の機能です。

### メソッド

- OPTIONS
- DESCRIBE
- ANNOUNCE
- SETUP
- PLAY
- PAUSE
- TEARDOWN
- GET_PARAMETER
- SET_PARAMETER
- RECORD

### ヘッダー

- CSeq
- Session (タイムアウト対応)
- Transport (RTP/AVP, RTP/AVP/TCP, unicast, interleaved, client_port, server_port, ssrc, mode)
- Range (NPT, SMPTE, Clock)
- RTP-Info (url, seq, rtptime)
- Content-Type / Content-Length
- Accept / User-Agent / Public

### トランスポート

- RTP/RTCP over TCP (Interleaved)
- RTP/RTCP over UDP

### Interleaved (RFC 2326 Section 10.12)

- `$` フレーミングによる RTP/RTCP の多重化
- 偶数チャネルを RTP、奇数チャネルを RTCP として自動判別
- フレームサイズ制限 (デフォルト: 64KB)

## RTP/RTCP (RFC 3550)

`shiguredo_rtsp` crate が提供する RTP/RTCP 機能です。

### RTP

- ヘッダーのパース/生成 (version, padding, extension, CSRC, marker, payload type, sequence number, timestamp, SSRC)
- 拡張ヘッダー対応
- パディング対応
- CSRC リスト対応

### RTCP

- Sender Report (SR, PT=200)
- Receiver Report (RR, PT=201)
- Source Description (SDES, PT=202)
  - CNAME, NAME, EMAIL, PHONE, LOC, TOOL, NOTE, PRIV
- Goodbye (BYE, PT=203)
- Application-Defined (APP, PT=204)
- Compound packet 対応
- Unknown packet type のパススルー

## SDP (RFC 8866)

- セッション記述のパース/生成
- メディア記述 (m= 行)
- 接続情報 (c= 行)
- 帯域幅情報 (b= 行)
- 属性 (a= 行)
- rtpmap / fmtp 属性

## 制限 (DoS 対策)

`RtspConnectionLimits` で各制限値をカスタマイズ可能です。

デフォルト値:

- 最大 Interleaved フレームサイズ: 64KB
- RTSP バージョン検証: 有効
- HTTP デコーダー制限: `shiguredo_http11` のデフォルト値に準拠

## サンプル

### rtsp_client

RTSP サーバーに接続してメディアストリームを受信するサンプルです。
H.264 / AAC の RTP デパケタイズ、MP4 録画、Digest 認証に対応しています。

```bash
# 基本
cargo run -p rtsp_client -- rtsp://127.0.0.1:8554/test

# MP4 録画
cargo run -p rtsp_client -- rtsp://127.0.0.1:8554/test output.mp4

# Digest 認証付き
cargo run -p rtsp_client -- rtsp://admin:password@192.168.1.100:554/stream1

# 映像表示を有効にする (shiguredo_openh264 + raw_player が必要)
cargo run -p rtsp_client --features video-display -- rtsp://127.0.0.1:8554/test
```

`video-display` feature を有効にすると [shiguredo_openh264](https://github.com/shiguredo/openh264-rs) で H.264 をデコードし、
[raw_player](https://github.com/shiguredo/raw-player-rs) (SDL2) でウィンドウに映像を表示します。
デフォルトでは無効のため、これらの依存は不要です。

## 規格書

このライブラリが準拠している RFC 一覧です。

- RFC 2326 - Real Time Streaming Protocol (RTSP)
  - <https://datatracker.ietf.org/doc/html/rfc2326>
- RFC 2617 - HTTP Authentication: Basic and Digest Access Authentication
  - <https://datatracker.ietf.org/doc/html/rfc2617>
- RFC 7235 - Hypertext Transfer Protocol (HTTP/1.1): Authentication
  - <https://datatracker.ietf.org/doc/html/rfc7235>
- RFC 9110 - HTTP Semantics
  - <https://datatracker.ietf.org/doc/html/rfc9110>
- RFC 3550 - RTP: A Transport Protocol for Real-Time Applications
  - <https://datatracker.ietf.org/doc/html/rfc3550>
- RFC 8866 - SDP: Session Description Protocol
  - <https://datatracker.ietf.org/doc/html/rfc8866>

## ライセンス

Apache License 2.0

```text
Copyright 2026-2026, Shiguredo Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
