use std::fmt;

/// RTSP ヘッダー名 (RFC 2326 Section 12)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RtspHeader {
    /// Accept
    Accept,
    /// Accept-Encoding
    AcceptEncoding,
    /// Accept-Language
    AcceptLanguage,
    /// Allow
    Allow,
    /// Authorization
    Authorization,
    /// Bandwidth
    Bandwidth,
    /// Blocksize
    Blocksize,
    /// Cache-Control
    CacheControl,
    /// Conference
    Conference,
    /// Connection
    Connection,
    /// Content-Base
    ContentBase,
    /// Content-Encoding
    ContentEncoding,
    /// Content-Language
    ContentLanguage,
    /// Content-Length
    ContentLength,
    /// Content-Location
    ContentLocation,
    /// Content-Type
    ContentType,
    /// CSeq (必須)
    CSeq,
    /// Date
    Date,
    /// Expires
    Expires,
    /// From
    From,
    /// Host
    Host,
    /// If-Match
    IfMatch,
    /// If-Modified-Since
    IfModifiedSince,
    /// Last-Modified
    LastModified,
    /// Location
    Location,
    /// Proxy-Authenticate
    ProxyAuthenticate,
    /// Proxy-Require
    ProxyRequire,
    /// Public
    Public,
    /// Range
    Range,
    /// Referer
    Referer,
    /// Retry-After
    RetryAfter,
    /// Require
    Require,
    /// RTP-Info
    RtpInfo,
    /// Scale
    Scale,
    /// Speed
    Speed,
    /// Server
    Server,
    /// Session
    Session,
    /// Timestamp
    Timestamp,
    /// Transport
    Transport,
    /// Unsupported
    Unsupported,
    /// User-Agent
    UserAgent,
    /// Vary
    Vary,
    /// Via
    Via,
    /// WWW-Authenticate
    WwwAuthenticate,
    /// 拡張ヘッダー
    Extension(String),
}

impl RtspHeader {
    pub fn as_str(&self) -> &str {
        match self {
            RtspHeader::Accept => "Accept",
            RtspHeader::AcceptEncoding => "Accept-Encoding",
            RtspHeader::AcceptLanguage => "Accept-Language",
            RtspHeader::Allow => "Allow",
            RtspHeader::Authorization => "Authorization",
            RtspHeader::Bandwidth => "Bandwidth",
            RtspHeader::Blocksize => "Blocksize",
            RtspHeader::CacheControl => "Cache-Control",
            RtspHeader::Conference => "Conference",
            RtspHeader::Connection => "Connection",
            RtspHeader::ContentBase => "Content-Base",
            RtspHeader::ContentEncoding => "Content-Encoding",
            RtspHeader::ContentLanguage => "Content-Language",
            RtspHeader::ContentLength => "Content-Length",
            RtspHeader::ContentLocation => "Content-Location",
            RtspHeader::ContentType => "Content-Type",
            RtspHeader::CSeq => "CSeq",
            RtspHeader::Date => "Date",
            RtspHeader::Expires => "Expires",
            RtspHeader::From => "From",
            RtspHeader::Host => "Host",
            RtspHeader::IfMatch => "If-Match",
            RtspHeader::IfModifiedSince => "If-Modified-Since",
            RtspHeader::LastModified => "Last-Modified",
            RtspHeader::Location => "Location",
            RtspHeader::ProxyAuthenticate => "Proxy-Authenticate",
            RtspHeader::ProxyRequire => "Proxy-Require",
            RtspHeader::Public => "Public",
            RtspHeader::Range => "Range",
            RtspHeader::Referer => "Referer",
            RtspHeader::RetryAfter => "Retry-After",
            RtspHeader::Require => "Require",
            RtspHeader::RtpInfo => "RTP-Info",
            RtspHeader::Scale => "Scale",
            RtspHeader::Speed => "Speed",
            RtspHeader::Server => "Server",
            RtspHeader::Session => "Session",
            RtspHeader::Timestamp => "Timestamp",
            RtspHeader::Transport => "Transport",
            RtspHeader::Unsupported => "Unsupported",
            RtspHeader::UserAgent => "User-Agent",
            RtspHeader::Vary => "Vary",
            RtspHeader::Via => "Via",
            RtspHeader::WwwAuthenticate => "WWW-Authenticate",
            RtspHeader::Extension(s) => s,
        }
    }

