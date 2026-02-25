use std::collections::{HashMap, VecDeque};

use crate::rtcp::RtcpPacket;
use crate::rtp::RtpPacket;

use crate::error::Error;
use crate::rtsp_connection::{
    RtspConnectionEvent, RtspConnectionLimits, RtspConnectionState, encode_interleaved_frame,
};
use crate::rtsp_method::RtspMethod;
use crate::rtsp_request::RtspRequest;
use crate::rtsp_response::RtspResponse;
use shiguredo_http11::{BodyKind, BodyProgress, Response};

/// RTSP クライアント接続 (sansio)
#[derive(Debug)]
pub struct RtspClientConnection {
    state: RtspConnectionState,
    http_decoder: shiguredo_http11::ResponseDecoder,
    recv_buf: Vec<u8>,
    send_buf: Vec<u8>,
    event_queue: VecDeque<RtspConnectionEvent>,
    session_id: Option<String>,
    cseq: u32,
    pending_methods: HashMap<u32, RtspMethod>,
    user_agent: String,
    limits: RtspConnectionLimits,
}

impl Default for RtspClientConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl RtspClientConnection {
    pub fn new() -> Self {
        Self {
            state: RtspConnectionState::Init,
            http_decoder: shiguredo_http11::ResponseDecoder::new(),
            recv_buf: Vec::new(),
            send_buf: Vec::new(),
            event_queue: VecDeque::new(),
            session_id: None,
            cseq: 0,
            pending_methods: HashMap::new(),
            user_agent: format!("shiguredo_rtsp/{}", env!("CARGO_PKG_VERSION")),
            limits: RtspConnectionLimits::default(),
        }
    }

    /// 制限付きで作成
    pub fn with_limits(limits: RtspConnectionLimits) -> Self {
        Self {
            state: RtspConnectionState::Init,
            http_decoder: shiguredo_http11::ResponseDecoder::with_limits(
                limits.http_limits.clone(),
            ),
            recv_buf: Vec::new(),
            send_buf: Vec::new(),
            event_queue: VecDeque::new(),
            session_id: None,
            cseq: 0,
            pending_methods: HashMap::new(),
            user_agent: format!("shiguredo_rtsp/{}", env!("CARGO_PKG_VERSION")),
            limits,
        }
    }

    /// 制限を取得
    pub fn limits(&self) -> &RtspConnectionLimits {
        &self.limits
    }

    /// 現在の状態を取得
    pub fn state(&self) -> RtspConnectionState {
        self.state
    }

    /// セッション ID を取得
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// セッション ID を設定
    pub fn set_session_id(&mut self, session_id: &str) {
        self.session_id = Some(session_id.to_string());
    }

    /// User-Agent を設定
    pub fn set_user_agent(&mut self, user_agent: &str) {
        self.user_agent = user_agent.to_string();
    }

    /// 次の CSeq を取得してインクリメント
    fn next_cseq(&mut self) -> u32 {
        self.cseq += 1;
        self.cseq
    }

    /// Interleaved フレームデータを処理してイベントキューに追加する
    fn process_interleaved_frame(&mut self, channel: u8, frame_data: Vec<u8>) {
        if channel.is_multiple_of(2) {
            // Even channel = RTP
            match RtpPacket::parse(&frame_data) {
                Ok(packet) => {
                    self.event_queue
                        .push_back(RtspConnectionEvent::RtpReceived { channel, packet });
                }
                Err(_) => {
                    self.event_queue
                        .push_back(RtspConnectionEvent::InterleavedData {
                            channel,
                            data: frame_data,
                        });
                }
            }
        } else {
            // Odd channel = RTCP
            match RtcpPacket::parse(&frame_data) {
                Ok(packets) => {
                    self.event_queue
                        .push_back(RtspConnectionEvent::RtcpReceived { channel, packets });
                }
                Err(_) => {
                    self.event_queue
                        .push_back(RtspConnectionEvent::InterleavedData {
                            channel,
                            data: frame_data,
                        });
                }
            }
        }
    }

