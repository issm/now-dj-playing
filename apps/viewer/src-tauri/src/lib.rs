mod config;

use config::AppConfig;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Emitter, Manager};
use watch_core::{DirWatcher, DjProfile, WatchEvent};

/// アプリケーション設定のグローバルインスタンス
static APP_CONFIG: Mutex<Option<Result<AppConfig, String>>> = Mutex::new(None);

fn get_or_init_config() -> Result<AppConfig, String> {
    let mut guard = APP_CONFIG.lock().unwrap();
    if guard.is_none() {
        *guard = Some(config::load_config());
    }
    guard.as_ref().unwrap().clone()
}

/// 設定を再読み込みする
fn reload_config_inner() -> Result<AppConfig, String> {
    let mut guard = APP_CONFIG.lock().unwrap();
    let result = config::load_config();
    *guard = Some(result.clone());
    result
}

/// フロントエンドに送る楽曲情報
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackPayload {
    pub dir_name: String,
    pub dj_name: Option<String>,
    pub dj_logo_path: Option<String>,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub comment: Option<String>,
    pub artwork_path: Option<String>,
    pub updated_at: String,
}

/// フロントエンドに送るエラー情報
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub dir_name: String,
    pub message: String,
}

/// フロントエンドに送るバージョン情報
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    /// SemVer バージョン (例: "0.1.0")
    pub version: String,
    /// ビルドメタデータ (例: "20260704T123045.a1b2c3d")
    pub build_metadata: String,
    /// ビルド時刻 (例: "20260704T123045")
    pub build_timestamp: String,
    /// git commit hash (例: "a1b2c3d")
    pub commit_hash: String,
    /// フル表記 (例: "0.1.0+20260704T123045.a1b2c3d")
    pub full: String,
}

/// 背景画像一覧のレスポンス
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImageEntry {
    /// base_dir からの相対パス（ファイル名）
    pub path: String,
    /// 絶対パス（convertFileSrc 用）
    pub absolute_path: String,
}

/// フロントエンドに設定を返すコマンド
#[tauri::command]
fn get_app_config() -> Result<AppConfig, String> {
    get_or_init_config()
}

/// 設定を再読み込みして返すコマンド
#[tauri::command]
fn reload_config() -> Result<AppConfig, String> {
    reload_config_inner()
}

/// バージョン情報を返すコマンド
#[tauri::command]
fn get_version_info() -> VersionInfo {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let build_metadata = env!("BUILD_METADATA").to_string();
    let build_timestamp = env!("BUILD_TIMESTAMP").to_string();
    let commit_hash = env!("BUILD_COMMIT_HASH").to_string();
    let dev_suffix = env!("BUILD_DEV_SUFFIX").to_string();
    let full = format!("{}+{}{}", version, build_metadata, dev_suffix);

    VersionInfo {
        version,
        build_metadata,
        build_timestamp,
        commit_hash,
        full,
    }
}

/// 背景画像ディレクトリ内の画像ファイル一覧を返すコマンド（再帰的に探索）
#[tauri::command]
fn list_background_images() -> Result<Vec<BackgroundImageEntry>, String> {
    let config = get_or_init_config()?;

    let bg_config = config
        .background_image
        .ok_or_else(|| "背景画像ディレクトリが未設定です".to_string())?;

    let base_dir = PathBuf::from(&bg_config.base_dir);
    if !base_dir.is_dir() {
        return Err(format!(
            "背景画像ディレクトリが見つかりません: {}",
            bg_config.base_dir
        ));
    }

    let supported_extensions = ["png", "jpg", "jpeg", "webp"];
    let mut entries: Vec<BackgroundImageEntry> = Vec::new();

    fn collect_images(
        dir: &PathBuf,
        base_dir: &PathBuf,
        extensions: &[&str],
        entries: &mut Vec<BackgroundImageEntry>,
    ) -> Result<(), String> {
        let read_dir =
            std::fs::read_dir(dir).map_err(|e| format!("ディレクトリの読み込みに失敗: {}", e))?;

        for entry in read_dir.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                collect_images(&path, base_dir, extensions, entries)?;
            } else if path.is_file() {
                let is_image = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| extensions.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false);
                if is_image {
                    let relative = path
                        .strip_prefix(base_dir)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    entries.push(BackgroundImageEntry {
                        path: relative,
                        absolute_path: path.display().to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    collect_images(&base_dir, &base_dir, &supported_extensions, &mut entries)?;

    // パスでソート
    entries.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));

    Ok(entries)
}

/// 監視を開始するコマンド（設定はバックエンドから取得）
#[tauri::command]
fn start_watch(app: AppHandle) -> Result<String, String> {
    let config = get_or_init_config()?;
    let base_dir = &config.local.watch_dir;
    let dj_id = &config.local.dj_id;

    let path = PathBuf::from(base_dir);

    // ベースディレクトリがなければ作成する
    if !path.is_dir() {
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("ディレクトリの作成に失敗: {}: {}", base_dir, e))?;
    }

    let dj_id = dj_id.clone();
    let base_dir_display = base_dir.clone();
    let app_handle = app.clone();
    thread::spawn(move || {
        run_watcher(app_handle, path, dj_id);
    });

    Ok(format!("監視を開始しました: {}", base_dir_display))
}

