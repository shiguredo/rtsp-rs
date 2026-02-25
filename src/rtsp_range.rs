use crate::error::Error;
use std::fmt;

/// RTSP Range ヘッダー (RFC 2326 Section 12.29)
///
/// 形式:
/// - `npt=<start>-<end>`
/// - `smpte=<start>-<end>`
/// - `clock=<start>-<end>`
#[derive(Debug, Clone, PartialEq)]
pub enum RtspRange {
    /// Normal Play Time (npt)
    Npt(NptRange),
    /// SMPTE タイムコード
    Smpte(SmpteRange),
    /// 絶対時刻 (UTC)
    Clock(ClockRange),
}

impl RtspRange {
    /// Range ヘッダー値をパース
    pub fn parse(s: &str) -> Result<Self, Error> {
        let s = s.trim();

        if let Some(rest) = s.strip_prefix("npt=") {
            Ok(RtspRange::Npt(NptRange::parse(rest)?))
        } else if let Some(rest) = s.strip_prefix("smpte=") {
            Ok(RtspRange::Smpte(SmpteRange::parse_with_type(
                rest,
                SmpteType::Smpte,
            )?))
        } else if let Some(rest) = s.strip_prefix("smpte-30-drop=") {
            Ok(RtspRange::Smpte(SmpteRange::parse_with_type(
                rest,
                SmpteType::Smpte30Drop,
            )?))
        } else if let Some(rest) = s.strip_prefix("smpte-25=") {
            Ok(RtspRange::Smpte(SmpteRange::parse_with_type(
                rest,
                SmpteType::Smpte25,
            )?))
        } else if let Some(rest) = s.strip_prefix("clock=") {
            Ok(RtspRange::Clock(ClockRange::parse(rest)?))
        } else {
            Err(Error::invalid_data(format!("unknown range format: {}", s)))
        }
    }
}

impl fmt::Display for RtspRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RtspRange::Npt(npt) => write!(f, "npt={}", npt),
            RtspRange::Smpte(smpte) => {
                let prefix = match smpte.smpte_type {
                    SmpteType::Smpte => "smpte",
                    SmpteType::Smpte30Drop => "smpte-30-drop",
                    SmpteType::Smpte25 => "smpte-25",
                };
                write!(f, "{}={}", prefix, smpte)
            }
            RtspRange::Clock(clock) => write!(f, "clock={}", clock),
        }
    }
}

/// NPT (Normal Play Time) の時刻値
#[derive(Debug, Clone, PartialEq)]
pub enum NptTime {
    /// "now" - 現在時刻
    Now,
    /// 秒数 (小数点以下も可)
    Seconds(f64),
}

impl NptTime {
    fn parse(s: &str) -> Result<Self, Error> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("now") {
            return Ok(NptTime::Now);
        }

        // npt-hhmmss = npt-hh ":" npt-mm ":" npt-ss [ "." *DIGIT ]
        // RFC 2326 Section 3.6 では hh:mm:ss の 3 要素形式のみ定義されている
        if s.contains(':') {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() == 3 {
                let hours: f64 = parts[0]
                    .parse()
                    .map_err(|_| Error::invalid_data(format!("invalid hours: {}", parts[0])))?;
                let minutes: f64 = parts[1]
                    .parse()
                    .map_err(|_| Error::invalid_data(format!("invalid minutes: {}", parts[1])))?;
                let seconds: f64 = parts[2]
                    .parse()
                    .map_err(|_| Error::invalid_data(format!("invalid seconds: {}", parts[2])))?;
                return Ok(NptTime::Seconds(hours * 3600.0 + minutes * 60.0 + seconds));
            }
            return Err(Error::invalid_data(format!(
                "invalid npt-hhmmss format (expected hh:mm:ss): {}",
                s
            )));
        }

        // Plain seconds
        let seconds: f64 = s
            .parse()
            .map_err(|_| Error::invalid_data(format!("invalid npt time: {}", s)))?;
        Ok(NptTime::Seconds(seconds))
    }
}

impl fmt::Display for NptTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NptTime::Now => write!(f, "now"),
            NptTime::Seconds(s) => write!(f, "{}", s),
        }
    }
}

/// NPT Range
#[derive(Debug, Clone, PartialEq)]
pub struct NptRange {
    /// 開始時刻
    pub start: NptTime,
    /// 終了時刻 (None = end of media)
    pub end: Option<NptTime>,
}

impl NptRange {
    pub fn new(start: NptTime, end: Option<NptTime>) -> Self {
        Self { start, end }
    }

    /// 開始位置から最後まで
    pub fn from_start(start: f64) -> Self {
        Self {
            start: NptTime::Seconds(start),
            end: None,
        }
    }

