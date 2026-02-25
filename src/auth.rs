//! RTSP Digest 認証 (RFC 2617)
//!
//! サーバーから受け取った WWW-Authenticate チャレンジに対して
//! Authorization ヘッダー値を生成する。

use md5::{Digest, Md5};
use shiguredo_http11::auth::DigestChallenge;

/// Digest 認証に必要な資格情報
pub struct DigestCredentials {
    pub username: String,
    pub password: String,
}

/// Digest チャレンジに対する Authorization ヘッダー値を生成する
///
/// RFC 2617 Section 3.2.2 に従い、MD5 ダイジェストを計算する。
pub fn build_authorization(
    credentials: &DigestCredentials,
    challenge: &DigestChallenge,
    method: &str,
    uri: &str,
) -> String {
    let realm = challenge.realm().unwrap_or("");
    let nonce = challenge.nonce().unwrap_or("");

    // HA1 = MD5(username:realm:password)
    let ha1 = md5_hex(&format!(
        "{}:{}:{}",
        credentials.username, realm, credentials.password
    ));

    // HA2 = MD5(method:uri)
    let ha2 = md5_hex(&format!("{}:{}", method, uri));

    // response = MD5(HA1:nonce:HA2)
    let response = md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2));

    format!(
        "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", response=\"{}\"",
        credentials.username, realm, nonce, uri, response
    )
}

fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    // 16 進数文字列に変換する
    result.iter().map(|b| format!("{:02x}", b)).collect()
}
