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
```

fish:
```fish
scp -i "$DEPLOY_SSH_KEY" etc/ndp-server.service "$DEPLOY_USER@$DEPLOY_HOST:/tmp/"
scp -i "$DEPLOY_SSH_KEY" /tmp/Caddyfile.generated "$DEPLOY_USER@$DEPLOY_HOST:/tmp/Caddyfile"
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

# Caddyfile 配置
sudo mv /tmp/Caddyfile /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

### 7. DNS 設定

Value Domain で A レコードを設定:
```
a ndp <Lightsail Static IP>
```

### 8. 動作確認

DNS 浸透後、ローカルから:
```bash
curl https://<DEPLOY_DOMAIN>/health
# 期待: {"status":"ok","version":"..."}
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
