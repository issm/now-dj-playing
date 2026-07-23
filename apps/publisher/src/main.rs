mod config;
mod local;
mod tags;
mod web;

use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum Mode {
    Local,
    Web,
}

#[derive(Parser)]
#[command(name = "ndp-publish")]
#[command(
    about = "楽曲ファイルからタグ・アートワークを抽出し、共有ディレクトリまたは ndp-server に送信する"
)]
#[command(version = env!("BUILD_VERSION_FULL"), disable_version_flag = true)]
struct Cli {
    /// バージョン情報を表示する
    #[arg(short = 'v', long = "version", action = ArgAction::Version)]
    version: (),

    /// 設定ファイルのパス
    #[arg(short = 'c', long = "config-file")]
    config_file: Option<PathBuf>,

    /// 動作モード
    #[arg(short = 'm', long = "mode", value_enum, default_value = "local")]
    mode: Mode,

    /// 楽曲ファイルのパス (mp3, m4a)
    #[arg(short, long)]
    file: PathBuf,

    /// 出力先ベースディレクトリ（設定ファイルの local.publish_base_dir をオーバーライド）
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// DJ ディレクトリ名（設定ファイルの local.dj_id をオーバーライド）
    #[arg(long)]
    id: Option<String>,

    /// DJ 名（設定ファイルの dj_name をオーバーライド）
    #[arg(long)]
    dj_name: Option<String>,

    /// セッション参加用 6 桁コード（web モード、初回 join 時に必要）
    #[arg(long)]
    code: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 楽曲ファイルの存在確認
    if !cli.file.is_file() {
        anyhow::bail!("楽曲ファイルが見つかりません: {}", cli.file.display());
    }

    // 設定ファイルの読み込み
    let config = config::load_config(cli.config_file.as_deref())?;

    // -c 未指定時、ルックアップで見つかった設定ファイルのパスを表示
    if cli.config_file.is_none() {
        if let Some(path) = &config.config_path {
            eprintln!("  設定ファイル: {}", path.display());
        }
    }

    // タグを読み取る
    let meta = tags::read_tags(&cli.file)?;

    match cli.mode {
        Mode::Local => {
            let out = cli
                .out
                .or_else(|| config.local_publish_base_dir())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "出力先が未指定です。--out を指定するか、設定ファイルの local.publish_base_dir を設定してください"
                    )
                })?;
            let id = cli
                .id
                .or_else(|| config.local_dj_id())
                .unwrap_or_else(|| "dj-000".to_string());
            let dj_name = cli.dj_name.or_else(|| config.dj_name());

            local::publish_local(&meta, &out, &id, dj_name.as_deref())?;
        }
        Mode::Web => {
            let dj_name = cli
                .dj_name
                .or_else(|| config.dj_name())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "DJ 名が未指定です。--dj-name を指定するか、設定ファイルの dj_name を設定してください"
                    )
                })?;

            web::publish_web(&config, &meta, &dj_name, cli.code.as_deref())?;
        }
    }

    Ok(())
}
