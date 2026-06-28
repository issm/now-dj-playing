# ADR-0006: Viewer 改善 (asset プロトコル、DJ ID 指定、キャッシュバスティング、ウィンドウサイズ)

## ステータス

Accepted

## コンテキスト

viewer アプリの開発中に以下の問題が発覚した。

1. `convertFileSrc` でローカルファイル（アートワーク等）を WebView に表示できない
2. 監視対象が全 DJ ディレクトリだが、特定の DJ に絞りたいケースがある
3. 同じファイルパス (`artwork.png`) の内容が変わっても WebView がキャッシュを返す
4. デスクトップ開発時のウィンドウサイズが iPad の実際の表示と乖離している

## 決定

### 1. Asset プロトコルの有効化

`tauri.conf.json` に `assetProtocol` 設定を追加し、`Cargo.toml` に `protocol-asset` feature を追加。

```json
"security": {
  "csp": null,
  "assetProtocol": {
    "enable": true,
    "scope": ["**"]
  }
}
```

これにより `convertFileSrc` が返す `asset://` URL でローカルファイルを WebView から読み込める。

### 2. DJ ID による監視対象の制御

- `VITE_DEFAULT_DJ_ID` 環境変数（デフォルト: `dj-000`）で監視対象の DJ ディレクトリを指定
- `start_watch` コマンドに `dj_id` パラメータを追加
- 監視対象は `{VITE_WATCH_DIR}/{VITE_DEFAULT_DJ_ID}/` となる
- publisher 側の `--id` と対応させて使用する

### 3. アートワークのキャッシュバスティング

`convertFileSrc` で生成した URL にクエリパラメータ `?t={updatedAt}` を付与。曲が変わるたびに `updatedAt` が異なるため、WebView が常に最新の画像を取得する。

### 4. ウィンドウサイズを iPad 横位置に合わせる

デスクトップ開発時のウィンドウサイズを 1194x834 に設定。iPad Air / iPad Pro 11" の横位置論理解像度に相当し、実機に近い見た目で開発できる。

### 5. デフォルトアートワーク

アートワークが存在しない場合のフォールバックとして、`public/default-artwork.png`（800x800、"NO ARTWORK" テキスト）を表示。画像の読み込みエラー時も `onError` で同画像にフォールバックする。

## 理由

- asset プロトコルを有効にしないと、Tauri 2 の WebView からローカルファイルにアクセスできない
- DJ ID 指定により、複数 DJ が同一ベースディレクトリを共有する環境でも特定の DJ だけをフォローできる
- キャッシュバスティングは WebView の画像キャッシュに対する最もシンプルな回避策
- iPad 横位置の比率でウィンドウを表示することで、レイアウト崩れの早期発見が可能

## 影響

- `assetProtocol.scope` を `["**"]` としているため、全パスへのアクセスが許可される。本番では必要最小限に絞ることを検討すべき
- `VITE_DEFAULT_DJ_ID` を publisher 側の `--id` と合わせる運用が必要