    pub fn parse_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "accept" => RtspHeader::Accept,
            "accept-encoding" => RtspHeader::AcceptEncoding,
            "accept-language" => RtspHeader::AcceptLanguage,
            "allow" => RtspHeader::Allow,
            "authorization" => RtspHeader::Authorization,
            "bandwidth" => RtspHeader::Bandwidth,
            "blocksize" => RtspHeader::Blocksize,
            "cache-control" => RtspHeader::CacheControl,
            "conference" => RtspHeader::Conference,
            "connection" => RtspHeader::Connection,
            "content-base" => RtspHeader::ContentBase,
            "content-encoding" => RtspHeader::ContentEncoding,
            "content-language" => RtspHeader::ContentLanguage,
            "content-length" => RtspHeader::ContentLength,
            "content-location" => RtspHeader::ContentLocation,
            "content-type" => RtspHeader::ContentType,
            "cseq" => RtspHeader::CSeq,
            "date" => RtspHeader::Date,
            "expires" => RtspHeader::Expires,
            "from" => RtspHeader::From,
            "host" => RtspHeader::Host,
            "if-match" => RtspHeader::IfMatch,
            "if-modified-since" => RtspHeader::IfModifiedSince,
            "last-modified" => RtspHeader::LastModified,
            "location" => RtspHeader::Location,
            "proxy-authenticate" => RtspHeader::ProxyAuthenticate,
            "proxy-require" => RtspHeader::ProxyRequire,
            "public" => RtspHeader::Public,
            "range" => RtspHeader::Range,
            "referer" => RtspHeader::Referer,
            "retry-after" => RtspHeader::RetryAfter,
            "require" => RtspHeader::Require,
            "rtp-info" => RtspHeader::RtpInfo,
            "scale" => RtspHeader::Scale,
            "speed" => RtspHeader::Speed,
            "server" => RtspHeader::Server,
            "session" => RtspHeader::Session,
            "timestamp" => RtspHeader::Timestamp,
            "transport" => RtspHeader::Transport,
            "unsupported" => RtspHeader::Unsupported,
            "user-agent" => RtspHeader::UserAgent,
            "vary" => RtspHeader::Vary,
            "via" => RtspHeader::Via,
            "www-authenticate" => RtspHeader::WwwAuthenticate,
            _ => RtspHeader::Extension(s.to_string()),
        }
    }
}

impl fmt::Display for RtspHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// RTSP ステータスコード (RFC 2326 Section 7.1.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum RtspStatusCode {
    // 1xx Informational
    Continue = 100,

    // 2xx Success
    Ok = 200,
    Created = 201,
    LowOnStorageSpace = 250,

    // 3xx Redirection
    MultipleChoices = 300,
    MovedPermanently = 301,
    MovedTemporarily = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,

    // 4xx Client Error
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    RequestEntityTooLarge = 413,
    RequestUriTooLarge = 414,
    UnsupportedMediaType = 415,
    ParameterNotUnderstood = 451,
    ConferenceNotFound = 452,
    NotEnoughBandwidth = 453,
    SessionNotFound = 454,
    MethodNotValidInThisState = 455,
    HeaderFieldNotValidForResource = 456,
    InvalidRange = 457,
    ParameterIsReadOnly = 458,
    AggregateOperationNotAllowed = 459,
    OnlyAggregateOperationAllowed = 460,
    UnsupportedTransport = 461,
    DestinationUnreachable = 462,

    // 5xx Server Error
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    RtspVersionNotSupported = 505,
    OptionNotSupported = 551,
}

impl RtspStatusCode {
    pub fn code(&self) -> u16 {
        *self as u16
    }

