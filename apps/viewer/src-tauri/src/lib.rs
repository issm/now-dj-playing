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
    if !path.is_dir() {
        return Err(format!("ディレクトリが存在しません: {}", base_dir));
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
    // 監視対象: {base_dir}/{dj_id}/
    let watch_dir = base_dir.join(&dj_id);

    // 起動時に既存の .ready をスキャンして即座に emit
    if watch_dir.is_dir() {
        let ready_path = watch_dir.join(".ready");
        if ready_path.is_file() {
            if let Ok(manifest) = watch_core::parse_ready(&ready_path) {
                if let Ok(state) = watch_core::build_dj_state(&watch_dir, &manifest) {
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
                        artwork_path: state.artwork_path.map(|p| p.display().to_string()),
                        updated_at: state.now_playing.updated_at.to_rfc3339(),
                    };

                    log::info!("既存トラック検出: {} - {}", payload.artist, payload.title);
                    let _ = app.emit("track-changed", payload);
                }
            }
        }
    }

    // 監視は DJ ディレクトリ自体を対象にする
    let watcher = match DirWatcher::new(&watch_dir) {
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

    log::info!("Watcher 起動: {}", base_dir.display());

    loop {
        match watcher.next_event() {
            Some(WatchEvent::TrackChanged(state)) => {
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
                    artwork_path: state.artwork_path.map(|p| p.display().to_string()),
                    updated_at: state.now_playing.updated_at.to_rfc3339(),
                };

                log::info!("TrackChanged: {} - {}", payload.artist, payload.title);
                let _ = app.emit("track-changed", payload);
            }
            Some(WatchEvent::DjRemoved { dir_name }) => {
                log::info!("DJ removed: {}", dir_name);
                let _ = app.emit("dj-removed", dir_name);
            }
            Some(WatchEvent::Error { dir_name, message }) => {
                log::warn!("Watch error in {}: {}", dir_name, message);
                let _ = app.emit("watch-error", ErrorPayload { dir_name, message });
            }
            None => {
                log::info!("Watcher stopped");
                break;
            }
        }
    }
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