    /// 最初から最後まで
    pub fn all() -> Self {
        Self {
            start: NptTime::Seconds(0.0),
            end: None,
        }
    }

    /// now から最後まで
    pub fn from_now() -> Self {
        Self {
            start: NptTime::Now,
            end: None,
        }
    }

    fn parse(s: &str) -> Result<Self, Error> {
        let s = s.trim();

        // "-" npt-time 形式: 先頭から指定時刻まで (RFC 2326 Section 12.29)
        if let Some(end_str) = s.strip_prefix('-') {
            if end_str.is_empty() {
                return Err(Error::invalid_data("empty npt end time after '-'"));
            }
            return Ok(NptRange {
                start: NptTime::Seconds(0.0),
                end: Some(NptTime::parse(end_str)?),
            });
        }

        let parts: Vec<&str> = s.splitn(2, '-').collect();

        let start = NptTime::parse(parts[0])?;
        let end = if parts.len() > 1 && !parts[1].is_empty() {
            Some(NptTime::parse(parts[1])?)
        } else {
            None
        };

        Ok(NptRange { start, end })
    }
}

impl fmt::Display for NptRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.end {
            Some(end) => write!(f, "{}-{}", self.start, end),
            None => write!(f, "{}-", self.start),
        }
    }
}

/// SMPTE タイプ (RFC 2326 Section 3.5)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpteType {
    /// smpte
    Smpte,
    /// smpte-30-drop
    Smpte30Drop,
    /// smpte-25
    Smpte25,
}

/// SMPTE タイムコード
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmpteTime {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
    pub subframes: Option<u8>,
}

impl SmpteTime {
    fn parse(s: &str) -> Result<Self, Error> {
        let s = s.trim();
        let parts: Vec<&str> = s.split(':').collect();

        if parts.len() < 3 {
            return Err(Error::invalid_data(format!("invalid smpte time: {}", s)));
        }

        let hours: u8 = parts[0]
            .parse()
            .map_err(|_| Error::invalid_data(format!("invalid hours: {}", parts[0])))?;
        let minutes: u8 = parts[1]
            .parse()
            .map_err(|_| Error::invalid_data(format!("invalid minutes: {}", parts[1])))?;
        let seconds: u8 = parts[2]
            .parse()
            .map_err(|_| Error::invalid_data(format!("invalid seconds: {}", parts[2])))?;

        let (frames, subframes) =
            if parts.len() > 3 {
                if let Some((f, sf)) = parts[3].split_once('.') {
                    (
                        f.parse()
                            .map_err(|_| Error::invalid_data(format!("invalid frames: {}", f)))?,
                        Some(sf.parse().map_err(|_| {
                            Error::invalid_data(format!("invalid subframes: {}", sf))
                        })?),
                    )
                } else {
                    (
                        parts[3].parse().map_err(|_| {
                            Error::invalid_data(format!("invalid frames: {}", parts[3]))
                        })?,
                        None,
                    )
                }
            } else {
                (0, None)
            };

        Ok(SmpteTime {
            hours,
            minutes,
            seconds,
            frames,
            subframes,
        })
    }
}

impl fmt::Display for SmpteTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}:{:02}",
            self.hours, self.minutes, self.seconds, self.frames
        )?;
        if let Some(sf) = self.subframes {
            write!(f, ".{:02}", sf)?;
        }
        Ok(())
    }
}

/// SMPTE Range
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmpteRange {
    /// SMPTE タイプ
    pub smpte_type: SmpteType,
    pub start: SmpteTime,
    pub end: Option<SmpteTime>,
}

impl SmpteRange {
    fn parse_with_type(s: &str, smpte_type: SmpteType) -> Result<Self, Error> {
        let s = s.trim();

        // smpte format: hh:mm:ss:ff.sf - hh:mm:ss:ff.sf
        // We need to split on '-' but be careful of the format
        let parts: Vec<&str> = s.splitn(2, '-').collect();

        let start = SmpteTime::parse(parts[0])?;
        let end = if parts.len() > 1 && !parts[1].is_empty() {
            Some(SmpteTime::parse(parts[1])?)
        } else {
            None
        };

        Ok(SmpteRange {
            smpte_type,
            start,
            end,
        })
    }
}

impl fmt::Display for SmpteRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.end {
            Some(end) => write!(f, "{}-{}", self.start, end),
            None => write!(f, "{}-", self.start),
        }
    }
}

/// Clock (UTC) Range
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockRange {
    /// 開始時刻 (ISO 8601 形式)
    pub start: String,
    /// 終了時刻
    pub end: Option<String>,
}

