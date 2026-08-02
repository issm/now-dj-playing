use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// 背景画像設定（設定ファイル側）
#[derive(Debug, Clone, Deserialize)]
pub struct BackgroundImageConfigFile {
    /// 背景画像を格納するディレクトリ（~ 展開・相対パス解決あり）
    pub base_dir: String,
    /// base_dir からの相対パス（null で「なし」）
    pub path: Option<String>,
}

/// 背景画像設定（解決済み・フロントエンドに送る形式）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImageConfig {
    /// 背景画像ディレクトリの絶対パス
    pub base_dir: String,
    /// base_dir からの相対パス（null で「なし」）
    pub path: Option<String>,
}

/// データソースモード
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Local,
    Web,
}

/// local モード固有の設定（設定ファイル側）
#[derive(Debug, Clone, Deserialize)]
pub struct LocalConfigFile {
    pub watch_dir: Option<String>,
    pub dj_id: Option<String>,
}

/// web モード固有の設定（設定ファイル側）
#[derive(Debug, Clone, Deserialize)]
pub struct WebConfigFile {
    pub server_url: Option<String>,
}

/// local モード固有の設定（解決済み・フロントエンドに送る形式）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalConfig {
    pub watch_dir: String,
    pub dj_id: String,
}

/// web モード固有の設定（解決済み・フロントエンドに送る形式）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebConfig {
    pub server_url: String,
}

/// 設定ファイルのスキーマ（JSONC でパースされる）
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigFile {
    /// データソースモード（省略時は local）
    pub mode: Option<Mode>,
    /// local モード固有の設定
    pub local: Option<LocalConfigFile>,
    /// web モード固有の設定
    pub web: Option<WebConfigFile>,
    pub enable_comments: Option<bool>,
    pub show_tags: Option<bool>,
    pub event_name: Option<String>,
    pub show_event_name: Option<bool>,
    /// 背景画像設定（オブジェクト形式）
    pub background_image: Option<BackgroundImageConfigFile>,
}

/// アプリケーション設定（デフォルト値が適用済み）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// データソースモード
    pub mode: Mode,
    /// local モード固有の設定
    pub local: LocalConfig,
    /// web モード固有の設定
    pub web: WebConfig,
    pub enable_comments: bool,
    pub show_tags: bool,
    /// イベント名（省略時は None）
    pub event_name: Option<String>,
    /// イベント名を表示するかどうか（デフォルト: true）
    pub show_event_name: bool,
    /// 背景画像設定（省略時は None = 機能無効）
    pub background_image: Option<BackgroundImageConfig>,
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

/// 設定ファイル内の値をテキスト置換で更新する
///
/// JSONC のコメントを保持したまま、指定キーの値のみ書き換える。
/// 書き戻し対象はホワイトリストで制限する。
///
/// # 引数
/// - `config_path`: 設定ファイルのパス
/// - `key`: 設定キー名（例: "enable_comments", "event_name", "background_image.path"）
/// - `value`: 新しい値の JSON リテラル（例: "true", "false", "\"Club Night\"", "null"）
pub fn update_config_value(config_path: &str, key: &str, value: &str) -> Result<(), String> {
    // 書き戻し対象キーのホワイトリスト
    const ALLOWED_BOOL_KEYS: &[&str] = &["enable_comments", "show_tags", "show_event_name"];
    const ALLOWED_STRING_KEYS: &[&str] = &["event_name"];
    const ALLOWED_NESTED_KEYS: &[&str] = &["background_image.path"];

    let path = PathBuf::from(config_path);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("設定ファイルの読み込みに失敗 ({}): {}", config_path, e))?;

    let new_content = if ALLOWED_BOOL_KEYS.contains(&key) {
        // bool 値のバリデーション
        if value != "true" && value != "false" {
            return Err(format!(
                "\"{}\" の値は true/false である必要があります: {}",
                key, value
            ));
        }
        replace_bool_value(&content, key, value)?
    } else if ALLOWED_STRING_KEYS.contains(&key) {
        replace_string_value(&content, key, value)?
    } else if ALLOWED_NESTED_KEYS.contains(&key) {
        // "background_image.path" → parent="background_image", child="path"
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        if parts.len() != 2 {
            return Err(format!("ネストキーの形式が不正です: {}", key));
        }
        replace_nested_value(&content, parts[0], parts[1], value)?
    } else {
        return Err(format!("書き戻し対象外のキーです: {}", key));
    };

    fs::write(&path, new_content.as_bytes())
        .map_err(|e| format!("設定ファイルの書き込みに失敗 ({}): {}", config_path, e))?;

    log::info!("設定を書き戻し: {} = {} ({})", key, value, config_path);
    Ok(())
}

