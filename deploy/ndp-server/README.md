# ndp-server デプロイ

ndp-server を AWS Lightsail (Debian 13) にデプロイするためのスクリプト・設定ファイル群。ブラウザ版 viewer (viewer-web) も、同じホストの `/v/` で配信する。

## 前提条件

### ローカル（開発マシン）

- Rust toolchain
- `cargo-zigbuild` (`cargo install cargo-zigbuild`)
- `zig` (zigbuild が内部で使用)
- musl ターゲット (`rustup target add x86_64-unknown-linux-musl`)
- viewer-web のデプロイ: Node.js / yarn (`apps/viewer` の依存インストール済み)、`rsync`

### リモート（Lightsail インスタンス）

- Debian 13
- SSH アクセス可能
- ファイアウォールで TCP 22, 80, 443 を開放
- viewer-web のデプロイ: `rsync` (`sudo apt install -y rsync`)

## ディレクトリ構成

```
deploy/ndp-server/
├── build.sh              # クロスコンパイル
├── deploy.sh             # バイナリ転送 + サービス再起動
├── build-viewer.sh       # viewer-web のビルド
├── deploy-viewer.sh      # viewer-web の同期 (rsync)
├── dist/                 # ビルド成果物（.gitignore 対象）
├── etc/
│   ├── Caddyfile         # Caddy 設定テンプレート
│   ├── fail2ban/
│   │   ├── caddy-ndp.conf   # fail2ban filter 定義
│   │   └── caddy-ndp.local  # fail2ban jail 定義
│   └── ndp-server.service # systemd ユニットファイル
├── .env.example          # デプロイ設定テンプレート
└── README.md
```

## 初回セットアップ

### 1. ローカル設定（ローカルで実行）

```bash
cp .env.example .env
# .env を編集して DEPLOY_HOST, DEPLOY_SSH_KEY, DEPLOY_DOMAIN 等を設定
```

### 2. Caddyfile のドメイン置換（ローカルで実行）

Caddyfile テンプレートの `{$DOMAIN}` を実際のドメインに置換したファイルを生成する:

bash:
```bash
source .env
sed "s/{\\\$DOMAIN}/$DEPLOY_DOMAIN/" etc/Caddyfile > /tmp/Caddyfile.generated
```

fish:
```fish
# .env を fish 変数として読み込む
for line in (grep -v '^\s*#' .env | grep -v '^\s*$')
    set -l kv (string replace -r '^export\s+' '' -- $line)
    set -l key (string split -m1 '=' -- $kv)[1]
    set -l val (string split -m1 '=' -- $kv)[2]
    set -gx $key $val
end

sed "s/{\\\$DOMAIN}/$DEPLOY_DOMAIN/" etc/Caddyfile > /tmp/Caddyfile.generated
```

### 3. 設定ファイルをリモートに転送（ローカルで実行）

bash:
```bash
scp -i "$DEPLOY_SSH_KEY" etc/ndp-server.service "${DEPLOY_USER}@${DEPLOY_HOST}:/tmp/"
scp -i "$DEPLOY_SSH_KEY" /tmp/Caddyfile.generated "${DEPLOY_USER}@${DEPLOY_HOST}:/tmp/Caddyfile"
scp -i "$DEPLOY_SSH_KEY" etc/fail2ban/caddy-ndp.conf "${DEPLOY_USER}@${DEPLOY_HOST}:/tmp/"
scp -i "$DEPLOY_SSH_KEY" etc/fail2ban/caddy-ndp.local "${DEPLOY_USER}@${DEPLOY_HOST}:/tmp/"
```

fish:
```fish
scp -i "$DEPLOY_SSH_KEY" etc/ndp-server.service "$DEPLOY_USER@$DEPLOY_HOST:/tmp/"
scp -i "$DEPLOY_SSH_KEY" /tmp/Caddyfile.generated "$DEPLOY_USER@$DEPLOY_HOST:/tmp/Caddyfile"
scp -i "$DEPLOY_SSH_KEY" etc/fail2ban/caddy-ndp.conf "$DEPLOY_USER@$DEPLOY_HOST:/tmp/"
scp -i "$DEPLOY_SSH_KEY" etc/fail2ban/caddy-ndp.local "$DEPLOY_USER@$DEPLOY_HOST:/tmp/"
```

