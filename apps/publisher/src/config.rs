//! 設定ファイルの読み込みとルックアップ

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// 設定ファイルの構造
#[derive(Debug, Deserialize, Default)]
pub struct ConfigFile {
    /// 基本設定（新構造）
    pub base: Option<BaseConfig>,
    /// DJ 名 (後方互換: base.dj_name が未設定の場合のフォールバック)
    pub dj_name: Option<String>,
    /// DJ 画像パス (後方互換: base.dj_image が未設定の場合のフォールバック)
    pub dj_image: Option<String>,
    /// local モード設定
    pub local: Option<LocalConfig>,
    /// web モード設定
    pub web: Option<WebConfig>,
}

/// 基本設定
#[derive(Debug, Deserialize, Default)]
pub struct BaseConfig {
    /// DJ 名 (テキスト)
    pub dj_name: Option<String>,
    /// DJ 画像パス (画像ファイルのフルパス)
    pub dj_image: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LocalConfig {
    /// DJ ディレクトリ名
    pub dj_id: Option<String>,
    /// 出力先ベースディレクトリ（~ 展開あり）
    pub publish_base_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebConfig {
    /// ndp-server の API エンドポイント URL
    pub endpoint_url: Option<String>,
}

/// 読み込み済み設定
pub struct AppConfig {
    /// 設定ファイルのパス（トークンファイル配置先の基準になる）
    pub config_path: Option<PathBuf>,
    /// パース済みの設定内容
    file: ConfigFile,
}

impl AppConfig {
    /// dj_name を取得
    ///
    /// 優先順: base.dj_name → トップレベル dj_name
    pub fn dj_name(&self) -> Option<String> {
        self.file
            .base
            .as_ref()
            .and_then(|b| b.dj_name.clone())
            .or_else(|| self.file.dj_name.clone())
    }

    /// dj_image を取得
    ///
    /// 優先順: base.dj_image → トップレベル dj_image
    pub fn dj_image(&self) -> Option<String> {
        self.file
            .base
            .as_ref()
            .and_then(|b| b.dj_image.clone())
            .or_else(|| self.file.dj_image.clone())
    }

    /// dj_image をパス解決して取得
    pub fn dj_image_path(&self) -> Option<PathBuf> {
        self.dj_image()
            .map(|raw| resolve_path(&raw, self.config_path.as_deref()))
    }

    /// local.dj_id を取得
    pub fn local_dj_id(&self) -> Option<String> {
        self.file.local.as_ref().and_then(|l| l.dj_id.clone())
    }

    /// local.publish_base_dir をパス解決して取得
    pub fn local_publish_base_dir(&self) -> Option<PathBuf> {
        self.file
            .local
            .as_ref()
            .and_then(|l| l.publish_base_dir.as_ref())
            .map(|raw| resolve_path(raw, self.config_path.as_deref()))
    }

    /// web.endpoint_url を取得
    ///
    /// 環境変数 NDP_PUBLISH_ENDPOINT_URL が設定されている場合はそちらを優先する。
    /// これにより publisher-gui 等で UI から渡された値を反映できる。
    pub fn web_endpoint_url(&self) -> Option<String> {
        if let Ok(url) = std::env::var("NDP_PUBLISH_ENDPOINT_URL") {
            if !url.is_empty() {
                return Some(url);
            }
        }
        self.file.web.as_ref().and_then(|w| w.endpoint_url.clone())
    }

    /// セッションファイルのパスを返す (ndp-publish.session.json)
    ///
    /// 配置先ルックアップ:
    /// 1. 環境変数 NDP_PUBLISH_SESSION_DIR
    /// 2. 設定ファイルと同じディレクトリ
    pub fn session_file_path(&self) -> Option<PathBuf> {
        // 環境変数による指定
        if let Ok(session_dir) = std::env::var("NDP_PUBLISH_SESSION_DIR") {
            let dir = PathBuf::from(session_dir);
            if dir.is_dir() {
                return Some(dir.join("ndp-publish.session.json"));
            }
        }

        // 設定ファイルの隣
        self.config_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|dir| dir.join("ndp-publish.session.json"))
    }
}

/// 設定ファイルを読み込む
///
/// ルックアップ順:
/// 1. 引数で指定されたパス
/// 2. 環境変数 NDP_PUBLISH_CONFIG
/// 3. カレントディレクトリの ndp-publish.config.json
/// 4. $HOME/.config/ndp/publish.config.json
pub fn load_config(explicit_path: Option<&Path>) -> Result<AppConfig> {
    if let Some(path) = explicit_path {
        let config = read_config_file(path)?;
        let abs_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        return Ok(AppConfig {
            config_path: Some(abs_path),
            file: config,
        });
    }

    // 環境変数
    if let Ok(env_path) = std::env::var("NDP_PUBLISH_CONFIG") {
        let path = PathBuf::from(env_path);
        if path.is_file() {
            let config = read_config_file(&path)?;
            let abs_path = fs::canonicalize(&path).unwrap_or(path);
            return Ok(AppConfig {
                config_path: Some(abs_path),
                file: config,
            });
        }
    }

    // カレントディレクトリ
    let cwd_config = PathBuf::from("ndp-publish.config.json");
    if cwd_config.is_file() {
        let config = read_config_file(&cwd_config)?;
        let abs_path = fs::canonicalize(&cwd_config).unwrap_or(cwd_config);
        return Ok(AppConfig {
            config_path: Some(abs_path),
            file: config,
        });
    }

    // $HOME/.config/ndp/publish.config.json
    if let Some(home) = dirs::home_dir() {
        let home_config = home.join(".config/ndp/publish.config.json");
        if home_config.is_file() {
            let config = read_config_file(&home_config)?;
            return Ok(AppConfig {
                config_path: Some(home_config),
                file: config,
            });
        }
    }

    // 設定ファイルが見つからない場合もエラーにせず空の設定で続行
    // （--out 等の CLI 引数のみで動作可能にするため）
    Ok(AppConfig {
        config_path: None,
        file: ConfigFile::default(),
    })
}

/// 設定ファイルを読み込んでパースする
fn read_config_file(path: &Path) -> Result<ConfigFile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("設定ファイルの読み込みに失敗: {}", path.display()))?;

    let config: ConfigFile = serde_json_lenient::from_str(&content)
        .with_context(|| format!("設定ファイルのパースに失敗: {}", path.display()))?;

    Ok(config)
}

/// パスを解決する（~ 展開 + 相対パス解決）
fn resolve_path(raw: &str, config_path: Option<&Path>) -> PathBuf {
    let expanded = shellexpand::tilde(raw).to_string();
    let path = PathBuf::from(&expanded);

    if path.is_absolute() {
        path
    } else if let Some(config) = config_path {
        // 設定ファイルの親ディレクトリを基準に解決
        config.parent().map(|dir| dir.join(&path)).unwrap_or(path)
    } else {
        path
    }
}