    /// 受信データをフィード
    ///
    /// RTSP レスポンスと interleaved ($) データが同一 TCP セグメントで到着しても
    /// 正しく分離して処理する。部分フレームは内部バッファに保持される。
    pub fn feed_recv_buf(&mut self, data: &[u8]) -> Result<(), Error> {
        self.recv_buf.extend_from_slice(data);

        loop {
            if self.recv_buf.is_empty() {
                break;
            }

            if self.recv_buf[0] == b'$' {
                // Interleaved フレーム処理
                if self.recv_buf.len() < 4 {
                    // ヘッダーが揃っていない
                    break;
                }
                let channel = self.recv_buf[1];
                let length = u16::from_be_bytes([self.recv_buf[2], self.recv_buf[3]]) as usize;

                if length > self.limits.max_interleaved_frame_size {
                    return Err(Error::invalid_data(format!(
                        "interleaved frame size exceeds limit: {} > {}",
                        length, self.limits.max_interleaved_frame_size
                    )));
                }

                let total_len = 4 + length;
                if self.recv_buf.len() < total_len {
                    // フレームデータが揃っていない
                    break;
                }

                let frame_data = self.recv_buf[4..total_len].to_vec();
                self.recv_buf.drain(..total_len);

                self.process_interleaved_frame(channel, frame_data);
            } else {
                // RTSP メッセージ: recv_buf を HTTP デコーダーにフィード
                self.http_decoder.feed(&self.recv_buf)?;
                self.recv_buf.clear();

                while let Some(http_response) = self.decode_rtsp_response()? {
                    // RTSP バージョンを検証
                    if self.limits.validate_version && !http_response.version.starts_with("RTSP/") {
                        return Err(Error::invalid_data(format!(
                            "invalid RTSP version: {}",
                            http_response.version
                        )));
                    }

                    let response = RtspResponse::from_http(http_response);

                    // セッション ID を更新
                    if let Some(session) = response.get_session() {
                        self.session_id =
                            Some(session.split(';').next().unwrap_or(session).to_string());
                    }

                    // リダイレクト (3xx) を検出
                    if response.is_redirect()
                        && let Some(location) = response.get_header("Location")
                    {
                        self.event_queue.push_back(RtspConnectionEvent::Redirect {
                            location: location.to_string(),
                        });
                    }

                    // 200 OK レスポンスの場合、CSeq でメソッドを特定して状態遷移
                    if response.is_success()
                        && let Some(cseq) = response.get_cseq()
                        && let Some(method) = self.pending_methods.remove(&cseq)
                    {
                        // RFC 2326 Appendix A.1 Client State Machine に基づく状態遷移
                        let new_state = match method {
                            RtspMethod::Setup => match self.state {
                                // Playing/Recording 中の SETUP は状態を維持する (transport 変更)
                                RtspConnectionState::Playing => Some(RtspConnectionState::Playing),
                                RtspConnectionState::Recording => {
                                    Some(RtspConnectionState::Recording)
                                }
                                _ => Some(RtspConnectionState::Ready),
                            },
                            RtspMethod::Play => Some(RtspConnectionState::Playing),
                            RtspMethod::Record => Some(RtspConnectionState::Recording),
                            RtspMethod::Pause => Some(RtspConnectionState::Ready),
                            RtspMethod::Teardown => Some(RtspConnectionState::Disconnected),
                            _ => None,
                        };
                        if let Some(state) = new_state {
                            self.set_state(state);
                        }
                    }

                    self.event_queue
                        .push_back(RtspConnectionEvent::ResponseReceived(response));
                }

                // デコーダーの残留データを確認
                let remaining = self.http_decoder.remaining();
                if !remaining.is_empty() && remaining[0] == b'$' {
                    // $ データをデコーダーから救出して recv_buf に戻す
                    self.recv_buf = remaining.to_vec();
                    self.http_decoder.reset();
                    continue;
                }

                // 部分 RTSP メッセージ待ち
                break;
            }
        }

        Ok(())
    }

    /// 送信バッファを取得
    pub fn send_buf(&self) -> &[u8] {
        &self.send_buf
    }

