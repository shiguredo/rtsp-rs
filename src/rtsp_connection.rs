use crate::rtcp::RtcpPacket;
use crate::rtp::RtpPacket;

use crate::rtsp_method::RtspMethod;
use crate::rtsp_request::RtspRequest;
use crate::rtsp_response::RtspResponse;

/// RTSP 接続の制限設定
#[derive(Debug, Clone)]
pub struct RtspConnectionLimits {
    /// HTTP デコーダーの制限
    pub http_limits: shiguredo_http11::DecoderLimits,
    /// 最大 Interleaved フレームサイズ (デフォルト: 64KB)
    pub max_interleaved_frame_size: usize,
    /// RTSP バージョンを検証するか (デフォルト: true)
    pub validate_version: bool,
}

impl Default for RtspConnectionLimits {
    fn default() -> Self {
        Self {
            http_limits: shiguredo_http11::DecoderLimits::default(),
            max_interleaved_frame_size: 64 * 1024, // 64KB
            validate_version: true,
        }
    }
}

/// RTSP 接続状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtspConnectionState {
    /// 初期状態
    Init,
    /// セットアップ済み (SETUP 完了)
    Ready,
    /// 再生中 (PLAY 完了)
    Playing,
    /// 録画中 (RECORD 完了)
    Recording,
    /// 切断済み
    Disconnected,
}

/// RTSP 接続イベント
#[derive(Debug, Clone)]
pub enum RtspConnectionEvent {
    /// リクエストを受信
    RequestReceived(RtspRequest),
    /// レスポンスを受信
    ResponseReceived(RtspResponse),
    /// RTP パケットを受信 (Interleaved)
    RtpReceived { channel: u8, packet: RtpPacket },
    /// RTCP パケットを受信 (Interleaved)
    RtcpReceived {
        channel: u8,
        packets: Vec<RtcpPacket>,
    },
    /// Interleaved データを受信 (raw)
    InterleavedData { channel: u8, data: Vec<u8> },
    /// 状態変更
    StateChanged(RtspConnectionState),
    /// リダイレクト要求 (3xx レスポンス)
    Redirect { location: String },
    /// エラー発生
    Error(String),
}

/// RTSP セッション情報
#[derive(Debug, Clone, Default)]
pub struct RtspSession {
    /// セッション ID
    pub id: Option<String>,
    /// タイムアウト (秒)
    pub timeout: Option<u32>,
}

impl RtspSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(id: &str) -> Self {
        Self {
            id: Some(id.to_string()),
            timeout: None,
        }
    }

    /// Session ヘッダーをパース
    pub fn parse(header_value: &str) -> Self {
        let mut session = Self::new();
        let parts: Vec<&str> = header_value.split(';').collect();

        if !parts.is_empty() {
            session.id = Some(parts[0].trim().to_string());
        }

        for part in &parts[1..] {
            let part = part.trim();
            let lower = part.to_ascii_lowercase();
            if let Some(timeout_str) = lower.strip_prefix("timeout=")
                && let Ok(timeout) = timeout_str.parse()
            {
                session.timeout = Some(timeout);
            }
        }

        session
    }

    /// Session ヘッダーに変換
    pub fn to_header(&self) -> Option<String> {
        self.id.as_ref().map(|id| {
            if let Some(timeout) = self.timeout {
                format!("{};timeout={}", id, timeout)
            } else {
                id.clone()
            }
        })
    }
}

/// RTSP Transport 情報 (RFC 2326 Section 12.39)
#[derive(Debug, Clone, Default)]
pub struct RtspTransport {
    /// プロトコル (RTP/AVP, RTP/AVP/TCP, etc.)
    pub protocol: String,
    /// unicast or multicast
    pub unicast: bool,
    /// Interleaved channels
    pub interleaved: Option<(u8, u8)>,
    /// Client port (RTP, RTCP)
    pub client_port: Option<(u16, u16)>,
    /// Server port (RTP, RTCP)
    pub server_port: Option<(u16, u16)>,
    /// SSRC
    pub ssrc: Option<u32>,
    /// mode (PLAY, RECORD)
    pub mode: Option<String>,
    /// destination アドレス
    pub destination: Option<String>,
    /// source アドレス
    pub source: Option<String>,
    /// TTL (multicast)
    pub ttl: Option<u8>,
    /// layers (multicast)
    pub layers: Option<u32>,
    /// port (RTP, RTCP) - multicast 用
    pub port: Option<(u16, u16)>,
    /// append フラグ
    pub append: bool,
}

