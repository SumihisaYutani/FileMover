# FileMover - API仕様書

## 6. コア API（Tauri/CLI 共有・概略）

### 6.1 走査＆マッチ API

```rust
/// 複数ルートを走査し、ルールに基づいてフォルダをマッチング
fn scan_and_match(
    roots: &[PathBuf], 
    rules: &[Rule], 
    options: ScanOptions
) -> impl Iterator<Item = FolderHit>;

struct ScanOptions {
    pub case_insensitive: bool,
    pub normalize_width: bool,
    pub strip_diacritics: bool,
    pub follow_junctions: bool,
    pub system_protections: bool,
    pub max_depth: Option<u32>,
    pub excluded_paths: Vec<PathBuf>,
}

struct FolderHit {
    pub path: PathBuf,
    pub name: String,
    pub matched_rule: Option<RuleId>,
    pub dest_preview: Option<PathBuf>,
    pub warnings: Vec<Warn>,
}
```

### 6.2 プラン生成 API

```rust
/// マッチしたフォルダからBefore/After木を生成し検証
fn build_move_plan(
    hits: &[FolderHit], 
    rules: &[Rule], 
    opts: PlanOptions
) -> MovePlan;

struct PlanOptions {
    pub default_conflict_policy: ConflictPolicy,
    pub preserve_acl: bool,
    pub preserve_timestamps: bool,
    pub enable_cross_volume: bool,
}
```

### 6.3 インクリメンタル検証 API

```rust
/// ノード編集時の差分検証
fn mutate_and_validate(
    plan: &mut MovePlan, 
    change: NodeChange
) -> ValidationDelta;

enum NodeChange {
    SetSkip(PlanNodeId, bool),
    SetConflictPolicy(PlanNodeId, ConflictPolicy),
    RenameNode(PlanNodeId, String),
    ExcludeNode(PlanNodeId),
}

struct ValidationDelta {
    pub affected_nodes: Vec<PlanNodeId>,
    pub new_conflicts: Vec<Conflict>,
    pub resolved_conflicts: Vec<Conflict>,
    pub summary_diff: PlanSummaryDiff,
}
```

### 6.4 Dry-run API

```rust
/// 実行シミュレーション
fn simulate(plan: &MovePlan) -> SimReport;

struct SimReport {
    pub success_estimate: u64,
    pub conflicts_remaining: u64,
    pub estimated_duration: Duration,
    pub required_permissions: Vec<RequiredPermission>,
    pub cross_volume_ops: u64,
    pub potential_issues: Vec<PotentialIssue>,
}

enum RequiredPermission {
    Administrator,
    FileSystemWrite(PathBuf),
    NetworkAccess(String),
}

enum PotentialIssue {
    LongPathSupport,
    OneDriveOffline(PathBuf),
    InsufficientSpace(PathBuf, u64),
    AccessRestriction(PathBuf),
}
```

### 6.5 実行 API

```rust
/// プラン実行（進捗コールバック付き）
fn apply(
    plan: &MovePlan, 
    progress: impl Fn(Progress)
) -> ExecResult;

struct Progress {
    pub current_node: Option<PlanNodeId>,
    pub completed_ops: u64,
    pub total_ops: u64,
    pub bytes_processed: u64,
    pub total_bytes: Option<u64>,
    pub current_speed: Option<u64>, // bytes/sec
    pub eta: Option<Duration>,
}

struct ExecResult {
    pub success_count: u64,
    pub failed_count: u64,
    pub skipped_count: u64,
    pub auto_renamed_count: u64,
    pub overwritten_count: u64,
    pub total_duration: Duration,
    pub journal_path: PathBuf,
    pub failed_operations: Vec<FailedOperation>,
}

struct FailedOperation {
    pub node_id: PlanNodeId,
    pub error: ExecutionError,
    pub retry_possible: bool,
}
```

### 6.6 Undo API

```rust
/// 直前ジャーナルを逆適用
fn undo(journal_path: &Path) -> UndoResult;

struct UndoResult {
    pub restored_count: u64,
    pub failed_count: u64,
    pub total_duration: Duration,
    pub failed_restores: Vec<FailedRestore>,
}

struct FailedRestore {
    pub original_source: PathBuf,
    pub original_dest: PathBuf,
    pub error: UndoError,
}
```

## 7. CLI サブコマンド

```bash
# フォルダスキャン
filemover scan --roots "C:\Users" "D:\Projects" --config config.json

# プラン生成
filemover plan --input scan_results.json --rules rules.json --output plan.json

# Dry-run実行  
filemover dry-run --plan plan.json

# 実際の移動実行
filemover apply --plan plan.json --journal journal.jsonl

# Undo実行
filemover undo --journal journal.jsonl
```

## 8. Tauri コマンド

```typescript
// フロントエンド向けTauriコマンド
interface TauriCommands {
  scan_folders(roots: string[], options: ScanOptions): Promise<FolderHit[]>;
  create_plan(hits: FolderHit[], rules: Rule[]): Promise<MovePlan>;
  simulate_plan(plan: MovePlan): Promise<SimReport>;
  execute_plan(plan: MovePlan): Promise<ExecResult>;
  undo_operation(journalPath: string): Promise<UndoResult>;
  
  // プロファイル管理
  save_profile(name: string, config: Config): Promise<void>;
  load_profile(name: string): Promise<Config>;
  list_profiles(): Promise<string[]>;
}
```