use std::path::{Path, PathBuf};
use tracing::{debug, warn};
use filemover_types::{ScanOptions, FileMoverError};
use crate::scanner::DirectoryEntry;

#[cfg(windows)]
use crate::windows_scanner::WindowsDirectoryWalker;

pub struct DirectoryWalker {
    options: ScanOptions,
}

impl DirectoryWalker {
    pub fn new(options: ScanOptions) -> Self {
        Self { options }
    }

    pub fn walk(&self, root: &Path) -> Result<Vec<DirectoryEntry>, FileMoverError> {
        debug!("Walking directory: {}", root.display());

        // システム保護フォルダのチェック
        if self.options.system_protections && self.is_protected_path(root) {
            warn!("Skipping protected path: {}", root.display());
            return Ok(vec![]);
        }

        // プラットフォーム固有のウォーカーを使用
        #[cfg(windows)]
        {
            let walker = WindowsDirectoryWalker::new(self.options.clone());
            walker.walk(root)
        }

        #[cfg(not(windows))]
        {
            self.walk_standard(root)
        }
    }

    #[cfg(not(windows))]
    fn walk_standard(&self, root: &Path) -> Result<Vec<DirectoryEntry>, FileMoverError> {
        use walkdir::WalkDir;

        let mut entries = Vec::new();
        let walker = WalkDir::new(root)
            .follow_links(!self.options.follow_junctions)
            .max_depth(self.options.max_depth.map(|d| d as usize).unwrap_or(usize::MAX));

        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_dir() {
                        let path = entry.path().to_path_buf();
                        
                        // 除外パスのチェック
                        if self.is_excluded_path(&path) {
                            continue;
                        }

                        let dir_entry = DirectoryEntry {
                            path,
                            is_directory: true,
                            is_junction: entry.file_type().is_symlink(),
                            access_denied: false,
                            size_bytes: None, // Unix系では一般的にディレクトリサイズは計算しない
                        };

                        entries.push(dir_entry);
                    }
                }
                Err(e) => {
                    warn!("Failed to access path: {}", e);
                    continue;
                }
            }
        }

        Ok(entries)
    }

    fn is_protected_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_uppercase();
        
        // Windows システムパス
        if path_str.starts_with("C:\\WINDOWS") ||
           path_str.starts_with("C:\\PROGRAM FILES") ||
           path_str.contains("$RECYCLE.BIN") ||
           path_str.contains("SYSTEM VOLUME INFORMATION") {
            return true;
        }

        // 設定で指定された除外パス
        for excluded in &self.options.excluded_paths {
            if path.starts_with(excluded) {
                return true;
            }
        }

        false
    }

    fn is_excluded_path(&self, path: &Path) -> bool {
        for excluded in &self.options.excluded_paths {
            if path.starts_with(excluded) {
                return true;
            }
        }
        false
    }
}

pub trait DirectoryWalkerTrait {
    fn walk(&self, root: &Path) -> Result<Vec<DirectoryEntry>, FileMoverError>;
}

impl DirectoryWalkerTrait for DirectoryWalker {
    fn walk(&self, root: &Path) -> Result<Vec<DirectoryEntry>, FileMoverError> {
        self.walk(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_protected_path_detection() {
        let walker = DirectoryWalker::new(ScanOptions::default());
        
        assert!(walker.is_protected_path(Path::new("C:\\Windows\\System32")));
        assert!(walker.is_protected_path(Path::new("C:\\Program Files\\Test")));
        assert!(!walker.is_protected_path(Path::new("C:\\Users\\Test")));
    }

    #[test]
    fn test_excluded_path_detection() {
        let mut options = ScanOptions::default();
        options.excluded_paths.push(PathBuf::from("C:\\Temp"));
        
        let walker = DirectoryWalker::new(options);
        
        assert!(walker.is_excluded_path(Path::new("C:\\Temp\\subfolder")));
        assert!(!walker.is_excluded_path(Path::new("C:\\Users\\Test")));
    }

    #[test]
    fn test_directory_walking() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // テスト用ディレクトリ構造を作成
        std::fs::create_dir_all(root.join("folder1")).unwrap();
        std::fs::create_dir_all(root.join("folder2/subfolder")).unwrap();
        
        let walker = DirectoryWalker::new(ScanOptions::default());
        let entries = walker.walk(root).unwrap();
        
        // ルート + folder1 + folder2 + subfolder = 4個のディレクトリ
        assert!(entries.len() >= 3); // 最低でも作成した3つのサブディレクトリ
        
        let paths: Vec<_> = entries.iter().map(|e| &e.path).collect();
        assert!(paths.iter().any(|p| p.ends_with("folder1")));
        assert!(paths.iter().any(|p| p.ends_with("folder2")));
    }
}