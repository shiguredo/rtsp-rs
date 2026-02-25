use crate::error::Error;
use std::fmt;

/// SDP セッション記述 (RFC 8866)
#[derive(Debug, Clone, PartialEq)]
pub struct Sdp {
    /// v= プロトコルバージョン (常に 0)
    pub version: u8,
    /// o= オリジン
    pub origin: SdpOrigin,
    /// s= セッション名
    pub session_name: String,
    /// i= セッション情報 (オプション)
    pub session_info: Option<String>,
    /// u= URI (オプション)
    pub uri: Option<String>,
    /// e= Email (オプション)
    pub email: Option<String>,
    /// p= Phone (オプション)
    pub phone: Option<String>,
    /// c= 接続情報 (オプション)
    pub connection: Option<SdpConnection>,
    /// b= 帯域幅情報 (オプション)
    pub bandwidth: Vec<SdpBandwidth>,
    /// t= タイミング
    pub timing: SdpTiming,
    /// a= セッションレベル属性
    pub attributes: Vec<SdpAttribute>,
    /// m= メディア記述
    pub media: Vec<SdpMedia>,
}

/// o= オリジン
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdpOrigin {
    pub username: String,
    pub session_id: String,
    pub session_version: String,
    pub net_type: String,
    pub addr_type: String,
    pub address: String,
}

/// c= 接続情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdpConnection {
    pub net_type: String,
    pub addr_type: String,
    pub address: String,
}

/// b= 帯域幅
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdpBandwidth {
    pub bwtype: String,
    pub bandwidth: u64,
}

/// t= タイミング
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdpTiming {
    pub start: u64,
    pub stop: u64,
}

/// m= メディア記述
#[derive(Debug, Clone, PartialEq)]
pub struct SdpMedia {
    pub media_type: String,
    pub port: u16,
    pub num_ports: Option<u16>,
    pub protocol: String,
    pub formats: Vec<String>,
    pub title: Option<String>,
    pub connection: Option<SdpConnection>,
    pub bandwidth: Vec<SdpBandwidth>,
    pub attributes: Vec<SdpAttribute>,
}

/// a= 属性
#[derive(Debug, Clone, PartialEq)]
pub enum SdpAttribute {
    /// a=rtpmap:PT encoding/clock-rate
    Rtpmap {
        payload_type: u8,
        encoding: String,
        clock_rate: u32,
        encoding_params: Option<String>,
    },
    /// a=fmtp:PT parameters
    Fmtp {
        payload_type: u8,
        parameters: String,
    },
    /// a=control:url
    Control(String),
    /// a=range:range
    Range(String),
    /// a=recvonly
    Recvonly,
    /// a=sendrecv
    Sendrecv,
    /// a=sendonly
    Sendonly,
    /// a=inactive
    Inactive,
    /// a=framerate:fps
    Framerate(f64),
    /// a=tool:name
    Tool(String),
    /// a=type:type
    Type(String),
    /// a=charset:charset
    Charset(String),
    /// a=sdplang:lang
    Sdplang(String),
    /// a=lang:lang
    Lang(String),
    /// 拡張属性
    Custom { name: String, value: Option<String> },
}

impl Sdp {
    /// SDP テキストをパース
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut version = None;
        let mut origin = None;
        let mut session_name = None;
        let mut session_info = None;
        let mut uri = None;
        let mut email = None;
        let mut phone = None;
        let mut connection = None;
        let mut bandwidth = Vec::new();
        let mut timing = None;
        let mut attributes = Vec::new();
        let mut media = Vec::new();
        let mut current_media: Option<SdpMedia> = None;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // SDP lines must be ASCII: single char + '=' + value
            // Check that we have at least 2 ASCII bytes
            let bytes = line.as_bytes();
            if bytes.len() < 2 || bytes[1] != b'=' || !bytes[0].is_ascii_alphabetic() {
                continue; // Invalid line format
            }

            let field_type = bytes[0] as char;
            let value = &line[2..];

