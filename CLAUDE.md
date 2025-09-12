# Claude Code 設定

## プロジェクト情報

**プロジェクト名:** FileMover  
**説明:** 安全なフォルダ移動・整理ツール（Windows用）  
**言語:** Rust + Tauri (GUI) / CLI  

## ディレクトリ構成

### ソース構成
```
/
├── apps/
│   ├── gui/           # Tauri フロントエンド (React/Vue)
│   │   ├── src/       # フロントエンドソースコード
│   │   ├── src-tauri/ # Tauri Rustバックエンド
│   │   └── dist/      # フロントエンドビルド出力
│   └── cli/           # Rust CLI アプリケーション
│       ├── src/       # CLIソースコード
│       └── target/    # CLIビルド出力
├── core/              # 共通Rustライブラリ
│   ├── types/         # データ型定義
│   ├── scanner/       # ディレクトリ走査
│   ├── matcher/       # パターンマッチング
│   ├── planner/       # 移動プラン生成
│   ├── executor/      # ファイル操作実行
│   ├── journal/       # 操作ログ・Undo
│   └── target/        # ライブラリビルド出力
├── docs/              # 設計ドキュメント
├── examples/          # サンプル設定・テストデータ
├── tests/             # 統合テスト
└── Cargo.toml         # Workspace設定
```

### ビルド出力先

#### CLI ビルド出力
```
apps/cli/target/
├── debug/
│   └── filemover.exe      # デバッグビルド
└── release/
    └── filemover.exe      # リリースビルド
```

#### GUI ビルド出力  
```
apps/gui/src-tauri/target/
├── debug/
│   └── filemover-gui.exe  # デバッグビルド
└── release/
    ├── filemover-gui.exe  # リリースビルド
    └── bundle/            # インストーラー等
        ├── msi/           # Windows MSIインストーラー
        └── nsis/          # NSISインストーラー
```

## ビルドコマンド

### 開発時
```bash
# CLI開発・テスト
cd apps/cli
cargo run -- scan --help

# GUI開発
cd apps/gui  
npm run tauri dev

# 全体テスト
cargo test --workspace
```

### リリースビルド
```bash
# CLI リリースビルド
cd apps/cli
cargo build --release

# GUI リリースビルド
cd apps/gui
npm run tauri build

# 全体リリースビルド（workspace root）
cargo build --release --workspace
```

## テスト・QA

### テストコマンド
```bash
# ユニットテスト
cargo test --workspace

# 統合テスト  
cargo test --test integration

# パフォーマンステスト
cargo test --test performance --release

# サンドボックステスト
cargo test --test sandbox
```

### 品質チェック
```bash
# リント
cargo clippy --workspace --all-targets

# フォーマット確認
cargo fmt --check

# セキュリティ監査
cargo audit
```

## 設定ファイル

### 開発環境設定
- `Cargo.toml` - Workspace依存関係
- `apps/gui/tauri.conf.json` - Tauri設定
- `apps/gui/package.json` - フロントエンド依存関係

### ランタイム設定
- `config.json` - アプリケーション設定
- `profiles/*.json` - 移動ルールプロファイル
- `logs/` - 実行ログディレクトリ

## 開発経過ログ

### 2025-09-11 Tauriデスクトップアプリ開発

#### 実装状況
✅ **完了項目**
- 基本プロジェクト構造の作成（Rust Workspace）
- CLIアプリケーションの実装（apps/cli/）
- 共通ライブラリの実装（core/types, matcher, scanner, planner）
- React + TypeScript GUIの実装（apps/gui/src/）
- 全5ページのUI実装（Setup, ScanResults, Plan, Execution, Results）
- TypeScriptサービス層の実装（fileScanner, planGenerator, fileExecutor, sessionManager）
- デモモード・ブラウザモード・Tauriモードの3層フォールバック実装
- Tauriプロジェクト設定（Cargo.toml, tauri.conf.json）

🔄 **現在の課題**
- **Rustコンパイル環境の問題**: Cargoの環境変数PATH設定が不完全
- **Tauriビルドの失敗**: 初回ビルド時に240/399のクレートでコンパイルが停止
- **依存関係の多さ**: 399個のクレートコンパイルでメモリ・時間リソース不足

#### 技術スタック
- **バックエンド**: Rust + Tauri 1.5
- **フロントエンド**: React 18 + TypeScript + Vite + Tailwind CSS
- **主要依存関係**: 
  - Tauri API (ファイルダイアログ、ファイルシステム操作)
  - UUID、Serde、Tokio、Anyhow

#### 現在の解決手順
1. ✅ Visual Studio Build Toolsインストール完了
2. 🔄 Windows環境変数にRust/Cargoパス追加中
3. ⏳ 新しいターミナルセッションでTauriビルド再実行予定

#### 次のステップ
1. 環境変数追加後、新しいターミナルで `cargo --version` 確認
2. `cd apps/gui && npm run tauri dev` でTauriデスクトップアプリ起動
3. デスクトップアプリの機能テスト（フォルダダイアログ、ファイル操作）
4. 実際のファイル操作機能の統合

#### ファイル構成
```
D:\ClaudeCode\project\FileMover\
├── apps/gui/
│   ├── src/                 # React UI (5ページ完成)
│   ├── src-tauri/          # Rust Tauriバックエンド
│   ├── Cargo.toml          # Tauri依存関係
│   └── package.json        # React依存関係
├── core/                   # Rust共通ライブラリ
└── docs/                   # 設計ドキュメント
```

#### 備考
- ブラウザ版は完全動作（localhost:3000）
- デモデータとUI動作確認済み
- 実際のファイル操作はTauriネイティブ機能に依存