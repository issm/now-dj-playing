//! ndp-publish ライブラリ
//!
//! 楽曲ファイルからタグ・アートワークを抽出し、
//! ローカルファイルシステムまたは ndp-server に送信するコア機能を提供する。

pub mod config;
pub mod local;
pub mod tags;
pub mod web;
