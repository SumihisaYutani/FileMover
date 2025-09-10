# FileMover - アーキテクチャ設計書

## 4. アーキテクチャ / モジュール

### プロジェクト構造
```
/apps
  /gui       # Tauri フロント（React/Vue 任意）
  /cli       # Rust CLI
/core        # Rust ライブラリ（共通ロジック）
  scanner/   # 走査（WinAPI + 並列）
  matcher/   # 正規化 + Glob/Regex/Contains/DSL
  planner/   # Before/After 木生成 + 衝突検出 + 自動リネーム
  executor/  # IFileOperation 呼出 + 進捗/リトライ/Undo
  journal/   # JSONL 出力・逆適用
  types/     # モデル定義
```

### 画面設計（UI/Flow）

#### 3.1 ① 起動画面（ルート＆ルール）

**ルートリスト：** 追加/削除/履歴/並行スキャン

**ルール表：**
- **列：** 有効、パターン、移動先ルート、テンプレ、衝突ポリシー、ラベル、優先度
- **行操作：** 追加/複製/削除、プレビュー（サンプル名→パス例）

[検索開始] → ②へ

#### 3.2 ② 検索結果

**表：** 名前、ルート、フルパス、採用ルール、移動先プレビュー、バッジ（衝突/長/権限…）

**操作：** 選択除外、CSV出力、（右クリ）この選択でプラン作成

[移動プラン作成] → ③へ

#### 3.3 ③ 移動プラン確認（最重要）

**上部：** サマリ（件数/容量/跨ぎ/衝突）、ポリシー既定、テンプレ表示

**2ペイン：** Before / After（差分ハイライト、同期スクロール）

**ノード詳細：** オペ種別、警告、衝突解決ドロップダウン、名前編集→差分再検証

[Dry-run] / [プラン保存] / [実行へ] → ③' 最終確認

#### 3.4 ③' 実行前 最終確認

件数・容量、危険操作数、ログ/ジャーナル保存先、必要権限

[実行]

#### 3.5 ④ 実行

全体進捗、現在ノード、速度、ログタブ

[一時停止] [キャンセル] [ログを開く]

#### 3.6 ⑤ 結果

成功/失敗/Skip、AutoRename/Overwrite 件数、所要時間

失敗のみ再実行、Undo（直前操作）、ログ/ジャーナルリンク

## 5. データモデル（抜粋・Rust）

### パターン
```rust
enum PatternKind { Glob, Regex, Contains }

struct PatternSpec {
    kind: PatternKind,
    value: String,
    is_exclude: bool,
}
```

### ルール
```rust
struct Rule {
    id: String,
    enabled: bool,
    pattern: PatternSpec,
    dest_root: PathBuf,
    template: String,                // e.g. "{yyyy}\\{name}"
    policy: ConflictPolicy,          // AutoRename | Skip | Overwrite
    label: Option<String>,
    priority: u32,
}

enum ConflictPolicy { AutoRename, Skip, Overwrite }
enum OpKind { Move, CopyDelete, Rename, Skip, None }
```

### プランノード
```rust
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
struct PlanNodeId(u64);

struct PlanNode {
    id: PlanNodeId,
    is_dir: bool,
    name_before: String,
    path_before: PathBuf,
    name_after: String,
    path_after: PathBuf,
    kind: OpKind,
    size_bytes: Option<u64>,
    warnings: Vec<Warn>,
    conflicts: Vec<Conflict>,
    children: Vec<PlanNodeId>,       // 遅延ロードのためID参照
}

enum Warn { LongPath, AclDiffers, Offline, AccessDenied, Junction }
enum Conflict { NameExists, CycleDetected, DestInsideSource, NoSpace, Permission }
```

### 移動プラン
```rust
struct MovePlan {
    roots: Vec<PlanNodeId>,
    nodes: HashMap<PlanNodeId, PlanNode>,
    summary: PlanSummary,
}

struct PlanSummary {
    count_dirs: u64,
    count_files: u64,
    total_bytes: Option<u64>,
    cross_volume: u64,
    conflicts: u64,
    warnings: u64,
}
```

### ジャーナル
```rust
// ジャーナル（JSONL の1行分）
struct JournalEntry {
    when_utc: String,
    source: PathBuf,
    dest: PathBuf,
    op: OpKind,
    result: ResultKind,              // Ok/Skip/Failed
    message: Option<String>,
}
```