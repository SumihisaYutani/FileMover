# FileMover - 機能要件書

## 2. 機能要件（Functional Specs）

### 2.1 複数ルート走査

**ルート：** 複数選択可（C:, D:\Projects, E:\Archive\2024 等）

**走査：** WinAPI FindFirstFileExW / FindNextFileW（`\\?\` 長パス対応）、並列（rayon or jwalk 相当）

### 2.2 パターンマッチ

#### 種類
- **Glob**（`?` `[..]`）
- **Regex** 
- **Contains**（複数語 OR; Aho-Corasick）

#### 論理
**AND / OR / NOT**（簡易 DSL で表現可能）

#### 正規化
- NFC/NFKC
- ケースフォールド
- ダイアクリティクス除去
- 全/半角正規化（オプション）

#### 対象
**既定：** フォルダ名、**オプション：** 相対パス

### 2.3 ルール（Pattern → Destination）

画面①でルール表として定義：
```
pattern, destRoot, template, conflictPolicy, label, enabled, priority
```

**優先度：** 上から評価、最初にマッチしたルールを採用。NOT/除外は最優先。

**テンプレート：** `{name}` `{label}` `{yyyy}` `{yyyyMM}` `{drive}` `{parent}` など

### 2.4 移動プラン生成・確認

- プランにBefore/After構造、警告・衝突・跨ぎ（別ボリューム）を付与
- 2ペイン比較ツリー（遅延ロード、同期ハイライト）
- ノード毎に Skip / AutoRename / Overwrite(危険)、名前編集

### 2.5 Dry-run / 実行 / Undo

#### Dry-run
結果レポート（成功見込み/衝突残/推定時間）

#### 実行
- IFileOperation（Shell）で Move / Copy+Delete（跨ぎ）
- ACL/属性維持、進捗、キャンセル/一時停止、指数バックオフ再試行

#### Undo
Shell Undo + **独自ジャーナル（JSONL）**で from↔to を逆適用

### 2.6 安全・互換

#### 既定除外
`C:\Windows`, `C:\Program Files*`, `$Recycle.Bin`, `%TEMP%` 等（設定で編集可）

#### 既定
ジャンクション/シンボリック非追従（追従時は循環検出）

#### 警告
OneDrive オフライン/長パス/アクセス拒否はバッジ警告

### 2.7 ログ・設定

**設定：** JSON（プロファイル保存可）

**ログ：** tracing（日次ローテーション）

**エクスポート：** MovePlan（JSON）/ 一覧（CSV）