/// モニタウィンドウを開くコマンド（既に開いている場合はフォーカス）
#[tauri::command]
fn open_monitor(app: AppHandle) -> Result<(), String> {
    // 既にモニタウィンドウが存在する場合はフォーカスのみ
    if let Some(window) = app.get_webview_window("monitor") {
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // 新しいモニタウィンドウを作成
    WebviewWindowBuilder::new(&app, "monitor", tauri::WebviewUrl::App("/".into()))
        .title("ndp-monitor")
        .inner_size(240.0, 280.0)
        .resizable(true)
        .always_on_top(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// watcher のメインループ
fn run_watcher(app: AppHandle, base_dir: PathBuf, dj_id: String) {
    let dj_dir = base_dir.join(&dj_id);

    // 起動時に既存の .ready をスキャンして即座に emit
    if dj_dir.is_dir() {
        let ready_path = dj_dir.join(".ready");
        if ready_path.is_file() {
            if let Ok(manifest) = watch_core::parse_ready(&ready_path) {
                if let Ok(state) = watch_core::build_dj_state(&dj_dir, &manifest) {
                    emit_track_changed(&app, state);
                }
            }
        }
    }

    // ベースディレクトリを再帰的に監視（DJ ディレクトリが後から作られても検知可能）
    let watcher = match DirWatcher::new(&base_dir) {
        Ok(w) => w,
        Err(e) => {
            log::error!("Watcher の起動に失敗: {}", e);
            let _ = app.emit(
                "watch-error",
                ErrorPayload {
                    dir_name: base_dir.display().to_string(),
                    message: e.to_string(),
                },
            );
            return;
        }
    };

    log::info!(
        "Watcher 起動 (base: {}, dj_id: {})",
        base_dir.display(),
        dj_id
    );

    loop {
        match watcher.next_event() {
            Some(WatchEvent::TrackChanged(state)) => {
                // DJ ID でフィルタリング
                if state.dir_name == dj_id {
                    emit_track_changed(&app, state);
                } else {
                    log::debug!("無視 (dj_id不一致): {}", state.dir_name);
                }
            }
            Some(WatchEvent::DjRemoved { dir_name }) => {
                if dir_name == dj_id {
                    log::info!("DJ removed: {}", dir_name);
                    let _ = app.emit("dj-removed", dir_name);
                }
            }
            Some(WatchEvent::Error { dir_name, message }) => {
                if dir_name == dj_id {
                    log::warn!("Watch error in {}: {}", dir_name, message);
                    let _ = app.emit("watch-error", ErrorPayload { dir_name, message });
                }
            }
            None => {
                log::info!("Watcher stopped");
                break;
            }
        }
    }
}

/// DjState を TrackPayload に変換して emit する
fn emit_track_changed(app: &AppHandle, state: watch_core::DjState) {
    let (dj_name, dj_logo_path) = match &state.profile {
        DjProfile::Name(name) => (Some(name.clone()), None),
        DjProfile::Logo(path) => (None, Some(path.display().to_string())),
    };

    let payload = TrackPayload {
        dir_name: state.dir_name,
        dj_name,
        dj_logo_path,
        title: state.now_playing.title,
        artist: state.now_playing.artist,
        album: state.now_playing.album,
        comment: state.now_playing.comment,
        artwork_path: state.artwork_path.map(|p| p.display().to_string()),
        updated_at: state.now_playing.updated_at.to_rfc3339(),
    };

    log::info!("TrackChanged: {} - {}", payload.artist, payload.title);
    let _ = app.emit("track-changed", payload);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // 設定を早期にロードしてログに表示
            match get_or_init_config() {
                Ok(config) => log::info!("App config loaded: {:?}", config),
                Err(e) => log::error!("App config error: {}", e),
            }

            // バージョン情報をログに表示
            let version_info = get_version_info();
            log::info!("App version: {}", version_info.full);

            // メインウィンドウが閉じられたらアプリ全体を終了する
            let main_window = app.get_webview_window("main").unwrap();
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::Destroyed = event {
                    std::process::exit(0);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_watch,
            get_app_config,
            reload_config,
            open_monitor,
            get_version_info,
            list_background_images,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
