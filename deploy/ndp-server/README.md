# ndp-server デプロイ

ndp-server を AWS Lightsail (Debian 13) にデプロイするためのスクリプト・設定ファイル群。

## 前提条件

### ローカル（開発マシン）

- Rust toolchain
- `cargo-zigbuild` (`cargo install cargo-zigbuild`)
- `zig` (zigbuild が内部で使用)
- musl ターゲット (`rustup target add x86_64-unknown-linux-musl`)

### リモート（Lightsail インスタンス）

- Debian 13
- SSH アクセス可能
- ファイアウォールで TCP 22, 80, 443 を開放

## ディレクトリ構成

```
deploy/ndp-server/
├── build.sh              # クロスコンパイル
├── deploy.sh             # バイナリ転送 + サービス再起動
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
