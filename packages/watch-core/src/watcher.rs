use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::parser::{build_dj_state, parse_ready};
use crate::types::WatchEvent;

/// ベースディレクトリを再帰的に監視し、.ready の変更を検知して WatchEvent を発行する
pub struct DirWatcher {
    base_dir: PathBuf,
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<Result<Event, notify::Error>>,
}

impl DirWatcher {
    /// 新しい DirWatcher を作成し、監視を開始する
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self, WatcherError> {
        let base_dir = base_dir.as_ref().to_path_buf();

        if !base_dir.is_dir() {
            return Err(WatcherError::NotADirectory(base_dir));
        }

        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )
        .map_err(WatcherError::Notify)?;

        watcher
            .watch(&base_dir, RecursiveMode::Recursive)
            .map_err(WatcherError::Notify)?;

        log::info!("Watching directory: {}", base_dir.display());

        Ok(Self {
            base_dir,
            _watcher: watcher,
            rx,
        })
    }

    /// 次のイベントをブロッキングで待つ
    pub fn next_event(&self) -> Option<WatchEvent> {
        loop {
            match self.rx.recv() {
                Ok(Ok(event)) => {
                    if let Some(watch_event) = self.process_event(&event) {
                        return Some(watch_event);
                    }
                    // .ready 以外のイベントは無視してループ継続
                }
                Ok(Err(e)) => {
                    log::error!("Watcher error: {}", e);
                }
                Err(_) => {
                    // チャンネル切断 = watcher 終了
                    return None;
                }
            }
        }
    }

    /// タイムアウト付きで次のイベントを待つ
    pub fn next_event_timeout(&self, timeout: Duration) -> Option<WatchEvent> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match self.rx.recv_timeout(remaining) {
                Ok(Ok(event)) => {
                    if let Some(watch_event) = self.process_event(&event) {
                        return Some(watch_event);
                    }
                }
                Ok(Err(e)) => {
                    log::error!("Watcher error: {}", e);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => return None,
                Err(mpsc::RecvTimeoutError::Disconnected) => return None,
            }
        }
    }

    /// notify のイベントを処理し、.ready の変更なら WatchEvent に変換する
    fn process_event(&self, event: &Event) -> Option<WatchEvent> {
        // 作成 or 変更イベントのみ対象
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {}
            _ => return None,
        }

        // .ready ファイルへの変更か確認
        for path in &event.paths {
            if path.file_name().and_then(|n| n.to_str()) == Some(".ready") {
                return self.handle_ready_change(path);
            }
        }

        None
    }

    /// .ready ファイルの変更を処理する
    fn handle_ready_change(&self, ready_path: &Path) -> Option<WatchEvent> {
        let dj_dir = ready_path.parent()?;
        let dir_name = dj_dir.file_name()?.to_str()?.to_string();

        log::info!("Detected .ready change in: {}", dir_name);

        match parse_ready(ready_path) {
            Ok(manifest) => match build_dj_state(dj_dir, &manifest) {
                Ok(state) => Some(WatchEvent::TrackChanged(state)),
                Err(e) => Some(WatchEvent::Error {
                    dir_name,
                    message: e.to_string(),
                }),
            },
            Err(e) => Some(WatchEvent::Error {
                dir_name,
                message: e.to_string(),
            }),
        }
    }

    /// 監視中のベースディレクトリを返す
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("Not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("Notify error: {0}")]
    Notify(notify::Error),
}
