# ADR-0039: viewer-web のデプロイ

## ステータス

Accepted

## コンテキスト

ブラウザ版 viewer (viewer-web、ADR-0038) は、`yarn build:web` で静的ファイル (`apps/viewer/dist-web/`) を出力する。ADR-0038 では、配信方法を決めず、ルートパス (`/`) での配信を前提としていた (#77)。

ndp-server は、Lightsail 上で Caddy (TLS 終端、`ndp.iss.ms`) の背後で動いている (ADR-0025)。viewer-web もこのホストから配信する。

## 決定

### 1. 配信する URL とファイル

- viewer-web は、ndp-server と同じホストの **`/v/`** 配下で配信する (例: `https://ndp.iss.ms/v/`)
- 静的ファイルは、サーバー上の **`/opt/ndp-server/viewer/`** に置き、Caddy が `file_server` で配信する
- API (`/api/*`) と `/health` は、従来どおり ndp-server に中継する (同一オリジン)

### 2. ビルド

- ビルドは `/v/` 配下で動くように、**相対パス**で出力する (`vite.config.ts` の `base` を、`--mode web` では `./` にする。Tauri 版は従来どおり `/`)。ADR-0038 の「ルートパス前提」は、これに置き換える
- 画像のフォールバック (`default-artwork.png`) などの絶対パスは、`import.meta.env.BASE_URL` 経由にする
- デプロイ用のビルドは `VITE_NDP_SERVER_URL` を空にする。同一オリジンなので、`/api/...` を相対パスで呼べる
- `deploy/ndp-server/build-viewer.sh` が、上記の設定でビルドする

### 3. 同期

- `dist-web/` の内容を、`/opt/ndp-server/viewer/` に **完全同期**する。手元にないファイルは、リモートからも削除する (`rsync --delete`)
- `deploy/ndp-server/deploy-viewer.sh` が行う。`--dry-run` で、変更内容を確認できる
- 誤って消さないための保護:
  - `dist-web/index.html` がなければ中止する (空のディレクトリを同期すると、リモートの内容がすべて消えるため)
  - 同期先が 2 階層以上の絶対パスでなければ中止する
- リモートでは `sudo rsync` で書き込む。同期先は root 所有のままで、Caddy (`caddy` ユーザー) は読み取りのみ
- 同期後に `https://{DEPLOY_DOMAIN}/v/` へアクセスして、配信を確認する (失敗しても警告のみ)

### 4. Caddyfile

```
redir /v /v/
handle_path /v/* {
    root * /opt/ndp-server/viewer
    encode zstd gzip
    @html path / /index.html
    header @html Cache-Control "no-cache"
    @assets path /assets/*
    header @assets Cache-Control "public, max-age=31536000, immutable"
    file_server
}
respond /favicon.ico 204
```

- `/v` は `/v/` へリダイレクトする (相対パスのアセットが解決されるように)
- `index.html` は `no-cache` にする。完全同期でハッシュ付きの古いアセットが消えるため、古い `index.html` をキャッシュしたままだと、消えたアセットを要求して壊れる
- ハッシュ付きの `assets/*` は、長期キャッシュにする
- `/favicon.ico` は、`204` で応える。ブラウザは自動で取得しにくる。ndp-server に中継すると 404 になり、既存の fail2ban のフィルタ (404 を 10 分間に 5 回で、その IP を 1 時間 ban) が数えてしまうため。会場の Wi-Fi のように、NAT の背後で IP を共有する環境では、全員が ban されかねない

## 理由

- **同一ホストの `/v/`**: 新しいドメインや証明書を増やさず、既存の Caddy と TLS をそのまま使える。同一オリジンなので、CORS を考えなくてよい
- **相対パスのビルド**: 配信パスに依存せず、`/v/` 以外 (ルートや別パス、`vite preview`) でも動く
- **rsync の完全同期**: 手元の `dist-web/` を正とし、古いファイルがリモートに残らない。ファイル数が多くなく、リモートの `viewer/` はこのスクリプトだけが管理する前提なので、削除の危険は小さい
- **root 所有で配置**: 配信にだけ使う静的ファイルなので、書き込める権限を与える必要がない

## 影響

- リモートに `rsync` が必要 (`sudo apt install -y rsync`)。スクリプトが事前に確認する
- Caddyfile の更新が必要。既存環境では、再生成して配置し、`caddy validate` で検証してから `reload` する
- `deploy-viewer.sh` / `build-viewer.sh` が増える。ndp-server のバイナリのデプロイ (`deploy.sh`) とは独立しており、どちらを先に行ってもよい
- viewer-web の `/v/` 配下で、存在しないパスを取得すると、404 になる。fail2ban のフィルタが、これも数える
- **反映の順序に注意が必要。** Caddyfile の更新前に `/v/` を開くと、ndp-server に中継されて 404 になる。`favicon.ico` と合わせて 10 分間に 5 回 404 になると、開いた IP が 1 時間 ban され、その IP から HTTPS が `ERR_CONNECTION_REFUSED` になる (SSH は対象外)。デプロイ時の確認で、実際にこの ban が起きた。反映は、Caddyfile → 静的ファイルの同期 → ブラウザで確認、の順に行う。手順と復旧方法は `deploy/ndp-server/README.md` に書いた
- ブラウザ版のバージョン表示は、ビルド時のコミットのもの。サーバー (ndp-server) のバージョンとは別
