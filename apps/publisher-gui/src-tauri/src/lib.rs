use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use ndp_publish::config::{self, AppConfig};
use ndp_publish::local;
use ndp_publish::tags;
use ndp_publish::web;

/// フロントエンドに返す設定情報
#[derive(Debug, Serialize, Deserialize, Default)]
struct ConfigResponse {
    dj_name: String,
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

/// アプリ状態
struct AppState {
    config: Mutex<Option<AppConfig>>,
    config_path: Mutex<Option<PathBuf>>,
}

/// 設定ファイルを読み込む
#[tauri::command]
fn load_config(state: tauri::State<AppState>) -> Result<ConfigResponse, String> {
    // アプリ隣接の設定ファイルを最優先で探す
    let config = config::load_config(None).map_err(|e| e.to_string())?;

    let response = ConfigResponse {
        dj_name: config.dj_name().unwrap_or_default(),
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

    // 状態に保存
    *state.config_path.lock().unwrap() = config.config_path.clone();
    *state.config.lock().unwrap() = Some(config);

    Ok(response)
}

/// 設定ファイルを保存する
#[tauri::command]
fn save_config(
    state: tauri::State<AppState>,
    dj_name: String,
    dj_id: String,
    publish_base_dir: String,
    endpoint_url: String,
) -> Result<(), String> {
    let config_path = state.config_path.lock().unwrap().clone();
    let path = config_path.ok_or("設定ファイルのパスが不明です")?;

    let config_content = serde_json::json!({
        "dj_name": dj_name,
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

    Ok(())
}

/// セッションに参加する (web モード)
#[tauri::command]
fn join_session(
    state: tauri::State<AppState>,
    _endpoint_url: String,
    code: String,
    dj_name: String,
) -> Result<(), String> {
    let config_guard = state.config.lock().unwrap();
    let config = config_guard.as_ref().ok_or("設定が読み込まれていません")?;

    web::join_only(config, &dj_name, Some(&code)).map_err(|e| e.to_string())
}

/// ファイルを publish する
#[tauri::command]
fn publish(
    state: tauri::State<AppState>,
    file_path: String,
    mode: String,
    dj_name: String,
    _endpoint_url: String,
    code: Option<String>,
    dj_id: String,
    publish_base_dir: String,
) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.is_file() {
        return Err(format!("ファイルが見つかりません: {}", file_path));
    }

    let meta = tags::read_tags(path).map_err(|e| e.to_string())?;

    match mode.as_str() {
        "web" => {
            let config_guard = state.config.lock().unwrap();
            let config = config_guard.as_ref().ok_or("設定が読み込まれていません")?;

            web::publish_web(config, &meta, &dj_name, code.as_deref()).map_err(|e| e.to_string())
        }
        "local" => {
            let out = PathBuf::from(shellexpand::tilde(&publish_base_dir).to_string());
            let dj_name_opt = if dj_name.is_empty() {
                None
            } else {
                Some(dj_name.as_str())
            };
            local::publish_local(&meta, &out, &dj_id, dj_name_opt).map_err(|e| e.to_string())
        }
        _ => Err(format!("不明なモード: {}", mode)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .manage(AppState {
            config: Mutex::new(None),
            config_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            join_session,
            publish,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