            match field_type {
                'v' => {
                    version =
                        Some(value.parse().map_err(|_| {
                            Error::invalid_data(format!("invalid version: {}", value))
                        })?);
                }
                'o' => {
                    origin = Some(Self::parse_origin(value)?);
                }
                's' => {
                    session_name = Some(value.to_string());
                }
                'i' => {
                    if current_media.is_some() {
                        if let Some(ref mut m) = current_media {
                            m.title = Some(value.to_string());
                        }
                    } else {
                        session_info = Some(value.to_string());
                    }
                }
                'u' => {
                    uri = Some(value.to_string());
                }
                'e' => {
                    email = Some(value.to_string());
                }
                'p' => {
                    phone = Some(value.to_string());
                }
                'c' => {
                    let conn = Self::parse_connection(value)?;
                    if let Some(ref mut m) = current_media {
                        m.connection = Some(conn);
                    } else {
                        connection = Some(conn);
                    }
                }
                'b' => {
                    let bw = Self::parse_bandwidth(value)?;
                    if let Some(ref mut m) = current_media {
                        m.bandwidth.push(bw);
                    } else {
                        bandwidth.push(bw);
                    }
                }
                't' => {
                    timing = Some(Self::parse_timing(value)?);
                }
                'a' => {
                    let attr = Self::parse_attribute(value)?;
                    if let Some(ref mut m) = current_media {
                        m.attributes.push(attr);
                    } else {
                        attributes.push(attr);
                    }
                }
                'm' => {
                    if let Some(m) = current_media.take() {
                        media.push(m);
                    }
                    current_media = Some(Self::parse_media(value)?);
                }
                _ => {
                    // Ignore unknown fields
                }
            }
        }

        if let Some(m) = current_media.take() {
            media.push(m);
        }

        Ok(Self {
            version: version.unwrap_or(0),
            origin: origin.ok_or_else(|| Error::invalid_data("missing origin (o=)"))?,
            session_name: session_name
                .ok_or_else(|| Error::invalid_data("missing session name (s=)"))?,
            session_info,
            uri,
            email,
            phone,
            connection,
            bandwidth,
            timing: timing.ok_or_else(|| Error::invalid_data("missing timing (t=)"))?,
            attributes,
            media,
        })
    }

    fn parse_origin(value: &str) -> Result<SdpOrigin, Error> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() != 6 {
            return Err(Error::invalid_data(format!("invalid origin: {}", value)));
        }
        Ok(SdpOrigin {
            username: parts[0].to_string(),
            session_id: parts[1].to_string(),
            session_version: parts[2].to_string(),
            net_type: parts[3].to_string(),
            addr_type: parts[4].to_string(),
            address: parts[5].to_string(),
        })
    }

    fn parse_connection(value: &str) -> Result<SdpConnection, Error> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(Error::invalid_data(format!(
                "invalid connection: {}",
                value
            )));
        }
        Ok(SdpConnection {
            net_type: parts[0].to_string(),
            addr_type: parts[1].to_string(),
            address: parts[2].to_string(),
        })
    }

    fn parse_bandwidth(value: &str) -> Result<SdpBandwidth, Error> {
        let parts: Vec<&str> = value.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(Error::invalid_data(format!("invalid bandwidth: {}", value)));
        }
        let bandwidth = parts[1]
            .parse()
            .map_err(|_| Error::invalid_data(format!("invalid bandwidth value: {}", parts[1])))?;
        Ok(SdpBandwidth {
            bwtype: parts[0].to_string(),
            bandwidth,
        })
    }

    fn parse_timing(value: &str) -> Result<SdpTiming, Error> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(Error::invalid_data(format!("invalid timing: {}", value)));
        }
        let start = parts[0]
            .parse()
            .map_err(|_| Error::invalid_data(format!("invalid timing start: {}", parts[0])))?;
        let stop = parts[1]
            .parse()
            .map_err(|_| Error::invalid_data(format!("invalid timing stop: {}", parts[1])))?;
        Ok(SdpTiming { start, stop })
    }

    fn parse_attribute(value: &str) -> Result<SdpAttribute, Error> {
        if let Some((name, attr_value)) = value.split_once(':') {
            match name {
                "rtpmap" => {
                    let parts: Vec<&str> = attr_value.splitn(2, ' ').collect();
                    if parts.len() != 2 {
                        return Err(Error::invalid_data(format!("invalid rtpmap: {}", value)));
                    }
                    let payload_type: u8 = parts[0].parse().map_err(|_| {
                        Error::invalid_data(format!("invalid payload type: {}", parts[0]))
                    })?;
                    let encoding_parts: Vec<&str> = parts[1].split('/').collect();
                    if encoding_parts.len() < 2 {
                        return Err(Error::invalid_data(format!(
                            "invalid encoding: {}",
                            parts[1]
                        )));
                    }
                    let clock_rate: u32 = encoding_parts[1].parse().map_err(|_| {
                        Error::invalid_data(format!("invalid clock rate: {}", encoding_parts[1]))
                    })?;
                    Ok(SdpAttribute::Rtpmap {
                        payload_type,
                        encoding: encoding_parts[0].to_string(),
                        clock_rate,
                        encoding_params: encoding_parts.get(2).map(|s| s.to_string()),
                    })
                }
                "fmtp" => {
                    let parts: Vec<&str> = attr_value.splitn(2, ' ').collect();
                    if parts.len() != 2 {
                        return Err(Error::invalid_data(format!("invalid fmtp: {}", value)));
                    }
                    let payload_type: u8 = parts[0].parse().map_err(|_| {
                        Error::invalid_data(format!("invalid payload type: {}", parts[0]))
                    })?;
                    Ok(SdpAttribute::Fmtp {
                        payload_type,
                        parameters: parts[1].to_string(),
                    })
                }
                "control" => Ok(SdpAttribute::Control(attr_value.to_string())),
                "range" => Ok(SdpAttribute::Range(attr_value.to_string())),
                "framerate" => {
                    let fps: f64 = attr_value.parse().map_err(|_| {
                        Error::invalid_data(format!("invalid framerate: {}", attr_value))
                    })?;
                    Ok(SdpAttribute::Framerate(fps))
                }
                "tool" => Ok(SdpAttribute::Tool(attr_value.to_string())),
                "type" => Ok(SdpAttribute::Type(attr_value.to_string())),
                "charset" => Ok(SdpAttribute::Charset(attr_value.to_string())),
                "sdplang" => Ok(SdpAttribute::Sdplang(attr_value.to_string())),
                "lang" => Ok(SdpAttribute::Lang(attr_value.to_string())),
                _ => Ok(SdpAttribute::Custom {
                    name: name.to_string(),
                    value: Some(attr_value.to_string()),
                }),
            }
        } else {
            match value {
                "recvonly" => Ok(SdpAttribute::Recvonly),
                "sendrecv" => Ok(SdpAttribute::Sendrecv),
                "sendonly" => Ok(SdpAttribute::Sendonly),
                "inactive" => Ok(SdpAttribute::Inactive),
                _ => Ok(SdpAttribute::Custom {
                    name: value.to_string(),
                    value: None,
                }),
            }
        }
    }

    fn parse_media(value: &str) -> Result<SdpMedia, Error> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(Error::invalid_data(format!("invalid media: {}", value)));
        }

        let media_type = parts[0].to_string();
        let (port, num_ports) = if let Some((p, n)) = parts[1].split_once('/') {
            let port: u16 = p
                .parse()
                .map_err(|_| Error::invalid_data(format!("invalid port: {}", p)))?;
            let num: u16 = n
                .parse()
                .map_err(|_| Error::invalid_data(format!("invalid num_ports: {}", n)))?;
            (port, Some(num))
        } else {
            let port: u16 = parts[1]
                .parse()
                .map_err(|_| Error::invalid_data(format!("invalid port: {}", parts[1])))?;
            (port, None)
        };
        let protocol = parts[2].to_string();
        let formats: Vec<String> = parts[3..].iter().map(|s| s.to_string()).collect();

        Ok(SdpMedia {
            media_type,
            port,
            num_ports,
            protocol,
            formats,
            title: None,
            connection: None,
            bandwidth: Vec::new(),
            attributes: Vec::new(),
        })
    }

    /// ビルダーを作成
    pub fn builder() -> SdpBuilder {
        SdpBuilder::new()
    }
}

