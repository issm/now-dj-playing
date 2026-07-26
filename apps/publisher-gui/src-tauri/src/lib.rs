use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::Manager;

use ndp_publish::config::{self, AppConfig};
use ndp_publish::local;
use ndp_publish::tags;
use ndp_publish::web;

/// フロントエンドに返す設定情報
#[derive(Debug, Serialize, Deserialize, Default)]
struct ConfigResponse {
    dj_name: String,
    dj_image: Option<String>,
    local: LocalConfigResponse,
    web: WebConfigResponse,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct LocalConfigResponse {
    dj_id: String,
    publish_base_dir: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct WebConfigResponse {
    endpoint_url: String,
}

/// publish 成功時にフロントに返すトラック情報
#[derive(Debug, Serialize)]
struct PublishResult {
    title: String,
    artist: String,
    /// アートワークの Base64 Data URI (存在しない場合は None)
    artwork: Option<String>,
}

/// アプリ状態
struct AppState {
    config: Mutex<Option<AppConfig>>,
    config_path: Mutex<Option<PathBuf>>,
}

/// アプリ隣接ディレクトリの候補を返す
///
/// macOS の .app バンドルの場合、バイナリは `Foo.app/Contents/MacOS/binary` にあるため、
/// `.app` の親ディレクトリも探索対象に含める。
fn app_adjacent_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
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
    }

    dirs
}

/// アプリ隣接の設定ファイルパスを探索する
fn app_adjacent_config_path() -> Option<PathBuf> {
    for dir in app_adjacent_dirs() {
        let path = dir.join("ndp-publish.config.json");
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// 設定ファイルを読み込む
///
/// ルックアップ優先順:
/// 1. アプリ隣接の ndp-publish.config.json
/// 2. ndp_publish::config::load_config の通常ルックアップ
#[tauri::command]
fn load_config(state: tauri::State<AppState>) -> Result<ConfigResponse, String> {
    let config = if let Some(adjacent) = app_adjacent_config_path() {
        if adjacent.is_file() {
            config::load_config(Some(&adjacent)).map_err(|e| e.to_string())?
        } else {
            config::load_config(None).map_err(|e| e.to_string())?
        }
    } else {
        config::load_config(None).map_err(|e| e.to_string())?
    };

    let response = ConfigResponse {
        dj_name: config.dj_name().unwrap_or_default(),
        dj_image: config.dj_image(),
        local: LocalConfigResponse {
            dj_id: config.local_dj_id().unwrap_or_else(|| "dj-000".to_string()),
            publish_base_dir: config
                .local_publish_base_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        },
        web: WebConfigResponse {
            endpoint_url: config.web_endpoint_url().unwrap_or_default(),
        },
    };

    *state.config_path.lock().unwrap() = config.config_path.clone();
    *state.config.lock().unwrap() = Some(config);

    Ok(response)
}

/// 設定ファイルを保存する
///
/// config_path が None の場合はアプリ隣接に新規作成する
#[tauri::command]
fn save_config(
    state: tauri::State<AppState>,
    dj_name: String,
    dj_image: Option<String>,
    dj_id: String,
    publish_base_dir: String,
    endpoint_url: String,
) -> Result<(), String> {
    let config_path = state.config_path.lock().unwrap().clone();
    let path = config_path
        .or_else(app_adjacent_config_path)
        .ok_or("設定ファイルの保存先を特定できません")?;

    let mut base = serde_json::json!({
        "dj_name": dj_name
    });
    if let Some(ref image_path) = dj_image {
        base["dj_image"] = serde_json::Value::String(image_path.clone());
    }

    let config_content = serde_json::json!({
        "base": base,
        "local": {
            "dj_id": dj_id,
            "publish_base_dir": publish_base_dir
        },
        "web": {
            "endpoint_url": endpoint_url
        }
    });

    let json = serde_json::to_string_pretty(&config_content).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("保存に失敗: {}", e))?;

    *state.config_path.lock().unwrap() = Some(path);

    Ok(())
}

/// セッションに参加する (web モード)
#[tauri::command]
fn join_session(
    state: tauri::State<AppState>,
    endpoint_url: String,
    code: String,
    dj_name: String,
    dj_image: Option<String>,
) -> Result<(), String> {
    let config_guard = state.config.lock().unwrap();
    let config = config_guard.as_ref().ok_or("設定が読み込まれていません")?;

    if !endpoint_url.is_empty() {
        std::env::set_var("NDP_PUBLISH_ENDPOINT_URL", &endpoint_url);
    }

    let dj_image_path = dj_image.map(|p| PathBuf::from(p));

    web::join_only(config, &dj_name, Some(&code), dj_image_path.as_deref())
        .map_err(|e| e.to_string())
}

/// セッションから離脱する (web モード)
#[tauri::command]
fn leave_session(state: tauri::State<AppState>) -> Result<(), String> {
    let config_guard = state.config.lock().unwrap();
    let config = config_guard.as_ref().ok_or("設定が読み込まれていません")?;

    web::leave(config).map_err(|e| e.to_string())
}

/// ファイルを publish する
#[tauri::command]
fn publish(
    state: tauri::State<AppState>,
    file_path: String,
    mode: String,
    dj_name: String,
    endpoint_url: String,
    code: Option<String>,
    dj_id: String,
    publish_base_dir: String,
) -> Result<PublishResult, String> {
    let path = Path::new(&file_path);
    if !path.is_file() {
        return Err(format!("ファイルが見つかりません: {}", file_path));
    }

    let meta = tags::read_tags(path).map_err(|e| e.to_string())?;

    // アートワークの Data URI を生成（フロント表示用）
    let artwork_data_uri = meta.artwork.as_ref().map(|art| {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&art.data);
        format!("data:{};base64,{}", art.mime, b64)
    });

    match mode.as_str() {
        "web" => {
            let config_guard = state.config.lock().unwrap();
            let config = config_guard.as_ref().ok_or("設定が読み込まれていません")?;

            if !endpoint_url.is_empty() {
                std::env::set_var("NDP_PUBLISH_ENDPOINT_URL", &endpoint_url);
            }

            web::publish_web(config, &meta, &dj_name, code.as_deref())
                .map_err(|e| e.to_string())?;
        }
        "local" => {
            let out = PathBuf::from(shellexpand::tilde(&publish_base_dir).to_string());
            let dj_name_opt = if dj_name.is_empty() {
                None
            } else {
                Some(dj_name.as_str())
            };
            let config_guard = state.config.lock().unwrap();
            let dj_image_path = config_guard.as_ref().and_then(|c| c.dj_image_path());
            local::publish_local(&meta, &out, &dj_id, dj_name_opt, dj_image_path.as_deref())
                .map_err(|e| e.to_string())?;
        }
        _ => return Err(format!("不明なモード: {}", mode)),
    }

    Ok(PublishResult {
        title: meta.title,
        artist: meta.artist,
        artwork: artwork_data_uri,
    })
}

