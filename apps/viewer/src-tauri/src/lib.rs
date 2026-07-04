use serde::Serialize;
use std::path::PathBuf;
use std::thread;
use tauri::{AppHandle, Emitter};
use watch_core::{DirWatcher, DjProfile, WatchEvent};

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

/// 監視ディレクトリを指定して watcher を開始するコマンド
#[tauri::command]
fn start_watch(app: AppHandle, base_dir: String, dj_id: Option<String>) -> Result<String, String> {
    let path = PathBuf::from(&base_dir);

    // ベースディレクトリがなければ作成する
    if !path.is_dir() {
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("ディレクトリの作成に失敗: {}: {}", base_dir, e))?;
    }

    let dj_id = dj_id.unwrap_or_else(|| "dj-000".to_string());
    let app_handle = app.clone();
    thread::spawn(move || {
        run_watcher(app_handle, path, dj_id);
    });

    Ok(format!("監視を開始しました: {}", base_dir))
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![start_watch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
