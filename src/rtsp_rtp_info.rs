use crate::error::Error;
use std::fmt;

/// RTP-Info ヘッダー (RFC 2326 Section 12.33)
///
/// 形式: `url=<url>;seq=<num>;rtptime=<num>`
/// 複数のストリームがある場合はカンマ区切り
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpInfo {
    pub streams: Vec<RtpInfoStream>,
}

impl RtpInfo {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    /// RTP-Info ヘッダー値をパース
    pub fn parse(s: &str) -> Result<Self, Error> {
        let mut streams = Vec::new();

        // カンマで分割（ただし url 内のカンマに注意）
        for part in split_rtp_info_streams(s) {
            let stream = RtpInfoStream::parse(part.trim())?;
            streams.push(stream);
        }

        Ok(RtpInfo { streams })
    }

    /// ストリームを追加
    pub fn add_stream(&mut self, stream: RtpInfoStream) {
        self.streams.push(stream);
    }

    /// URL でストリームを検索
    pub fn find_by_url(&self, url: &str) -> Option<&RtpInfoStream> {
        self.streams.iter().find(|s| s.url == url)
    }
}

impl Default for RtpInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RtpInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.streams.iter().map(|s| s.to_string()).collect();
        write!(f, "{}", parts.join(","))
    }
}

/// RTP-Info の個別ストリーム情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpInfoStream {
    /// ストリーム URL
    pub url: String,
    /// RTP シーケンス番号
    pub seq: Option<u16>,
    /// RTP タイムスタンプ
    pub rtptime: Option<u32>,
}

impl RtpInfoStream {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            seq: None,
            rtptime: None,
        }
    }

    pub fn with_seq(mut self, seq: u16) -> Self {
        self.seq = Some(seq);
        self
    }

    pub fn with_rtptime(mut self, rtptime: u32) -> Self {
        self.rtptime = Some(rtptime);
        self
    }

    fn parse(s: &str) -> Result<Self, Error> {
        let mut url = None;
        let mut seq = None;
        let mut rtptime = None;

        // セミコロンで分割
        for part in s.split(';') {
            let part = part.trim();
            if let Some(value) = part.strip_prefix("url=") {
                url = Some(value.to_string());
            } else if let Some(value) = part.strip_prefix("seq=") {
                seq = Some(
                    value
                        .parse()
                        .map_err(|_| Error::invalid_data(format!("invalid seq: {}", value)))?,
                );
            } else if let Some(value) = part.strip_prefix("rtptime=") {
                rtptime = Some(
                    value
                        .parse()
                        .map_err(|_| Error::invalid_data(format!("invalid rtptime: {}", value)))?,
                );
            }
        }

        let url = url.ok_or_else(|| Error::invalid_data("missing url in RTP-Info"))?;

        Ok(RtpInfoStream { url, seq, rtptime })
    }
}

impl fmt::Display for RtpInfoStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "url={}", self.url)?;
        if let Some(seq) = self.seq {
            write!(f, ";seq={}", seq)?;
        }
        if let Some(rtptime) = self.rtptime {
            write!(f, ";rtptime={}", rtptime)?;
        }
        Ok(())
    }
}

/// RTP-Info ヘッダーをストリームごとに分割
fn split_rtp_info_streams(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_url = false;

    for (i, c) in s.char_indices() {
        match c {
            '=' if !in_url => {
                // url= の後
                if s[start..i].trim().ends_with("url") {
                    in_url = true;
                }
            }
            ';' if in_url => {
                in_url = false;
            }
            ',' if !in_url => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < s.len() {
        result.push(&s[start..]);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_stream() {
        let info =
            RtpInfo::parse("url=rtsp://example.com/audio;seq=1234;rtptime=12345678").unwrap();
        assert_eq!(info.streams.len(), 1);
        assert_eq!(info.streams[0].url, "rtsp://example.com/audio");
        assert_eq!(info.streams[0].seq, Some(1234));
        assert_eq!(info.streams[0].rtptime, Some(12345678));
    }

    #[test]
    fn test_parse_multiple_streams() {
        let info = RtpInfo::parse(
            "url=rtsp://example.com/audio;seq=100;rtptime=1000,url=rtsp://example.com/video;seq=200;rtptime=2000"
        ).unwrap();
        assert_eq!(info.streams.len(), 2);

        assert_eq!(info.streams[0].url, "rtsp://example.com/audio");
        assert_eq!(info.streams[0].seq, Some(100));

        assert_eq!(info.streams[1].url, "rtsp://example.com/video");
        assert_eq!(info.streams[1].seq, Some(200));
    }

    #[test]
    fn test_parse_without_optional() {
        let info = RtpInfo::parse("url=rtsp://example.com/stream").unwrap();
        assert_eq!(info.streams.len(), 1);
        assert_eq!(info.streams[0].url, "rtsp://example.com/stream");
        assert!(info.streams[0].seq.is_none());
        assert!(info.streams[0].rtptime.is_none());
    }

    #[test]
    fn test_display() {
        let mut info = RtpInfo::new();
        info.add_stream(
            RtpInfoStream::new("rtsp://example.com/audio")
                .with_seq(100)
                .with_rtptime(1000),
        );
        info.add_stream(
            RtpInfoStream::new("rtsp://example.com/video")
                .with_seq(200)
                .with_rtptime(2000),
        );

        let s = info.to_string();
        assert!(s.contains("url=rtsp://example.com/audio"));
        assert!(s.contains("seq=100"));
        assert!(s.contains("url=rtsp://example.com/video"));
    }

    #[test]
    fn test_find_by_url() {
        let info = RtpInfo::parse(
            "url=rtsp://example.com/audio;seq=100,url=rtsp://example.com/video;seq=200",
        )
        .unwrap();

        let audio = info.find_by_url("rtsp://example.com/audio");
        assert!(audio.is_some());
        assert_eq!(audio.unwrap().seq, Some(100));

        let video = info.find_by_url("rtsp://example.com/video");
        assert!(video.is_some());
        assert_eq!(video.unwrap().seq, Some(200));

        let unknown = info.find_by_url("rtsp://example.com/unknown");
        assert!(unknown.is_none());
    }
}