### 4. 専用ユーザ作成（リモートで実行）

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin ndp
sudo mkdir -p /opt/ndp-server
sudo chown ndp:ndp /opt/ndp-server
```

### 5. systemd サービス登録（リモートで実行）

```bash
sudo mv /tmp/ndp-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable ndp-server
```

### 6. Caddy インストール・設定（リモートで実行）

```bash
# Caddy 公式リポジトリ追加
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
    | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
    | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install caddy

# アクセスログディレクトリ・ファイル作成（Caddyfile の log 出力先）
sudo mkdir -p /var/log/caddy
sudo touch /var/log/caddy/access.log
sudo chown -R caddy:caddy /var/log/caddy

# Caddyfile 配置
sudo mv /tmp/Caddyfile /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

### 7. fail2ban セットアップ（リモートで実行）

不正リクエストを送信する IP を自動 ban する。

```bash
# fail2ban インストール
sudo apt install -y fail2ban

# filter 配置
sudo cp /tmp/caddy-ndp.conf /etc/fail2ban/filter.d/caddy-ndp.conf

# jail 配置
sudo cp /tmp/caddy-ndp.local /etc/fail2ban/jail.d/caddy-ndp.local

# fail2ban 起動・有効化
sudo systemctl enable fail2ban
sudo systemctl restart fail2ban

# 動作確認
sudo fail2ban-client status caddy-ndp
```

### 8. DNS 設定

Value Domain で A レコードを設定:
```
a ndp <Lightsail Static IP>
```

### 9. 動作確認

DNS 浸透後、ローカルから:
```bash
curl https://<DEPLOY_DOMAIN>/health
# 期待: {"status":"ok","version":"..."}
```

## 既存環境への fail2ban 追加

すでに稼働中の環境に fail2ban を追加する場合の手順。

### 1. Caddyfile 再生成・転送（ローカルで実行）

Caddyfile に `log` ディレクティブが追加されているため、再生成して転送する:

bash:
```bash
source .env
sed "s/{\\\$DOMAIN}/$DEPLOY_DOMAIN/" etc/Caddyfile > /tmp/Caddyfile.generated
scp -i "$DEPLOY_SSH_KEY" /tmp/Caddyfile.generated "${DEPLOY_USER}@${DEPLOY_HOST}:/tmp/Caddyfile"
scp -i "$DEPLOY_SSH_KEY" etc/fail2ban/caddy-ndp.conf "${DEPLOY_USER}@${DEPLOY_HOST}:/tmp/"
scp -i "$DEPLOY_SSH_KEY" etc/fail2ban/caddy-ndp.local "${DEPLOY_USER}@${DEPLOY_HOST}:/tmp/"
```

fish:
```fish
for line in (grep -v '^\s*#' .env | grep -v '^\s*$')
    set -l kv (string replace -r '^export\s+' '' -- $line)
    set -l key (string split -m1 '=' -- $kv)[1]
    set -l val (string split -m1 '=' -- $kv)[2]
    set -gx $key $val
end

sed "s/{\\\$DOMAIN}/$DEPLOY_DOMAIN/" etc/Caddyfile > /tmp/Caddyfile.generated
scp -i "$DEPLOY_SSH_KEY" /tmp/Caddyfile.generated "$DEPLOY_USER@$DEPLOY_HOST:/tmp/Caddyfile"
scp -i "$DEPLOY_SSH_KEY" etc/fail2ban/caddy-ndp.conf "$DEPLOY_USER@$DEPLOY_HOST:/tmp/"
scp -i "$DEPLOY_SSH_KEY" etc/fail2ban/caddy-ndp.local "$DEPLOY_USER@$DEPLOY_HOST:/tmp/"
```

### 2. Caddy アクセスログ有効化（リモートで実行）

```bash
# ログディレクトリ・ファイル作成
sudo mkdir -p /var/log/caddy
sudo touch /var/log/caddy/access.log
sudo chown -R caddy:caddy /var/log/caddy

# Caddyfile 更新・reload
sudo mv /tmp/Caddyfile /etc/caddy/Caddyfile
sudo systemctl reload caddy

# ログ出力を確認
sleep 2
ls -la /var/log/caddy/access.log
```

