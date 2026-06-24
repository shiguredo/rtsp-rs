# shiguredo_http11 を最新バージョン v2026.6.1 に更新する

- Priority: Medium
- Created: 2026-06-24
- Model: Sonnet 4.6
- Branch: feature/update-shiguredo-http11
- Polished: 2026-06-24

## 目的

依存ライブラリ `shiguredo_http11` を最新バージョン v2026.6.1 に追従することで、バグ修正・性能改善・新機能を取り込む。

## 優先度根拠

ライブラリのメンテナンス更新であり、緊急性はないが定期的に追従すべき作業のため Medium とする。

## 現状

v2026.5.0 への更新はコミット `44e969e` で実施済み。現在の各 `Cargo.toml` は以下のとおり。

- `Cargo.toml`: `shiguredo_http11 = { version = "2026.5" }`
- `examples/rtsp_client/Cargo.toml`: `shiguredo_http11 = { version = "2026.5" }`
- `pbt/Cargo.toml`: `shiguredo_http11 = { version = "2026.5.0" }`（バージョン指定形式が不統一）

crates.io での最新バージョンは `2026.6.1`。

`shiguredo_http11` を使用しているソースファイルと依存している型・関数は以下のとおり。

- `src/auth.rs`: `shiguredo_http11::auth::DigestChallenge`（`realm()`, `nonce()`）
- `src/error.rs`: `shiguredo_http11::Error`, `shiguredo_http11::EncodeError`
- `src/rtsp_request.rs`: `shiguredo_http11::HttpHead`, `shiguredo_http11::Request`, `shiguredo_http11::encode_request`, `shiguredo_http11::EncodeError`
- `src/rtsp_response.rs`: `shiguredo_http11::HttpHead`, `shiguredo_http11::Response`, `shiguredo_http11::encode_response`, `shiguredo_http11::EncodeError`
- `src/rtsp_connection.rs`: `shiguredo_http11::DecoderLimits`
- `src/rtsp_client_connection.rs`: `shiguredo_http11::BodyKind`, `shiguredo_http11::BodyProgress`, `shiguredo_http11::Response`, `shiguredo_http11::ResponseDecoder`
- `examples/rtsp_client/src/`: `shiguredo_http11::auth::DigestChallenge`, `shiguredo_http11::uri::Uri`

## 設計方針

1. `shiguredo_http11` v2026.5 → v2026.6 の CHANGELOG を確認し、破壊的変更・非推奨化・動作変更を把握する
2. 上記 3 つの `Cargo.toml` の `shiguredo_http11` バージョン指定を `"2026.6"` に統一して変更する（`pbt/Cargo.toml` は `"2026.5.0"` 形式も合わせて統一すること）
3. `cargo update -p shiguredo_http11` でロックファイルを更新する
4. API 変更によるコンパイルエラーが発生した場合、「現状」に列挙したファイルを確認して修正する
   - `ResponseDecoder` のメソッドシグネチャ変更は影響範囲が広いため優先的に確認すること
   - `DecoderLimits` のフィールド変更は `RtspConnectionLimits` の公開 API 破壊につながるため確認すること
   - `BodyKind` に新バリアントが追加された場合、`rtsp_client_connection.rs` の catch-all アームが意図しない動作をしないか確認すること
5. `CHANGES.md` の `## develop` セクションに `[UPDATE]` エントリを追記する（`CHANGES.md` が未作成の場合は新規作成する）

## 完了条件

- 3 つの `Cargo.toml` すべてで `shiguredo_http11 = { version = "2026.6" }` が統一して記載されている
- `cargo build --workspace` が成功する
- `cargo test --workspace` が成功する（PBT を含む）
- `CHANGES.md` の `## develop` セクションに `[UPDATE] shiguredo_http11 を v2026.6.1 に更新する` エントリが追記されている
