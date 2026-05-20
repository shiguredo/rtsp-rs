use crate::error::Error;
use crate::rtsp_header::RtspStatusCode;
use shiguredo_http11::HttpHead;

/// RTSP レスポンス
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtspResponse {
    pub version: String,
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RtspResponse {
    pub fn new(status: RtspStatusCode) -> Self {
        Self {
            version: "RTSP/1.0".to_string(),
            status_code: status.code(),
            reason_phrase: status.reason_phrase().to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn with_code(status_code: u16, reason_phrase: &str) -> Self {
        Self {
            version: "RTSP/1.0".to_string(),
            status_code,
            reason_phrase: reason_phrase.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn cseq(self, cseq: u32) -> Self {
        self.header("CSeq", &cseq.to_string())
    }

    pub fn session(self, session_id: &str) -> Self {
        self.header("Session", session_id)
    }

    pub fn transport(self, transport: &str) -> Self {
        self.header("Transport", transport)
    }

    pub fn public(self, methods: &str) -> Self {
        self.header("Public", methods)
    }

    pub fn content_type(self, content_type: &str) -> Self {
        self.header("Content-Type", content_type)
    }

    pub fn content_base(self, content_base: &str) -> Self {
        self.header("Content-Base", content_base)
    }

    pub fn rtp_info(self, rtp_info: &str) -> Self {
        self.header("RTP-Info", rtp_info)
    }

    pub fn range(self, range: &str) -> Self {
        self.header("Range", range)
    }

    pub fn server(self, server: &str) -> Self {
        self.header("Server", server)
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn get_cseq(&self) -> Option<u32> {
        self.get_header("CSeq").and_then(|v| v.parse().ok())
    }

    pub fn get_session(&self) -> Option<&str> {
        self.get_header("Session")
    }

    pub fn get_content_length(&self) -> Option<usize> {
        self.get_header("Content-Length")
            .and_then(|v| v.parse().ok())
    }

    pub fn status(&self) -> Option<RtspStatusCode> {
        RtspStatusCode::from_code(self.status_code)
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status_code)
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    /// HTTP レスポンスから変換
    pub fn from_http(response: shiguredo_http11::Response) -> Self {
        Self {
            version: response.version().to_string(),
            status_code: response.status_code(),
            reason_phrase: response.reason_phrase().to_string(),
            headers: response.headers().to_vec(),
            body: response.body_bytes().unwrap_or_default().to_vec(),
        }
    }

    /// HTTP レスポンスに変換
    pub fn to_http(&self) -> Result<shiguredo_http11::Response, shiguredo_http11::EncodeError> {
        let mut response = shiguredo_http11::Response::with_version(
            &self.version,
            self.status_code,
            &self.reason_phrase,
        )?;
        for (name, value) in &self.headers {
            response = response.header(name, value)?;
        }
        Ok(if self.body.is_empty() {
            response.body(Vec::new())
        } else {
            response.body(self.body.clone())
        })
    }

    /// バイト列にエンコード
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        Ok(shiguredo_http11::encode_response(&self.to_http()?)?)
    }
}

impl std::fmt::Display for RtspResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.version, self.status_code, self.reason_phrase
        )
    }
}