### 3. fail2ban インストール・設定（リモートで実行）

```bash
# fail2ban インストール
sudo apt install -y fail2ban

# filter 配置
sudo cp /tmp/caddy-ndp.conf /etc/fail2ban/filter.d/caddy-ndp.conf

# jail 配置
sudo cp /tmp/caddy-ndp.local /etc/fail2ban/jail.d/caddy-ndp.local

# fail2ban 起動・有効化
sudo systemctl enable fail2ban
sudo systemctl restart fail2ban

# 動作確認
sudo fail2ban-client status caddy-ndp
```

## デプロイ手順（2 回目以降）

### 1. ビルド（ローカルで実行）

```bash
./build.sh
```

### 2. デプロイ（ローカルで実行）

```bash
./deploy.sh
```

## viewer-web のデプロイ

ブラウザ版 viewer を、ndp-server と同じホストの `/v/` で配信する（例: `https://ndp.iss.ms/v/`）。設計は [ADR-0039](../../docs/adr/0039-20261103-viewer-web-deploy.md) を参照。

- 静的ファイルは、リモートの `/opt/ndp-server/viewer/` に置く（`DEPLOY_VIEWER_DIR` で変更できる）
- API (`/api/*`) と `/health` は、従来どおり ndp-server に中継する（同一オリジン）
- ndp-server のバイナリのデプロイ（`deploy.sh`）とは独立している

> **注意: Caddyfile と静的ファイルを反映してから、ブラウザで `/v/` を開く。**
> 反映前に `/v/` を開くと、リクエストが ndp-server に中継されて 404 になる（ブラウザが自動で取りに行く `/favicon.ico` も 404 になる）。fail2ban は、404 を 10 分間に 5 回数えると、その IP を 1 時間 ban する。ban されると、その IP からは HTTPS が `ERR_CONNECTION_REFUSED` になる（SSH は影響を受けない）。復旧は、後述の「ブラウザで `ERR_CONNECTION_REFUSED`」を参照。

### 初回（または Caddyfile を更新したとき）

#### 1. リモートに rsync を入れる（リモートで実行）

```bash
sudo apt install -y rsync
```

#### 2. Caddyfile を再生成して転送する（ローカルで実行）

「既存環境への fail2ban 追加」の「1. Caddyfile 再生成・転送」と同じ手順で、`/tmp/Caddyfile` として転送する（`/v/` の配信と、`/favicon.ico` の設定が追加されている）。

#### 3. 検証して反映する（リモートで実行）

```bash
# 構文の検証（エラーがあれば反映しない）
sudo caddy validate --config /tmp/Caddyfile --adapter caddyfile

# 反映
sudo mv /tmp/Caddyfile /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

### 更新のたび

#### 1. ビルド（ローカルで実行）

```bash
./build-viewer.sh
```

`apps/viewer/dist-web/` に出力される。サーバー URL は空（同一オリジン）でビルドする。

#### 2. 同期（ローカルで実行）

```bash
# 変更内容の確認のみ（リモートは変更しない）
./deploy-viewer.sh --dry-run

# 同期する
./deploy-viewer.sh
```

`dist-web/` の内容を、リモートの `/opt/ndp-server/viewer/` に**完全同期**する。手元にないファイルは、リモートからも削除される（`rsync --delete`）。`dist-web/index.html` がないときと、同期先のパスが不正なときは、何も変更せずに中止する。

### 動作確認

```bash
curl -sI https://<DEPLOY_DOMAIN>/v/ | head -3
# 期待: HTTP/2 200、cache-control: no-cache

curl -s -o /dev/null -w "%{http_code}\n" https://<DEPLOY_DOMAIN>/v
# 期待: 308 または 302（/v/ へのリダイレクト）

curl -s -o /dev/null -w "%{http_code}\n" https://<DEPLOY_DOMAIN>/favicon.ico
# 期待: 204
```

ブラウザで `https://<DEPLOY_DOMAIN>/v/` を開き、viewer の `view` のコードで Connect できることを確認する。

## 運用コマンド