impl RtspTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rtp_avp_tcp_interleaved(rtp_channel: u8, rtcp_channel: u8) -> Self {
        Self {
            protocol: "RTP/AVP/TCP".to_string(),
            unicast: true,
            interleaved: Some((rtp_channel, rtcp_channel)),
            ..Default::default()
        }
    }

    pub fn rtp_avp_udp(client_rtp_port: u16, client_rtcp_port: u16) -> Self {
        Self {
            protocol: "RTP/AVP".to_string(),
            unicast: true,
            client_port: Some((client_rtp_port, client_rtcp_port)),
            ..Default::default()
        }
    }

    /// Transport ヘッダーをパース
    pub fn parse(header_value: &str) -> Self {
        let mut transport = Self::new();
        let parts: Vec<&str> = header_value.split(';').collect();

        if !parts.is_empty() {
            transport.protocol = parts[0].trim().to_string();
        }

        for part in &parts[1..] {
            let part = part.trim();
            if part == "unicast" {
                transport.unicast = true;
            } else if part == "multicast" {
                transport.unicast = false;
            } else if part == "append" {
                transport.append = true;
            } else if let Some(value) = part.strip_prefix("interleaved=") {
                // RFC 2326 Section 12.39: channel [ "-" channel ]
                transport.interleaved = parse_channel_pair(value);
            } else if let Some(value) = part.strip_prefix("client_port=") {
                // RFC 2326 Section 12.39: port [ "-" port ]
                transport.client_port = parse_port_pair(value);
            } else if let Some(value) = part.strip_prefix("server_port=") {
                transport.server_port = parse_port_pair(value);
            } else if let Some(value) = part.strip_prefix("port=") {
                transport.port = parse_port_pair(value);
            } else if let Some(value) = part.strip_prefix("ssrc=") {
                if let Ok(ssrc) = u32::from_str_radix(value, 16) {
                    transport.ssrc = Some(ssrc);
                }
            } else if let Some(value) = part.strip_prefix("mode=") {
                transport.mode = Some(value.trim_matches('"').to_string());
            } else if part == "destination" {
                // RFC 2326 Section 12.39: "destination" [ "=" address ]
                // 値なしの場合は空文字列で表現
                transport.destination = Some(String::new());
            } else if let Some(value) = part.strip_prefix("destination=") {
                transport.destination = Some(value.to_string());
            } else if let Some(value) = part.strip_prefix("source=") {
                transport.source = Some(value.to_string());
            } else if let Some(value) = part.strip_prefix("ttl=")
                && let Ok(ttl) = value.parse()
            {
                transport.ttl = Some(ttl);
            } else if let Some(value) = part.strip_prefix("layers=")
                && let Ok(layers) = value.parse()
            {
                transport.layers = Some(layers);
            }
        }

        transport
    }

    /// 複数の Transport ヘッダーをパース (カンマ区切り)
    pub fn parse_multiple(header_value: &str) -> Vec<Self> {
        header_value
            .split(',')
            .map(|s| Self::parse(s.trim()))
            .collect()
    }

    /// Transport ヘッダーに変換
    pub fn to_header(&self) -> String {
        let mut parts = vec![self.protocol.clone()];

        if self.unicast {
            parts.push("unicast".to_string());
        } else {
            parts.push("multicast".to_string());
        }

        if let Some(ref dest) = self.destination {
            parts.push(format!("destination={}", dest));
        }

        if let Some(ref src) = self.source {
            parts.push(format!("source={}", src));
        }

        if let Some((a, b)) = self.interleaved {
            parts.push(format!("interleaved={}-{}", a, b));
        }

        if self.append {
            parts.push("append".to_string());
        }

        if let Some(ttl) = self.ttl {
            parts.push(format!("ttl={}", ttl));
        }

        if let Some(layers) = self.layers {
            parts.push(format!("layers={}", layers));
        }

        if let Some((a, b)) = self.port {
            parts.push(format!("port={}-{}", a, b));
        }

        if let Some((a, b)) = self.client_port {
            parts.push(format!("client_port={}-{}", a, b));
        }

        if let Some((a, b)) = self.server_port {
            parts.push(format!("server_port={}-{}", a, b));
        }

        if let Some(ssrc) = self.ssrc {
            parts.push(format!("ssrc={:08X}", ssrc));
        }

        if let Some(ref mode) = self.mode {
            parts.push(format!("mode=\"{}\"", mode));
        }

        parts.join(";")
    }
}

/// ポートペアをパースする (RFC 2326 Section 12.39)
///
/// `port [ "-" port ]` 形式。単一値の場合は隣接ポートを推定する。
fn parse_port_pair(value: &str) -> Option<(u16, u16)> {
    if let Some((a, b)) = value.split_once('-') {
        if let (Ok(a), Ok(b)) = (a.parse::<u16>(), b.parse::<u16>()) {
            return Some((a, b));
        }
    } else if let Ok(a) = value.parse::<u16>() {
        // 単一ポート: RTP ポートのみ指定、RTCP は RTP + 1
        return Some((a, a.saturating_add(1)));
    }
    None
}

/// チャネルペアをパースする (RFC 2326 Section 12.39)
///
/// `channel [ "-" channel ]` 形式。単一値の場合は隣接チャネルを推定する。
fn parse_channel_pair(value: &str) -> Option<(u8, u8)> {
    if let Some((a, b)) = value.split_once('-') {
        if let (Ok(a), Ok(b)) = (a.parse::<u8>(), b.parse::<u8>()) {
            return Some((a, b));
        }
    } else if let Ok(a) = value.parse::<u8>() {
        return Some((a, a.saturating_add(1)));
    }
    None
}

/// Interleaved frame を検出する
pub fn parse_interleaved_frame(data: &[u8]) -> Option<(u8, u16, usize)> {
    // $ (1 byte) + channel (1 byte) + length (2 bytes) + data
    if data.len() >= 4 && data[0] == b'$' {
        let channel = data[1];
        let length = u16::from_be_bytes([data[2], data[3]]) as usize;
        Some((channel, length as u16, 4 + length))
    } else {
        None
    }
}

/// Interleaved frame をエンコード
pub fn encode_interleaved_frame(channel: u8, data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + data.len());
    buf.push(b'$');
    buf.push(channel);
    buf.extend_from_slice(&(data.len() as u16).to_be_bytes());
    buf.extend_from_slice(data);
    buf
}

/// 標準 RTSP メソッドのリスト
pub fn supported_methods() -> Vec<RtspMethod> {
    vec![
        RtspMethod::Options,
        RtspMethod::Describe,
        RtspMethod::Announce,
        RtspMethod::Setup,
        RtspMethod::Play,
        RtspMethod::Pause,
        RtspMethod::Teardown,
        RtspMethod::GetParameter,
        RtspMethod::SetParameter,
        RtspMethod::Redirect,
        RtspMethod::Record,
    ]
}

/// Public ヘッダーの値を生成
pub fn public_header_value() -> String {
    supported_methods()
        .iter()
        .map(|m| m.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