    pub fn reason_phrase(&self) -> &'static str {
        match self {
            RtspStatusCode::Continue => "Continue",
            RtspStatusCode::Ok => "OK",
            RtspStatusCode::Created => "Created",
            RtspStatusCode::LowOnStorageSpace => "Low on Storage Space",
            RtspStatusCode::MultipleChoices => "Multiple Choices",
            RtspStatusCode::MovedPermanently => "Moved Permanently",
            RtspStatusCode::MovedTemporarily => "Moved Temporarily",
            RtspStatusCode::SeeOther => "See Other",
            RtspStatusCode::NotModified => "Not Modified",
            RtspStatusCode::UseProxy => "Use Proxy",
            RtspStatusCode::BadRequest => "Bad Request",
            RtspStatusCode::Unauthorized => "Unauthorized",
            RtspStatusCode::PaymentRequired => "Payment Required",
            RtspStatusCode::Forbidden => "Forbidden",
            RtspStatusCode::NotFound => "Not Found",
            RtspStatusCode::MethodNotAllowed => "Method Not Allowed",
            RtspStatusCode::NotAcceptable => "Not Acceptable",
            RtspStatusCode::ProxyAuthenticationRequired => "Proxy Authentication Required",
            RtspStatusCode::RequestTimeout => "Request Timeout",
            RtspStatusCode::Gone => "Gone",
            RtspStatusCode::LengthRequired => "Length Required",
            RtspStatusCode::PreconditionFailed => "Precondition Failed",
            RtspStatusCode::RequestEntityTooLarge => "Request Entity Too Large",
            RtspStatusCode::RequestUriTooLarge => "Request-URI Too Large",
            RtspStatusCode::UnsupportedMediaType => "Unsupported Media Type",
            RtspStatusCode::ParameterNotUnderstood => "Parameter Not Understood",
            RtspStatusCode::ConferenceNotFound => "Conference Not Found",
            RtspStatusCode::NotEnoughBandwidth => "Not Enough Bandwidth",
            RtspStatusCode::SessionNotFound => "Session Not Found",
            RtspStatusCode::MethodNotValidInThisState => "Method Not Valid in This State",
            RtspStatusCode::HeaderFieldNotValidForResource => "Header Field Not Valid for Resource",
            RtspStatusCode::InvalidRange => "Invalid Range",
            RtspStatusCode::ParameterIsReadOnly => "Parameter Is Read-Only",
            RtspStatusCode::AggregateOperationNotAllowed => "Aggregate Operation Not Allowed",
            RtspStatusCode::OnlyAggregateOperationAllowed => "Only Aggregate Operation Allowed",
            RtspStatusCode::UnsupportedTransport => "Unsupported Transport",
            RtspStatusCode::DestinationUnreachable => "Destination Unreachable",
            RtspStatusCode::InternalServerError => "Internal Server Error",
            RtspStatusCode::NotImplemented => "Not Implemented",
            RtspStatusCode::BadGateway => "Bad Gateway",
            RtspStatusCode::ServiceUnavailable => "Service Unavailable",
            RtspStatusCode::GatewayTimeout => "Gateway Timeout",
            RtspStatusCode::RtspVersionNotSupported => "RTSP Version not supported",
            RtspStatusCode::OptionNotSupported => "Option not supported",
        }
    }

    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            100 => Some(RtspStatusCode::Continue),
            200 => Some(RtspStatusCode::Ok),
            201 => Some(RtspStatusCode::Created),
            250 => Some(RtspStatusCode::LowOnStorageSpace),
            300 => Some(RtspStatusCode::MultipleChoices),
            301 => Some(RtspStatusCode::MovedPermanently),
            302 => Some(RtspStatusCode::MovedTemporarily),
            303 => Some(RtspStatusCode::SeeOther),
            304 => Some(RtspStatusCode::NotModified),
            305 => Some(RtspStatusCode::UseProxy),
            400 => Some(RtspStatusCode::BadRequest),
            401 => Some(RtspStatusCode::Unauthorized),
            402 => Some(RtspStatusCode::PaymentRequired),
            403 => Some(RtspStatusCode::Forbidden),
            404 => Some(RtspStatusCode::NotFound),
            405 => Some(RtspStatusCode::MethodNotAllowed),
            406 => Some(RtspStatusCode::NotAcceptable),
            407 => Some(RtspStatusCode::ProxyAuthenticationRequired),
            408 => Some(RtspStatusCode::RequestTimeout),
            410 => Some(RtspStatusCode::Gone),
            411 => Some(RtspStatusCode::LengthRequired),
            412 => Some(RtspStatusCode::PreconditionFailed),
            413 => Some(RtspStatusCode::RequestEntityTooLarge),
            414 => Some(RtspStatusCode::RequestUriTooLarge),
            415 => Some(RtspStatusCode::UnsupportedMediaType),
            451 => Some(RtspStatusCode::ParameterNotUnderstood),
            452 => Some(RtspStatusCode::ConferenceNotFound),
            453 => Some(RtspStatusCode::NotEnoughBandwidth),
            454 => Some(RtspStatusCode::SessionNotFound),
            455 => Some(RtspStatusCode::MethodNotValidInThisState),
            456 => Some(RtspStatusCode::HeaderFieldNotValidForResource),
            457 => Some(RtspStatusCode::InvalidRange),
            458 => Some(RtspStatusCode::ParameterIsReadOnly),
            459 => Some(RtspStatusCode::AggregateOperationNotAllowed),
            460 => Some(RtspStatusCode::OnlyAggregateOperationAllowed),
            461 => Some(RtspStatusCode::UnsupportedTransport),
            462 => Some(RtspStatusCode::DestinationUnreachable),
            500 => Some(RtspStatusCode::InternalServerError),
            501 => Some(RtspStatusCode::NotImplemented),
            502 => Some(RtspStatusCode::BadGateway),
            503 => Some(RtspStatusCode::ServiceUnavailable),
            504 => Some(RtspStatusCode::GatewayTimeout),
            505 => Some(RtspStatusCode::RtspVersionNotSupported),
            551 => Some(RtspStatusCode::OptionNotSupported),
            _ => None,
        }
    }
}

impl fmt::Display for RtspStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.code(), self.reason_phrase())
    }
}