```bash
# ステータス確認
ssh admin@HOST sudo systemctl status ndp-server

# ログ確認
ssh admin@HOST sudo journalctl -u ndp-server -f

# 手動再起動
ssh admin@HOST sudo systemctl restart ndp-server

# Caddy ログ確認
ssh admin@HOST sudo journalctl -u caddy -f

# Caddy アクセスログ確認
ssh admin@HOST sudo tail -f /var/log/caddy/access.log

# fail2ban ステータス確認
ssh admin@HOST sudo fail2ban-client status caddy-ndp

# ban 中の IP 一覧
ssh admin@HOST sudo fail2ban-client status caddy-ndp | grep "Banned IP"

# 手動 unban
ssh admin@HOST sudo fail2ban-client set caddy-ndp unbanip <IP>
```

## トラブルシューティング

### HTTPS 接続で `tlsv1 alert internal error`

Caddy が TLS 証明書を取得できていない。以下を確認:
- Lightsail ファイアウォールで TCP 80, 443 が開放されているか
- Caddyfile のドメイン名が正しいか (`cat /etc/caddy/Caddyfile`)
- Caddy ログ: `sudo journalctl -u caddy --since "5 min ago"`

### デプロイ後にサービスが起動しない

- ログ確認: `sudo journalctl -u ndp-server -e`
- バイナリの実行権限: `ls -la /opt/ndp-server/ndp-server`
- ポート競合: `ss -tlnp | grep 8080`

### fail2ban が動作しない

- ステータス確認: `sudo systemctl status fail2ban`
- jail が有効か: `sudo fail2ban-client status`
- filter テスト: `sudo fail2ban-regex /var/log/caddy/access.log /etc/fail2ban/filter.d/caddy-ndp.conf`
- ログ確認: `sudo tail -f /var/log/fail2ban.log`

### viewer-web が表示されない

- `/v/` が 404: 同期されているか (`ssh admin@HOST ls -la /opt/ndp-server/viewer/`)、Caddyfile が更新されているか (`ssh admin@HOST grep -n "/v/" /etc/caddy/Caddyfile`)
- `/v/` が白画面: ブラウザの開発者ツールで、`/v/assets/...` が 404 になっていないか確認する。古い `index.html` が残っていれば、強制リロードする
- Connect に失敗する: `https://<DEPLOY_DOMAIN>/api/...` に届いているか（`VITE_NDP_SERVER_URL` を空でビルドしたか。`./build-viewer.sh` を使う）
- Caddy が読めない: 同期先が root 所有で、`caddy` ユーザーが読めるか (`ls -ld /opt/ndp-server /opt/ndp-server/viewer`)。`/opt/ndp-server` のモードが 750 など、他のユーザーに閉じていないか
- `/v/` で 404 が続くと、fail2ban が ban することがある。ban された場合は、手動で unban する

### deploy-viewer.sh が `rsync` で失敗する

- リモートに `rsync` がない: `sudo apt install -y rsync`
- ローカルの `rsync` が macOS 標準 (openrsync) でも動作する（`--chmod` や `--chown` は使っていない）

### ブラウザで `ERR_CONNECTION_REFUSED`（SSH は繋がる）

fail2ban に ban された可能性が高い（ban の対象は TCP 80/443 のみで、SSH は対象外）。ポート 80/443 が、タイムアウトではなく即座に拒否されるのが特徴。

```bash
# ban されている IP を確認
ssh admin@HOST sudo fail2ban-client status caddy-ndp | grep "Banned IP"

# ban された経緯（Found の連続と Ban）
ssh admin@HOST sudo grep caddy-ndp /var/log/fail2ban.log | tail -n 20

# 解除
ssh admin@HOST sudo fail2ban-client set caddy-ndp unbanip <IP>
```

よくある原因:

- Caddyfile を反映する前に `/v/` を開いた（404 が連続して数えられる）。反映の順序は、「viewer-web のデプロイ」を参照
- 存在しないパスを、短時間に何度も開いた（`/favicon.ico` を含む）

自分の IP を ban の対象外にしたいときは、サーバー上の jail 設定（`/etc/fail2ban/jail.d/caddy-ndp.local`）に `ignoreip = 127.0.0.1/8 ::1 <自分の IP>` を足して、`sudo systemctl restart fail2ban` する。IP が変わる環境では、使いにくい。