impl fmt::Display for Sdp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v={}\r\n", self.version)?;
        write!(
            f,
            "o={} {} {} {} {} {}\r\n",
            self.origin.username,
            self.origin.session_id,
            self.origin.session_version,
            self.origin.net_type,
            self.origin.addr_type,
            self.origin.address
        )?;
        write!(f, "s={}\r\n", self.session_name)?;

        if let Some(ref info) = self.session_info {
            write!(f, "i={}\r\n", info)?;
        }
        if let Some(ref uri) = self.uri {
            write!(f, "u={}\r\n", uri)?;
        }
        if let Some(ref email) = self.email {
            write!(f, "e={}\r\n", email)?;
        }
        if let Some(ref phone) = self.phone {
            write!(f, "p={}\r\n", phone)?;
        }
        if let Some(ref conn) = self.connection {
            write!(
                f,
                "c={} {} {}\r\n",
                conn.net_type, conn.addr_type, conn.address
            )?;
        }
        for bw in &self.bandwidth {
            write!(f, "b={}:{}\r\n", bw.bwtype, bw.bandwidth)?;
        }
        write!(f, "t={} {}\r\n", self.timing.start, self.timing.stop)?;

        for attr in &self.attributes {
            write!(f, "a=")?;
            fmt_attribute(f, attr)?;
            write!(f, "\r\n")?;
        }

        for media in &self.media {
            if let Some(num_ports) = media.num_ports {
                write!(
                    f,
                    "m={} {}/{} {}",
                    media.media_type, media.port, num_ports, media.protocol
                )?;
            } else {
                write!(
                    f,
                    "m={} {} {}",
                    media.media_type, media.port, media.protocol
                )?;
            }
            for fmt in &media.formats {
                write!(f, " {}", fmt)?;
            }
            write!(f, "\r\n")?;

            if let Some(ref title) = media.title {
                write!(f, "i={}\r\n", title)?;
            }
            if let Some(ref conn) = media.connection {
                write!(
                    f,
                    "c={} {} {}\r\n",
                    conn.net_type, conn.addr_type, conn.address
                )?;
            }
            for bw in &media.bandwidth {
                write!(f, "b={}:{}\r\n", bw.bwtype, bw.bandwidth)?;
            }
            for attr in &media.attributes {
                write!(f, "a=")?;
                fmt_attribute(f, attr)?;
                write!(f, "\r\n")?;
            }
        }

        Ok(())
    }
}

