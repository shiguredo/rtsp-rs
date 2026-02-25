use crate::error::Error;
use crate::rtsp_request::RtspRequest;
use crate::rtsp_response::RtspResponse;

/// RTSP メッセージ (リクエストまたはレスポンス)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtspMessage {
    Request(RtspRequest),
    Response(RtspResponse),
}

impl RtspMessage {
    pub fn is_request(&self) -> bool {
        matches!(self, RtspMessage::Request(_))
    }

    pub fn is_response(&self) -> bool {
        matches!(self, RtspMessage::Response(_))
    }

    pub fn as_request(&self) -> Option<&RtspRequest> {
        match self {
            RtspMessage::Request(req) => Some(req),
            _ => None,
        }
    }

    pub fn as_response(&self) -> Option<&RtspResponse> {
        match self {
            RtspMessage::Response(res) => Some(res),
            _ => None,
        }
    }

    pub fn into_request(self) -> Option<RtspRequest> {
        match self {
            RtspMessage::Request(req) => Some(req),
            _ => None,
        }
    }

    pub fn into_response(self) -> Option<RtspResponse> {
        match self {
            RtspMessage::Response(res) => Some(res),
            _ => None,
        }
    }

    pub fn get_cseq(&self) -> Option<u32> {
        match self {
            RtspMessage::Request(req) => req.get_cseq(),
            RtspMessage::Response(res) => res.get_cseq(),
        }
    }

    pub fn get_session(&self) -> Option<&str> {
        match self {
            RtspMessage::Request(req) => req.get_session(),
            RtspMessage::Response(res) => res.get_session(),
        }
    }

    /// バイト列にエンコード
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        match self {
            RtspMessage::Request(req) => req.encode(),
            RtspMessage::Response(res) => res.encode(),
        }
    }
}

impl From<RtspRequest> for RtspMessage {
    fn from(req: RtspRequest) -> Self {
        RtspMessage::Request(req)
    }
}

impl From<RtspResponse> for RtspMessage {
    fn from(res: RtspResponse) -> Self {
        RtspMessage::Response(res)
    }
}

impl std::fmt::Display for RtspMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RtspMessage::Request(req) => write!(f, "{}", req),
            RtspMessage::Response(res) => write!(f, "{}", res),
        }
    }
}