/// バージョン情報を返す
#[derive(Debug, Serialize)]
struct VersionInfo {
    gui: String,
}

#[tauri::command]
fn get_version() -> VersionInfo {
    VersionInfo {
        gui: env!("BUILD_VERSION_FULL").to_string(),
    }
}

/// 設定ファイルのフォルダを OS のファイルマネージャで開く
#[tauri::command]
fn open_config_folder(state: tauri::State<AppState>) -> Result<(), String> {
    let config_path = state.config_path.lock().unwrap().clone();
    let path = config_path
        .or_else(app_adjacent_config_path)
        .ok_or("設定ファイルのパスが不明です")?;

    let folder = path.parent().ok_or("親ディレクトリが取得できません")?;

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("フォルダを開けません: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("フォルダを開けません: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("フォルダを開けません: {}", e))?;
    }

    Ok(())
}

/// ディレクトリが存在するか確認する
#[tauri::command]
fn check_dir_exists(path: String) -> bool {
    let expanded = shellexpand::tilde(&path).to_string();
    Path::new(&expanded).is_dir()
}

/// 画像ファイルを読み取り Base64 Data URI を返す
#[tauri::command]
fn read_image_as_data_uri(path: String) -> Result<String, String> {
    let file_path = Path::new(&path);
    if !file_path.is_file() {
        return Err(format!("ファイルが見つかりません: {}", path));
    }

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => return Err(format!("未対応の画像形式: {}", ext)),
    };

    let data = std::fs::read(file_path).map_err(|e| format!("読み込みに失敗: {}", e))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

    Ok(format!("data:{};base64,{}", mime, b64))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .manage(AppState {
            config: Mutex::new(None),
            config_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            join_session,
            leave_session,
            publish,
            check_dir_exists,
            read_image_as_data_uri,
            open_config_folder,
            get_version,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            // アプリ終了時: セッションファイルが存在すれば leave を試行（ベストエフォート）
            leave_on_exit(app_handle);
        }
    });
}

/// アプリ終了時にセッションから離脱する（ベストエフォート）
///
/// セッションファイルが存在しなければ何もしない。
/// ネットワークエラー等で失敗しても無視してアプリを終了させる。
fn leave_on_exit(app_handle: &tauri::AppHandle) {
    let state: tauri::State<AppState> = app_handle.state();
    let config_guard = state.config.lock().unwrap();
    let config = match config_guard.as_ref() {
        Some(c) => c,
        None => return,
    };

    // セッションファイルが存在するか確認
    let session_path = match config.session_file_path() {
        Some(p) if p.is_file() => p,
        _ => return,
    };

    eprintln!("  終了時 leave を試行: {}", session_path.display());

    // タイムアウト付きで leave を実行（最大 3 秒）
    let _ = std::thread::scope(|s| {
        let handle = s.spawn(|| {
            let _ = web::leave(config);
        });
        // 3 秒待って終了（join が返らなくてもスコープ終了でスレッドは破棄される）
        let _ = handle.join();
    });
}