    /// 送信バッファを進める
    pub fn advance_send_buf(&mut self, n: usize) {
        self.send_buf.drain(..n.min(self.send_buf.len()));
    }

    /// 次のイベントを取得
    pub fn next_event(&mut self) -> Option<RtspConnectionEvent> {
        self.event_queue.pop_front()
    }

    /// リクエストを送信
    pub fn send_request(&mut self, mut request: RtspRequest) -> Result<(), Error> {
        // Add CSeq if not present
        if request.get_cseq().is_none() {
            let cseq = self.next_cseq();
            request = request.cseq(cseq);
        }

        // Add User-Agent
        if request.get_header("User-Agent").is_none() {
            request = request.user_agent(&self.user_agent);
        }

        // Add Session if present
        if request.get_session().is_none()
            && let Some(ref session_id) = self.session_id
        {
            request = request.session(session_id);
        }

        // CSeq→Method マッピングを記録 (状態遷移の自動化用)
        if let Some(cseq) = request.get_cseq() {
            self.pending_methods.insert(cseq, request.method.clone());
        }

        let encoded = request.encode()?;
        self.send_buf.extend_from_slice(&encoded);
        Ok(())
    }

    /// OPTIONS リクエストを送信
    pub fn send_options(&mut self, uri: &str) -> Result<(), Error> {
        let request = RtspRequest::new(RtspMethod::Options, uri);
        self.send_request(request)
    }

    /// DESCRIBE リクエストを送信
    pub fn send_describe(&mut self, uri: &str) -> Result<(), Error> {
        let request = RtspRequest::new(RtspMethod::Describe, uri).accept("application/sdp");
        self.send_request(request)
    }

    /// SETUP リクエストを送信 (Interleaved)
    pub fn send_setup_interleaved(
        &mut self,
        uri: &str,
        rtp_channel: u8,
        rtcp_channel: u8,
    ) -> Result<(), Error> {
        let transport = format!(
            "RTP/AVP/TCP;unicast;interleaved={}-{}",
            rtp_channel, rtcp_channel
        );
        let request = RtspRequest::new(RtspMethod::Setup, uri).transport(&transport);
        self.send_request(request)
    }

    /// SETUP リクエストを送信 (UDP)
    pub fn send_setup_udp(
        &mut self,
        uri: &str,
        client_rtp_port: u16,
        client_rtcp_port: u16,
    ) -> Result<(), Error> {
        let transport = format!(
            "RTP/AVP;unicast;client_port={}-{}",
            client_rtp_port, client_rtcp_port
        );
        let request = RtspRequest::new(RtspMethod::Setup, uri).transport(&transport);
        self.send_request(request)
    }

    /// PLAY リクエストを送信
    pub fn send_play(&mut self, uri: &str, range: Option<&str>) -> Result<(), Error> {
        let mut request = RtspRequest::new(RtspMethod::Play, uri);
        if let Some(range) = range {
            request = request.header("Range", range);
        }
        self.send_request(request)
    }

    /// PAUSE リクエストを送信
    pub fn send_pause(&mut self, uri: &str) -> Result<(), Error> {
        let request = RtspRequest::new(RtspMethod::Pause, uri);
        self.send_request(request)
    }

    /// TEARDOWN リクエストを送信
    pub fn send_teardown(&mut self, uri: &str) -> Result<(), Error> {
        let request = RtspRequest::new(RtspMethod::Teardown, uri);
        self.send_request(request)
    }

    /// GET_PARAMETER リクエストを送信
    pub fn send_get_parameter(&mut self, uri: &str) -> Result<(), Error> {
        let request = RtspRequest::new(RtspMethod::GetParameter, uri);
        self.send_request(request)
    }

    /// SET_PARAMETER リクエストを送信
    pub fn send_set_parameter(
        &mut self,
        uri: &str,
        body: &[u8],
        content_type: &str,
    ) -> Result<(), Error> {
        let request = RtspRequest::new(RtspMethod::SetParameter, uri)
            .content_type(content_type)
            .body(body.to_vec());
        self.send_request(request)
    }

