//! ローカルディレクトリ監視の動作確認用サンプル
//!
//! 使い方:
//!   cargo run --example watch_local -- ./sandbox
//!
//! 別ターミナルから sandbox 内にファイルを書き込むと、検知結果が表示される。

use std::env;
use std::path::PathBuf;

use watch_core::{DirWatcher, DjProfile, WatchEvent};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let base_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./sandbox"));

    println!("=== Now DJ Playing - Local Watcher ===");
    println!("Watching: {}", base_dir.display());
    println!("Press Ctrl+C to stop.\n");

    let watcher = match DirWatcher::new(&base_dir) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to start watcher: {}", e);
            std::process::exit(1);
        }
    };

    loop {
        match watcher.next_event() {
            Some(WatchEvent::TrackChanged(state)) => {
                println!("──────────────────────────────────");
                println!("🎵 Track Changed!");
                println!("   DJ: {}", format_profile(&state.profile));
                println!("   Title: {}", state.now_playing.title);
                println!("   Artist: {}", state.now_playing.artist);
                if let Some(album) = &state.now_playing.album {
                    println!("   Album: {}", album);
                }
                if let Some(artwork) = &state.artwork_path {
                    println!("   Artwork: {}", artwork.display());
                } else {
                    println!("   Artwork: (none)");
                }
                println!("   Updated: {}", state.now_playing.updated_at);
                println!("──────────────────────────────────\n");
            }
            Some(WatchEvent::DjRemoved { dir_name }) => {
                println!("❌ DJ removed: {}\n", dir_name);
            }
            Some(WatchEvent::Error { dir_name, message }) => {
                eprintln!("⚠️  Error in {}: {}\n", dir_name, message);
            }
            None => {
                println!("Watcher stopped.");
                break;
            }
        }
    }
}

fn format_profile(profile: &DjProfile) -> String {
    match profile {
        DjProfile::Name(name) => name.clone(),
        DjProfile::Logo(path) => format!("[logo: {}]", path.display()),
    }
}
