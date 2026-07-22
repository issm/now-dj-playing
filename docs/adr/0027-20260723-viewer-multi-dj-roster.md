# ADR-0027: viewer の複数 DJ 対応 (ロースター表示)

## ステータス

Accepted

## コンテキスト

ndp-server は複数 publisher (DJ) が同一セッションに参加する DJ リレー形式をサポートしている（ADR-0025）。サーバ側は `publisher_joined` イベントを SSE で配信済みだったが、viewer 側はこれを利用しておらず、常に直近の `track_changed` の DJ のみを表示していた。

複数 DJ がセッションに参加している状況を viewer のヘッダで可視化したい。また、local モードでも同じ表示ロジックを使い、n=1 として扱えるようにする。

## 決定

### ロースターの状態管理

`App.tsx` に参加中 DJ 一覧を `Map<string, string>`（`id → 表示名`）として保持する。

- `id`: local モードでは `dirName` (dj_id)、web モードでは `publisher_id`
- 追加/更新のタイミング:
  - `onTrack` (`track_changed` 受信時): 対象 DJ をロースターに追加/更新する。local モードではこれが唯一の追加経路となり、結果的に n=1 のロースターになる
  - `onDjJoined` (`publisher_joined` 受信時、web モードのみ): join した DJ をロースターに追加する

設定再読み込み時 (`r` キー) はロースターをクリアしてから再構築する。

### ヘッダ表示 (DjRosterHeader)

`apps/viewer/src/App.tsx` の `DjRosterHeader` コンポーネントで表示を切り替える:

- **n <= 1**: 単一表示（ロゴ + DJ 名、ハイライトなし）。従来の見た目を維持
- **n >= 2**: DJ 名を横並びで表示。現在再生中の DJ (`track.dirName` と一致するもの) に黄色い `border-b-4` のハイライトを付与する

フォントサイズは n の数に関わらず統一する（`text-xl md:text-3xl`）。DJ 数は運用上多くても数名程度のため、サイズの動的縮小は行わない。

ヘッダの高さは `h-[100px]` で固定し、ロースターが空（初回接続直後で `track_changed` も `publisher_joined` もまだ届いていない）でも高さを確保する。これにより、待機画面から実際の表示に切り替わる際にレイアウトが変動しない。

### 待機画面の位置

ロースターが空でもヘッダの高さが確保されるため、待機画面 (`WaitingScreen`) は常に同じ位置（トラック領域の上寄り、`pt-48`）に表示される。ロースターの有無によって待機画面の位置がずれることはない。

## 影響

- `apps/viewer/src/useDataSource.ts` に `onDjJoined` コールバックを追加
- `apps/viewer/src/App.tsx` に roster state と `DjRosterHeader` コンポーネントを追加
- 単一 DJ 運用時の見た目は変更なし（ハイライトなし、従来と同じフォントサイズ）
- 複数 DJ 運用時のみ新しい横並び表示 + ハイライトが有効になる