    /// ANNOUNCE リクエストを送信 (SDP をサーバーに送信)
    pub fn send_announce(&mut self, uri: &str, sdp: &str) -> Result<(), Error> {
        let request = RtspRequest::new(RtspMethod::Announce, uri)
            .content_type("application/sdp")
            .body(sdp.as_bytes().to_vec());
        self.send_request(request)
    }

    /// RECORD リクエストを送信
    pub fn send_record(&mut self, uri: &str, range: Option<&str>) -> Result<(), Error> {
        let mut request = RtspRequest::new(RtspMethod::Record, uri);
        if let Some(range) = range {
            request = request.header("Range", range);
        }
        self.send_request(request)
    }

    /// Interleaved データを送信 (RTP/RTCP over TCP)
    pub fn send_interleaved(&mut self, channel: u8, data: &[u8]) {
        let frame = encode_interleaved_frame(channel, data);
        self.send_buf.extend_from_slice(&frame);
    }

    /// RTCP パケットを送信 (Interleaved)
    pub fn send_rtcp(&mut self, channel: u8, packets: &[RtcpPacket]) {
        let data = RtcpPacket::build(packets);
        self.send_interleaved(channel, &data);
    }

    /// RTSP レスポンスをデコードする
    ///
    /// HTTP デコーダーの `decode()` は Content-Length も Transfer-Encoding もない
    /// レスポンスを close-delimited (接続終了まで待つ) として扱うが、
    /// RTSP (RFC 2326) ではこの場合ボディなしが正しい。
    ///
    /// `decode_headers()` でヘッダーを取得し、`BodyKind` に応じて
    /// RTSP セマンティクスでボディを処理する。
    fn decode_rtsp_response(&mut self) -> Result<Option<Response>, Error> {
        let (head, body_kind) = match self.http_decoder.decode_headers()? {
            Some(result) => result,
            None => return Ok(None),
        };

        let body = match body_kind {
            BodyKind::None => Vec::new(),
            BodyKind::ContentLength(_) | BodyKind::Chunked => {
                let mut body = Vec::new();
                loop {
                    if let Some(data) = self.http_decoder.peek_body() {
                        body.extend_from_slice(data);
                        let len = data.len();
                        match self.http_decoder.consume_body(len)? {
                            BodyProgress::Complete { .. } => break,
                            BodyProgress::Continue => continue,
                        }
                    }
                    match self.http_decoder.progress()? {
                        BodyProgress::Complete { .. } => break,
                        BodyProgress::Continue => {
                            if self.http_decoder.peek_body().is_some() {
                                continue;
                            }
                            // データ不足: ヘッダーはデコード済みだが
                            // ボディが不完全な状態は RTSP では発生しにくいが、
                            // 安全のため空ボディで返す
                            break;
                        }
                    }
                }
                body
            }
            BodyKind::CloseDelimited => {
                // RTSP ではボディなしとして扱う
                // HTTP デコーダーに EOF を通知してリセットする
                self.http_decoder.mark_eof();
                Vec::new()
            }
            BodyKind::Tunnel => Vec::new(),
        };

        // デコーダーをリセットして次のレスポンスに備える
        // (close-delimited でボディなしとして扱った場合、
        //  デコーダーの状態が Complete になっているので reset が必要)
        if matches!(body_kind, BodyKind::CloseDelimited) {
            // close-delimited の場合、remaining() に後続データがある可能性がある
            let remaining = self.http_decoder.remaining().to_vec();
            self.http_decoder.reset();
            if !remaining.is_empty() {
                self.http_decoder.feed(&remaining)?;
            }
        }

        Ok(Some(Response {
            version: head.version,
            status_code: head.status_code,
            reason_phrase: head.reason_phrase,
            headers: head.headers,
            body,
            omit_body: false,
        }))
    }

    /// 状態を設定
    pub fn set_state(&mut self, state: RtspConnectionState) {
        if self.state != state {
            self.state = state;
            self.event_queue
                .push_back(RtspConnectionEvent::StateChanged(state));
        }
    }

    /// 最後の CSeq を取得
    pub fn last_cseq(&self) -> u32 {
        self.cseq
    }
}