fn fmt_attribute(f: &mut fmt::Formatter<'_>, attr: &SdpAttribute) -> fmt::Result {
    match attr {
        SdpAttribute::Rtpmap {
            payload_type,
            encoding,
            clock_rate,
            encoding_params,
        } => {
            write!(f, "rtpmap:{} {}/{}", payload_type, encoding, clock_rate)?;
            if let Some(params) = encoding_params {
                write!(f, "/{}", params)?;
            }
        }
        SdpAttribute::Fmtp {
            payload_type,
            parameters,
        } => {
            write!(f, "fmtp:{} {}", payload_type, parameters)?;
        }
        SdpAttribute::Control(url) => {
            write!(f, "control:{}", url)?;
        }
        SdpAttribute::Range(range) => {
            write!(f, "range:{}", range)?;
        }
        SdpAttribute::Recvonly => {
            write!(f, "recvonly")?;
        }
        SdpAttribute::Sendrecv => {
            write!(f, "sendrecv")?;
        }
        SdpAttribute::Sendonly => {
            write!(f, "sendonly")?;
        }
        SdpAttribute::Inactive => {
            write!(f, "inactive")?;
        }
        SdpAttribute::Framerate(fps) => {
            write!(f, "framerate:{}", fps)?;
        }
        SdpAttribute::Tool(name) => {
            write!(f, "tool:{}", name)?;
        }
        SdpAttribute::Type(t) => {
            write!(f, "type:{}", t)?;
        }
        SdpAttribute::Charset(charset) => {
            write!(f, "charset:{}", charset)?;
        }
        SdpAttribute::Sdplang(lang) => {
            write!(f, "sdplang:{}", lang)?;
        }
        SdpAttribute::Lang(lang) => {
            write!(f, "lang:{}", lang)?;
        }
        SdpAttribute::Custom { name, value } => {
            write!(f, "{}", name)?;
            if let Some(v) = value {
                write!(f, ":{}", v)?;
            }
        }
    }
    Ok(())
}