/// bool キーの値を置換する
fn replace_bool_value(content: &str, key: &str, value: &str) -> Result<String, String> {
    let pattern = format!(r#"("{}"\s*:\s*)(true|false)"#, regex::escape(key));
    let re = Regex::new(&pattern).map_err(|e| format!("正規表現エラー: {}", e))?;

    if !re.is_match(content) {
        return Err(format!("設定ファイル内にキー \"{}\" が見つかりません", key));
    }

    Ok(re.replace(content, format!("${{1}}{}", value)).to_string())
}

/// 文字列キーの値を置換する（値は JSON 文字列リテラル形式: "\"value\""）
fn replace_string_value(content: &str, key: &str, value: &str) -> Result<String, String> {
    // "key": "..." or "key": null のパターン
    let pattern = format!(
        r#"("{}"\s*:\s*)("(?:[^"\\]|\\.)*"|null)"#,
        regex::escape(key)
    );
    let re = Regex::new(&pattern).map_err(|e| format!("正規表現エラー: {}", e))?;

    if !re.is_match(content) {
        return Err(format!("設定ファイル内にキー \"{}\" が見つかりません", key));
    }

    Ok(re.replace(content, format!("${{1}}{}", value)).to_string())
}

/// ネストされたオブジェクト内のキーの値を置換する
///
/// 例: background_image オブジェクト内の "path" キー
/// コメントアウトされた行は無視し、有効な行のみ置換する。
/// 有効な行が見つからない場合はオブジェクト内に追加する。
fn replace_nested_value(
    content: &str,
    parent_key: &str,
    child_key: &str,
    value: &str,
) -> Result<String, String> {
    // まずコメントアウトされていない有効な "path": ... 行を探す
    // parent オブジェクトの範囲内で探索する
    let parent_pattern = format!(r#""{}"\s*:\s*\{{"#, regex::escape(parent_key));
    let parent_re = Regex::new(&parent_pattern).map_err(|e| format!("正規表現エラー: {}", e))?;

    let parent_match = parent_re
        .find(content)
        .ok_or_else(|| format!("設定ファイル内にキー \"{}\" が見つかりません", parent_key))?;

    // parent オブジェクトの開始位置から閉じ `}` を探す
    let obj_start = parent_match.end();
    let obj_end = find_matching_brace(content, obj_start - 1)
        .ok_or_else(|| format!("\"{}\" オブジェクトの閉じ括弧が見つかりません", parent_key))?;

    let obj_content = &content[obj_start..obj_end];

    // オブジェクト内でコメントアウトされていない child_key の行を探す
    // 行頭がスペース/タブのみ（// で始まらない）で "child_key": ... のパターン
    let child_pattern = format!(
        r#"(?m)^([ \t]*"{}"\s*:\s*)("(?:[^"\\]|\\.)*"|null)"#,
        regex::escape(child_key)
    );
    let child_re = Regex::new(&child_pattern).map_err(|e| format!("正規表現エラー: {}", e))?;

    if let Some(child_match) = child_re.find(obj_content) {
        // 既存の有効行を置換
        let abs_start = obj_start + child_match.start();
        let abs_end = obj_start + child_match.end();
        let replacement = child_re
            .replace(&content[abs_start..abs_end], format!("${{1}}{}", value))
            .to_string();
        let mut result = String::with_capacity(content.len());
        result.push_str(&content[..abs_start]);
        result.push_str(&replacement);
        result.push_str(&content[abs_end..]);
        Ok(result)
    } else {
        // 有効行が見つからない場合、オブジェクトの閉じ `}` の手前に追加
        let before_close = &content[..obj_end];
        let trimmed = before_close.trim_end();
        let needs_comma = !trimmed.is_empty() && !trimmed.ends_with('{') && !trimmed.ends_with(',');

        let indent = "    "; // ネストなので 4 スペース
        let mut result = String::with_capacity(content.len() + 50);

        if needs_comma {
            result.push_str(trimmed);
            result.push(',');
            let trailing = &before_close[trimmed.len()..];
            if !trailing.contains('\n') {
                result.push('\n');
            } else {
                result.push_str(trailing);
            }
        } else {
            result.push_str(before_close);
            if !before_close.ends_with('\n') {
                result.push('\n');
            }
        }

        result.push_str(&format!("{}\"{}\": {}\n", indent, child_key, value));
        result.push_str(&content[obj_end..]);
        Ok(result)
    }
}

/// 開き括弧 `{` に対応する閉じ括弧 `}` の位置を返す
/// JSONC のコメント内の括弧は無視する
fn find_matching_brace(content: &str, open_pos: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut depth = 0;
    let mut i = open_pos;
    let len = bytes.len();

    while i < len {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'"' => {
                // 文字列リテラルをスキップ
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i += 1; // エスケープ文字をスキップ
                    } else if bytes[i] == b'"' {
                        break;
                    }
                    i += 1;
                }
            }
            b'/' if i + 1 < len => {
                if bytes[i + 1] == b'/' {
                    // 行コメント: 行末までスキップ
                    i += 2;
                    while i < len && bytes[i] != b'\n' {
                        i += 1;
                    }
                } else if bytes[i + 1] == b'*' {
                    // ブロックコメント: */ までスキップ
                    i += 2;
                    while i + 1 < len {
                        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    None
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
    serde_json_lenient::from_str::<AppConfigFile>(&content)
        .map_err(|e| format!("設定ファイルのパースに失敗 ({}): {}", path.display(), e))
}

/// ファイルの設定値をデフォルト値にマージする
fn merge_config(file: AppConfigFile, config_path: &PathBuf) -> AppConfig {
    // 設定ファイルの親ディレクトリ（相対パス解決の基準）
    let config_dir = config_path.parent().unwrap_or(Path::new("."));

    let mode = file.mode.unwrap_or(Mode::Local);

    // local 設定の解決
    let local_config = {
        let watch_dir_raw = file
            .local
            .as_ref()
            .and_then(|l| l.watch_dir.clone())
            .unwrap_or_else(|| {
                let home = dirs_home().unwrap_or_else(|| "/tmp".into());
                home.join("ndp").display().to_string()
            });
        let watch_dir = resolve_path(&watch_dir_raw, config_dir);

        let dj_id = file
            .local
            .as_ref()
            .and_then(|l| l.dj_id.clone())
            .unwrap_or_else(|| "dj-000".to_string());

        LocalConfig { watch_dir, dj_id }
    };

    // web 設定の解決
    let web_config = {
        let server_url = file
            .web
            .as_ref()
            .and_then(|w| w.server_url.clone())
            .unwrap_or_else(|| "http://localhost:8080".to_string());

        WebConfig { server_url }
    };

    // 背景画像設定の解決
    let background_image = file.background_image.map(|bg| {
        let base_dir = resolve_path(&bg.base_dir, config_dir);
        BackgroundImageConfig {
            base_dir,
            path: bg.path,
        }
    });

    AppConfig {
        mode,
        local: local_config,
        web: web_config,
        enable_comments: file.enable_comments.unwrap_or(false),
        show_tags: file.show_tags.unwrap_or(true),
        event_name: file.event_name,
        show_event_name: file.show_event_name.unwrap_or(true),
        background_image,
        config_path: config_path.display().to_string(),
    }
}

/// パス文字列を解決する
///
/// 1. `~` で始まる場合はホームディレクトリに展開する
/// 2. 展開後のパスが相対パスの場合、`base_dir` を基準に解決する
/// 3. 絶対パスの場合はそのまま返す
fn resolve_path(raw: &str, base_dir: &Path) -> String {
    // ~ をホームディレクトリに展開
    let expanded = shellexpand::tilde(raw).to_string();
    let path = Path::new(&expanded);

    if path.is_absolute() {
        expanded
    } else {
        // 相対パスは設定ファイルのディレクトリを基準に解決
        base_dir.join(path).display().to_string()
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
