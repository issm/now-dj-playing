use std::process::Command;

fn main() {
    // git commit hash (short) を取得
    let commit_hash = get_git_short_hash().unwrap_or_else(|| "unknown".to_string());

    // 対象コミットのタイムスタンプ (UTC) を yyyymmddThhmmss 形式で取得
    let build_timestamp = get_commit_timestamp().unwrap_or_else(|| "00000000T000000".to_string());

    // ビルドメタデータ: {yyyymmddThhmmss}.{commit_hash}
    let build_metadata = format!("{}.{}", build_timestamp, commit_hash);

    // デバッグビルド時は "-dev" サフィックスを付与
    let dev_suffix = if cfg!(debug_assertions) { "-dev" } else { "" };

    // フルバージョン文字列: {version}+{build_metadata}{dev_suffix}
    let version = env!("CARGO_PKG_VERSION");
    let version_full = format!("{}+{}{}", version, build_metadata, dev_suffix);

    println!("cargo:rustc-env=BUILD_METADATA={}", build_metadata);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp);
    println!("cargo:rustc-env=BUILD_COMMIT_HASH={}", commit_hash);
    println!("cargo:rustc-env=BUILD_DEV_SUFFIX={}", dev_suffix);
    println!("cargo:rustc-env=BUILD_VERSION_FULL={}", version_full);
}

/// HEAD コミットのコミッター日時を yyyymmddThhmmss (UTC) 形式で取得する
fn get_commit_timestamp() -> Option<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%cd", "--date=format:%Y%m%dT%H%M%S"])
        .env("TZ", "UTC")
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// git の short commit hash を取得する
fn get_git_short_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}
