# FileMover - プロダクト要件書

## 1. プロダクト要件（Requirements）

### 1.1 目的

Windows ストレージを複数ルートから走査し、パターンマッチでヒットした「フォルダ」を抽出。

事前に定義した**ルール（パターン→移動先）**に従って、Dry-run で確認 → 実行 → Undoできる安全な移動ツールを提供。

### 1.2 ユースケース（主要）

- 同じ命名規則のフォルダを定期的に一括整理（例：*report* を D:\Archive{yyyy}{name} へ）
- 写真・動画の月別自動仕分け
- OneDrive/外付けHDD/ネットワークドライブを含む広域整理

### 1.3 スコープ

**対象：** フォルダ（必要に応じて中身ごと移動）。ファイル個別は非対象（将来拡張可）。

**対応 OS：** Windows 10/11。

**UI：** Tauri（WebView2）。CLI 同梱。

### 1.4 非スコープ

- 権限昇格の自動化（プロンプトは案内、再起動は任意）
- 複数マシン横断コピー、差分同期、重複排除

### 1.5 受け入れ基準（抜粋）

- 複数ルート合計10万フォルダを走査して UI が固まらない（仮想化・遅延ロード）
- Dry-run で Before/After 差分ツリーが表示され、AutoRename/Skip/名前編集が反映される
- 実行後に **Undo（直前操作の復元）**が機能する
- システム保護フォルダは既定で対象外。長パス・ジャンクション・OneDrive などを警告可視化

## 2. エッジケース / ガード

- 危険フォルダの強制除外と解除時の二段警告
- 循環（dest が source 配下）検出
- 長パス：常に `\\?\` で内部処理、UI 表示は通常パス
- OneDrive のオンライン要求（遅延・エラー）を警告
- 権限不足：自動リトライ→失敗で Skip 記録、結果レポートに集約

## 3. パフォーマンス方針

### 列挙
並列ディレクトリ走査＋正規化は一度だけ（キャッシュ）

### マッチ
- **Contains** → Aho-Corasick（まとめて OR）
- **Glob** → globset（Set 検索）
- **Regex** → 前置フィルタ（接頭辞/長さ）＋ RegexBuilder

### UI
仮想化リスト＆ツリー、子ノードは遅延ページング

## 4. テスト / QA

- **ユニット：** パターン一致・テンプレ展開・自動リネーム・循環/衝突検出
- **統合：** Temp にサンドボックス作成→プラン生成→Dry-run→実行→Undo
- **負荷：** 10万ノードの疑似木を生成し、相互作用のスループットを測定

## 5. 設定・ファイル形式

### 5.1 設定（例）
```json
{
  "roots": ["C:\\Users\\Youji\\Documents", "D:\\Projects"],
  "rules": [ /* Rule の配列 */ ],
  "options": {
    "caseInsensitive": true,
    "normalizeWidth": true,
    "stripDiacritics": true,
    "followJunctions": false,
    "systemProtections": true
  },
  "profiles": ["WorkDefault", "PhotoSort"]
}
```

### 5.2 ジャーナル（JSONL）
```json
{"when_utc":"2025-09-11T00:00:01Z","source":"D:\\A","dest":"E:\\B","op":"CopyDelete","result":"Ok"}
```