impl ClockRange {
    fn parse(s: &str) -> Result<Self, Error> {
        let s = s.trim();
        let parts: Vec<&str> = s.splitn(2, '-').collect();

        let start = parts[0].to_string();
        let end = if parts.len() > 1 && !parts[1].is_empty() {
            Some(parts[1].to_string())
        } else {
            None
        };

        Ok(ClockRange { start, end })
    }
}

impl fmt::Display for ClockRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.end {
            Some(end) => write!(f, "{}-{}", self.start, end),
            None => write!(f, "{}-", self.start),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npt_parse() {
        // Simple seconds
        let range = RtspRange::parse("npt=0-").unwrap();
        if let RtspRange::Npt(npt) = range {
            assert!(matches!(npt.start, NptTime::Seconds(s) if s == 0.0));
            assert!(npt.end.is_none());
        } else {
            panic!("expected Npt");
        }

        // Seconds with decimal
        let range = RtspRange::parse("npt=10.5-20.3").unwrap();
        if let RtspRange::Npt(npt) = range {
            assert!(matches!(npt.start, NptTime::Seconds(s) if (s - 10.5).abs() < 0.001));
            assert!(matches!(npt.end, Some(NptTime::Seconds(s)) if (s - 20.3).abs() < 0.001));
        } else {
            panic!("expected Npt");
        }

        // now
        let range = RtspRange::parse("npt=now-").unwrap();
        if let RtspRange::Npt(npt) = range {
            assert!(matches!(npt.start, NptTime::Now));
        } else {
            panic!("expected Npt");
        }

        // hh:mm:ss format
        let range = RtspRange::parse("npt=0:10:30-").unwrap();
        if let RtspRange::Npt(npt) = range {
            // 0 hours + 10 minutes + 30 seconds = 630 seconds
            assert!(matches!(npt.start, NptTime::Seconds(s) if (s - 630.0).abs() < 0.001));
        } else {
            panic!("expected Npt");
        }
    }

    #[test]
    fn test_smpte_parse() {
        let range = RtspRange::parse("smpte=0:10:20:00-0:20:30:00").unwrap();
        if let RtspRange::Smpte(smpte) = range {
            assert_eq!(smpte.start.hours, 0);
            assert_eq!(smpte.start.minutes, 10);
            assert_eq!(smpte.start.seconds, 20);
            assert!(smpte.end.is_some());
        } else {
            panic!("expected Smpte");
        }
    }

    #[test]
    fn test_clock_parse() {
        let range = RtspRange::parse("clock=19960213T143205Z-").unwrap();
        if let RtspRange::Clock(clock) = range {
            assert_eq!(clock.start, "19960213T143205Z");
            assert!(clock.end.is_none());
        } else {
            panic!("expected Clock");
        }
    }

    #[test]
    fn test_npt_reverse_range() {
        // "-" npt-time 形式
        let range = RtspRange::parse("npt=-30.5").unwrap();
        if let RtspRange::Npt(npt) = range {
            assert!(matches!(npt.start, NptTime::Seconds(s) if s == 0.0));
            assert!(matches!(npt.end, Some(NptTime::Seconds(s)) if (s - 30.5).abs() < 0.001));
        } else {
            panic!("expected Npt");
        }

        // "-" だけはエラー
        assert!(RtspRange::parse("npt=-").is_err());
    }

    #[test]
    fn test_smpte_type_preserved() {
        // smpte
        let range = RtspRange::parse("smpte=0:10:20:00-").unwrap();
        if let RtspRange::Smpte(smpte) = &range {
            assert_eq!(smpte.smpte_type, SmpteType::Smpte);
        } else {
            panic!("expected Smpte");
        }
        assert_eq!(range.to_string(), "smpte=00:10:20:00-");

        // smpte-30-drop
        let range = RtspRange::parse("smpte-30-drop=0:10:20:00-").unwrap();
        if let RtspRange::Smpte(smpte) = &range {
            assert_eq!(smpte.smpte_type, SmpteType::Smpte30Drop);
        } else {
            panic!("expected Smpte");
        }
        assert_eq!(range.to_string(), "smpte-30-drop=00:10:20:00-");

        // smpte-25
        let range = RtspRange::parse("smpte-25=0:10:20:00-").unwrap();
        if let RtspRange::Smpte(smpte) = &range {
            assert_eq!(smpte.smpte_type, SmpteType::Smpte25);
        } else {
            panic!("expected Smpte");
        }
        assert_eq!(range.to_string(), "smpte-25=00:10:20:00-");
    }

    #[test]
    fn test_display() {
        let range = NptRange::from_start(10.5);
        assert_eq!(format!("npt={}", range), "npt=10.5-");

        let range = NptRange::all();
        assert_eq!(format!("npt={}", range), "npt=0-");
    }
}