/// SDP ビルダー
#[derive(Debug, Clone)]
pub struct SdpBuilder {
    version: u8,
    origin: Option<SdpOrigin>,
    session_name: Option<String>,
    session_info: Option<String>,
    uri: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    connection: Option<SdpConnection>,
    bandwidth: Vec<SdpBandwidth>,
    timing: Option<SdpTiming>,
    attributes: Vec<SdpAttribute>,
    media: Vec<SdpMedia>,
}

impl Default for SdpBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SdpBuilder {
    pub fn new() -> Self {
        Self {
            version: 0,
            origin: None,
            session_name: None,
            session_info: None,
            uri: None,
            email: None,
            phone: None,
            connection: None,
            bandwidth: Vec::new(),
            timing: None,
            attributes: Vec::new(),
            media: Vec::new(),
        }
    }

    pub fn version(mut self, version: u8) -> Self {
        self.version = version;
        self
    }

    pub fn origin(mut self, origin: SdpOrigin) -> Self {
        self.origin = Some(origin);
        self
    }

    pub fn origin_simple(mut self, session_id: &str, address: &str) -> Self {
        self.origin = Some(SdpOrigin {
            username: "-".to_string(),
            session_id: session_id.to_string(),
            session_version: "1".to_string(),
            net_type: "IN".to_string(),
            addr_type: "IP4".to_string(),
            address: address.to_string(),
        });
        self
    }

    pub fn session_name(mut self, name: &str) -> Self {
        self.session_name = Some(name.to_string());
        self
    }

    pub fn session_info(mut self, info: &str) -> Self {
        self.session_info = Some(info.to_string());
        self
    }

    pub fn uri(mut self, uri: &str) -> Self {
        self.uri = Some(uri.to_string());
        self
    }

    pub fn connection(mut self, connection: SdpConnection) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn connection_simple(mut self, address: &str) -> Self {
        self.connection = Some(SdpConnection {
            net_type: "IN".to_string(),
            addr_type: "IP4".to_string(),
            address: address.to_string(),
        });
        self
    }

    pub fn timing(mut self, start: u64, stop: u64) -> Self {
        self.timing = Some(SdpTiming { start, stop });
        self
    }

    pub fn attribute(mut self, attr: SdpAttribute) -> Self {
        self.attributes.push(attr);
        self
    }

    pub fn control(self, url: &str) -> Self {
        self.attribute(SdpAttribute::Control(url.to_string()))
    }

    pub fn range(self, range: &str) -> Self {
        self.attribute(SdpAttribute::Range(range.to_string()))
    }

    pub fn add_media(mut self, media: SdpMedia) -> Self {
        self.media.push(media);
        self
    }

    pub fn build(self) -> Result<Sdp, Error> {
        Ok(Sdp {
            version: self.version,
            origin: self
                .origin
                .ok_or_else(|| Error::invalid_state("missing origin"))?,
            session_name: self
                .session_name
                .ok_or_else(|| Error::invalid_state("missing session name"))?,
            session_info: self.session_info,
            uri: self.uri,
            email: self.email,
            phone: self.phone,
            connection: self.connection,
            bandwidth: self.bandwidth,
            timing: self
                .timing
                .ok_or_else(|| Error::invalid_state("missing timing"))?,
            attributes: self.attributes,
            media: self.media,
        })
    }
}

