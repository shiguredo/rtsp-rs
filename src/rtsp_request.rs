use crate::error::Error;
use crate::rtsp_method::RtspMethod;
use shiguredo_http11::HttpHead;

/// RTSP リクエスト
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtspRequest {
    pub method: RtspMethod,
    pub uri: String,
    pub version: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RtspRequest {
    pub fn new(method: RtspMethod, uri: &str) -> Self {
        Self {
            method,
            uri: uri.to_string(),
            version: "RTSP/1.0".to_string(),
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

    pub fn accept(self, accept: &str) -> Self {
        self.header("Accept", accept)
    }

    pub fn user_agent(self, user_agent: &str) -> Self {
        self.header("User-Agent", user_agent)
    }

    pub fn content_type(self, content_type: &str) -> Self {
        self.header("Content-Type", content_type)
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

    /// HTTP リクエストから変換
    pub fn from_http(request: shiguredo_http11::Request) -> Self {
        let method: RtspMethod = request
            .method()
            .parse()
            .unwrap_or_else(|e: std::convert::Infallible| match e {});
        Self {
            method,
            uri: request.uri().to_string(),
            version: request.version().to_string(),
            headers: request
                .headers()
                .iter()
                .map(|(n, v)| (n.to_string(), v.clone()))
                .collect(),
            body: request.body_bytes().unwrap_or_default().to_vec(),
        }
    }

    /// HTTP リクエストに変換
    pub fn to_http(&self) -> Result<shiguredo_http11::Request, shiguredo_http11::EncodeError> {
        let method = shiguredo_http11::Method::new(self.method.to_string())?;
        let mut request =
            shiguredo_http11::Request::with_version(method, &self.uri, &self.version)?;
        for (name, value) in &self.headers {
            let header_name = shiguredo_http11::HeaderName::new(name.as_str())?;
            request = request.header(header_name, value.as_str())?;
        }
        Ok(if self.body.is_empty() {
            request
        } else {
            request.body(self.body.clone())
        })
    }

    /// バイト列にエンコード
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        Ok(shiguredo_http11::encode_request(&self.to_http()?)?)
    }
}

impl std::fmt::Display for RtspRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.method, self.uri, self.version)
    }
}
