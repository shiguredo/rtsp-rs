use std::fmt;
use std::str::FromStr;

/// RTSP メソッド (RFC 2326 Section 10)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RtspMethod {
    /// サーバーがサポートするメソッドを問い合わせる
    Options,
    /// メディアの説明を取得する (SDP)
    Describe,
    /// サーバーにメディアの説明を送信する
    Announce,
    /// トランスポートを設定する
    Setup,
    /// 再生を開始する
    Play,
    /// 再生を一時停止する
    Pause,
    /// セッションを終了する
    Teardown,
    /// パラメータを取得する
    GetParameter,
    /// パラメータを設定する
    SetParameter,
    /// クライアントを別のサーバーにリダイレクトする
    Redirect,
    /// 録画を開始する
    Record,
    /// 拡張メソッド (RFC 2326 Section 10)
    Extension(String),
}

impl RtspMethod {
    pub fn as_str(&self) -> &str {
        match self {
            RtspMethod::Options => "OPTIONS",
            RtspMethod::Describe => "DESCRIBE",
            RtspMethod::Announce => "ANNOUNCE",
            RtspMethod::Setup => "SETUP",
            RtspMethod::Play => "PLAY",
            RtspMethod::Pause => "PAUSE",
            RtspMethod::Teardown => "TEARDOWN",
            RtspMethod::GetParameter => "GET_PARAMETER",
            RtspMethod::SetParameter => "SET_PARAMETER",
            RtspMethod::Redirect => "REDIRECT",
            RtspMethod::Record => "RECORD",
            RtspMethod::Extension(s) => s.as_str(),
        }
    }
}

impl fmt::Display for RtspMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RtspMethod {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "OPTIONS" => RtspMethod::Options,
            "DESCRIBE" => RtspMethod::Describe,
            "ANNOUNCE" => RtspMethod::Announce,
            "SETUP" => RtspMethod::Setup,
            "PLAY" => RtspMethod::Play,
            "PAUSE" => RtspMethod::Pause,
            "TEARDOWN" => RtspMethod::Teardown,
            "GET_PARAMETER" => RtspMethod::GetParameter,
            "SET_PARAMETER" => RtspMethod::SetParameter,
            "REDIRECT" => RtspMethod::Redirect,
            "RECORD" => RtspMethod::Record,
            _ => RtspMethod::Extension(s.to_string()),
        })
    }
}
