use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// 設定ファイルのスキーマ（JSONC でパースされる）
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigFile {
    pub watch_dir: Option<String>,
    pub dj_id: Option<String>,
    pub enable_comments: Option<bool>,
}

/// アプリケーション設定（デフォルト値が適用済み）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub watch_dir: String,
    pub dj_id: String,
    pub enable_comments: bool,
    /// 読み込まれた設定ファイルのフルパス
    pub config_path: String,
}

/// 設定ファイルをルックアップして読み込み、AppConfig を返す
/// ファイルが見つからない場合はエラーを返す
pub fn load_config() -> Result<AppConfig, String> {
    let config_file = lookup_config_file()?;

    log::info!("設定ファイルを読み込み: {}", config_file.display());

    let file_config = read_config_file(&config_file)?;
    Ok(merge_config(file_config, &config_file))
}

/// ルックアップ順に設定ファイルを探索し、最初に見つかったパスを返す
///
/// 1. 環境変数 NDP_CONFIG
/// 2. アプリ隣接の ndp.config.json
/// 3. $HOME/.config/ndp/config.json
fn lookup_config_file() -> Result<PathBuf, String> {
    // 1. 環境変数 NDP_CONFIG
    if let Ok(env_path) = env::var("NDP_CONFIG") {
        let path = PathBuf::from(&env_path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "NDP_CONFIG が指定されていますがファイルが見つかりません: {}",
            env_path
        ));
    }

    // 2. アプリ隣接の ndp.config.json
    //    macOS .app バンドルの場合: .app/Contents/MacOS/binary → .app の親ディレクトリを探索
    //    非バンドルの場合: バイナリと同じディレクトリを探索
    if let Ok(exe_path) = env::current_exe() {
        for dir in app_adjacent_dirs(&exe_path) {
            let adjacent = dir.join("ndp.config.json");
            if adjacent.is_file() {
                return Ok(adjacent);
            }
        }
    }

    // 3. $HOME/.config/ndp/config.json
    if let Some(home) = dirs_home() {
        let xdg_config = home.join(".config").join("ndp").join("config.json");
        if xdg_config.is_file() {
            return Ok(xdg_config);
        }
    }

    Err("設定ファイルが見つかりません。NDP_CONFIG 環境変数、バイナリ隣接の ndp.config.json、または ~/.config/ndp/config.json を配置してください".to_string())
}

/// JSONC ファイルを読み込みパースする
fn read_config_file(path: &PathBuf) -> Result<AppConfigFile, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("設定ファイルの読み込みに失敗 ({}): {}", path.display(), e))?;
    serde_jsonc::from_str::<AppConfigFile>(&content)
        .map_err(|e| format!("設定ファイルのパースに失敗 ({}): {}", path.display(), e))
}

/// ファイルの設定値をデフォルト値にマージする
fn merge_config(file: AppConfigFile, config_path: &PathBuf) -> AppConfig {
    let watch_dir_raw = file.watch_dir.unwrap_or_else(|| {
        let home = dirs_home().unwrap_or_else(|| "/tmp".into());
        home.join("ndp").display().to_string()
    });
    // ~ をホームディレクトリに展開
    let watch_dir = shellexpand::tilde(&watch_dir_raw).to_string();

    AppConfig {
        watch_dir,
        dj_id: file.dj_id.unwrap_or_else(|| "dj-000".to_string()),
        enable_comments: file.enable_comments.unwrap_or(false),
        config_path: config_path.display().to_string(),
    }
}

/// ホームディレクトリを取得するヘルパー
fn dirs_home() -> Option<PathBuf> {
    env::var("HOME").ok().map(PathBuf::from)
}

/// アプリ隣接ディレクトリの候補を返す
///
/// macOS の .app バンドルの場合、バイナリは `Foo.app/Contents/MacOS/binary` にあるため、
/// `.app` の親ディレクトリも探索対象に含める。
fn app_adjacent_dirs(exe_path: &PathBuf) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // バイナリ自身のディレクトリ（非バンドル時やデバッグビルド時）
    if let Some(exe_dir) = exe_path.parent() {
        dirs.push(exe_dir.to_path_buf());

        // macOS .app バンドル検出: .../Foo.app/Contents/MacOS/binary
        // → Contents/MacOS の 2 階層上が .app ディレクトリ
        if exe_dir.ends_with("Contents/MacOS") {
            if let Some(app_dir) = exe_dir.parent().and_then(|p| p.parent()) {
                // .app バンドルの親ディレクトリを追加
                if let Some(app_parent) = app_dir.parent() {
                    dirs.push(app_parent.to_path_buf());
                }
            }
        }
    }

    dirs
}
