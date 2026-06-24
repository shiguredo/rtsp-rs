# shiguredo_http11 を最新バージョン v2026.6.1 に更新する

- Priority: Medium
- Created: 2026-06-24
- Model: Sonnet 4.6
- Branch: feature/update-shiguredo-http11

## 目的

依存ライブラリ `shiguredo_http11` を最新バージョン v2026.6.1 に追従することで、バグ修正・性能改善・新機能を取り込む。
また、依存ライブラリのバージョンを最新に保つことでセキュリティリスクを低減する。

## 優先度根拠

ライブラリのメンテナンス更新であり、緊急性はないが定期的に追従すべき作業のため Medium とする。

## 現状

`Cargo.toml` の `shiguredo_http11` は `"2026.5"` を指定している。
crates.io での最新バージョンは `2026.6.1` であり、1 マイナーバージョン分の遅れがある。

## 設計方針

`Cargo.toml` の依存バージョンを `"2026.6"` に変更し、`cargo update` でロックファイルを更新する。
API 変更によるコンパイルエラーが発生した場合は修正する。

## 完了条件

- `Cargo.toml` に `shiguredo_http11 = { version = "2026.6" }` が記載されている
- `cargo build` が成功する
- 全テストが通る