/// メディアビルダー
#[derive(Debug, Clone)]
pub struct SdpMediaBuilder {
    media_type: String,
    port: u16,
    num_ports: Option<u16>,
    protocol: String,
    formats: Vec<String>,
    title: Option<String>,
    connection: Option<SdpConnection>,
    bandwidth: Vec<SdpBandwidth>,
    attributes: Vec<SdpAttribute>,
}

impl SdpMediaBuilder {
    pub fn new(media_type: &str, port: u16, protocol: &str) -> Self {
        Self {
            media_type: media_type.to_string(),
            port,
            num_ports: None,
            protocol: protocol.to_string(),
            formats: Vec::new(),
            title: None,
            connection: None,
            bandwidth: Vec::new(),
            attributes: Vec::new(),
        }
    }

    pub fn video(port: u16) -> Self {
        Self::new("video", port, "RTP/AVP")
    }

    pub fn audio(port: u16) -> Self {
        Self::new("audio", port, "RTP/AVP")
    }

    pub fn num_ports(mut self, num: u16) -> Self {
        self.num_ports = Some(num);
        self
    }

    pub fn format(mut self, format: &str) -> Self {
        self.formats.push(format.to_string());
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn connection(mut self, connection: SdpConnection) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn attribute(mut self, attr: SdpAttribute) -> Self {
        self.attributes.push(attr);
        self
    }

    pub fn rtpmap(self, payload_type: u8, encoding: &str, clock_rate: u32) -> Self {
        self.attribute(SdpAttribute::Rtpmap {
            payload_type,
            encoding: encoding.to_string(),
            clock_rate,
            encoding_params: None,
        })
    }

    pub fn fmtp(self, payload_type: u8, parameters: &str) -> Self {
        self.attribute(SdpAttribute::Fmtp {
            payload_type,
            parameters: parameters.to_string(),
        })
    }

    pub fn control(self, url: &str) -> Self {
        self.attribute(SdpAttribute::Control(url.to_string()))
    }

    pub fn build(self) -> SdpMedia {
        SdpMedia {
            media_type: self.media_type,
            port: self.port,
            num_ports: self.num_ports,
            protocol: self.protocol,
            formats: self.formats,
            title: self.title,
            connection: self.connection,
            bandwidth: self.bandwidth,
            attributes: self.attributes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sdp() {
        let sdp_text = r#"v=0
o=- 2890844526 2890842807 IN IP4 192.168.1.1
s=Example
t=0 0
m=video 49170 RTP/AVP 96
a=rtpmap:96 H264/90000
a=control:trackID=1
m=audio 49180 RTP/AVP 97
a=rtpmap:97 MPEG4-GENERIC/44100/2
a=control:trackID=2
"#;

        let sdp = Sdp::parse(sdp_text).unwrap();
        assert_eq!(sdp.version, 0);
        assert_eq!(sdp.session_name, "Example");
        assert_eq!(sdp.media.len(), 2);
        assert_eq!(sdp.media[0].media_type, "video");
        assert_eq!(sdp.media[1].media_type, "audio");
    }

    #[test]
    fn test_build_sdp() {
        let sdp = Sdp::builder()
            .origin_simple("1234567890", "127.0.0.1")
            .session_name("Test Session")
            .timing(0, 0)
            .control("*")
            .add_media(
                SdpMediaBuilder::video(0)
                    .format("96")
                    .rtpmap(96, "H264", 90000)
                    .control("trackID=1")
                    .build(),
            )
            .build()
            .unwrap();

        let text = sdp.to_string();
        assert!(text.contains("v=0"));
        assert!(text.contains("s=Test Session"));
        assert!(text.contains("m=video 0 RTP/AVP 96"));
    }
}
