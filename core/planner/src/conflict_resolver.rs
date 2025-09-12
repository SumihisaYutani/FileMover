use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use filemover_types::{Conflict, ConflictPolicy, FileMoverError, Warning, Permission};
use tracing::{debug, warn};

pub struct ConflictResolver {
    existing_paths: HashSet<PathBuf>,
    auto_rename_counters: HashMap<PathBuf, u32>,
}

impl ConflictResolver {
    pub fn new() -> Self {
        Self {
            existing_paths: HashSet::new(),
            auto_rename_counters: HashMap::new(),
        }
    }

    pub fn resolve_conflicts(
        &mut self,
        dest_path: &Path,
        policy: ConflictPolicy,
    ) -> Result<(PathBuf, Vec<Conflict>), FileMoverError> {
        let mut conflicts = Vec::new();
        let mut resolved_path = dest_path.to_path_buf();

        // 既存パス衝突のチェック
        if self.path_exists(&resolved_path) || self.existing_paths.contains(&resolved_path) {
            conflicts.push(Conflict::NameExists {
                existing_path: resolved_path.clone(),
            });

            match policy {
                ConflictPolicy::AutoRename => {
                    resolved_path = self.generate_auto_renamed_path(&resolved_path)?;
                    // 衝突は解決されたので除去
                    conflicts.retain(|c| !matches!(c, Conflict::NameExists { .. }));
                }
                ConflictPolicy::Skip => {
                    // Skipの場合は元のパスを返すが、衝突は残る
                }
                ConflictPolicy::Overwrite => {
                    // Overwriteの場合は衝突情報を残すが、パスはそのまま
                }
            }
        }

        // 循環参照のチェック
        if self.is_cyclic_move(dest_path, &resolved_path) {
            conflicts.push(Conflict::CycleDetected);
        }

        // 移動先が移動元の子である場合のチェック
        if self.is_dest_inside_source(dest_path, &resolved_path) {
            conflicts.push(Conflict::DestInsideSource);
        }

        // ディスク容量のチェック
        if let Some(space_conflict) = self.check_disk_space(&resolved_path)? {
            conflicts.push(space_conflict);
        }

        // 権限のチェック
        if let Some(permission_conflict) = self.check_permissions(&resolved_path)? {
            conflicts.push(permission_conflict);
        }

        // 解決済みパスを記録
        self.existing_paths.insert(resolved_path.clone());

        Ok((resolved_path, conflicts))
    }

    fn generate_auto_renamed_path(&mut self, original: &Path) -> Result<PathBuf, FileMoverError> {
        let parent = original.parent().unwrap_or(Path::new(""));
        let stem = original.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| FileMoverError::PlanValidation {
                message: "Invalid filename for auto-rename".to_string(),
            })?;
        
        let extension = original.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mut counter_value = *self.auto_rename_counters.entry(original.to_path_buf()).or_insert(0);
        
        loop {
            counter_value += 1;
            
            let new_name = if extension.is_empty() {
                format!("{}_{}", stem, counter_value)
            } else {
                format!("{}_{}.{}", stem, counter_value, extension)
            };
            
            let new_path = parent.join(new_name);
            
            if !self.path_exists(&new_path) && !self.existing_paths.contains(&new_path) {
                debug!("Auto-renamed {} to {}", original.display(), new_path.display());
                self.auto_rename_counters.insert(original.to_path_buf(), counter_value);
                return Ok(new_path);
            }
            
            // 無限ループ防止
            if counter_value > 9999 {
                return Err(FileMoverError::PlanValidation {
                    message: format!("Could not generate unique name after 9999 attempts for {}", original.display()),
                });
            }
        }
    }

    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_cyclic_move(&self, _source: &Path, _dest: &Path) -> bool {
        // 循環移動の検出ロジック
        // 移動先が移動元と同じ場合
        // より複雑なケースでは、移動元の祖先が移動先に含まれる場合など
        false // 簡単な実装のため、常にfalseを返す
    }

    fn is_dest_inside_source(&self, source: &Path, dest: &Path) -> bool {
        // 移動先が移動元の子ディレクトリかチェック
        dest.starts_with(source)
    }

    fn check_disk_space(&self, _path: &Path) -> Result<Option<Conflict>, FileMoverError> {
        // ディスク容量チェック（Windows固有の実装が必要）
        #[cfg(windows)]
        {
            // Stub implementation - in real code would use GetDiskFreeSpaceExW
            let free_bytes: u64 = 1024 * 1024 * 1024; // 1GB placeholder
            
            {
                // 仮の必要容量（実際には移動するフォルダのサイズを計算する必要がある）
                let required_bytes = 1024 * 1024; // 1MB as placeholder
                
                if free_bytes < required_bytes {
                    return Ok(Some(Conflict::NoSpace {
                        required: required_bytes,
                        available: free_bytes,
                    }));
                }
            }
        }
        
        Ok(None)
    }

    fn check_permissions(&self, path: &Path) -> Result<Option<Conflict>, FileMoverError> {
        // 権限チェック（簡易実装）
        if let Some(parent) = path.parent() {
            // 親ディレクトリが存在し、書き込み可能かチェック
            if parent.exists() {
                // テスト用の一時ファイル作成を試みる
                let test_file = parent.join(".filemover_permission_test");
                match std::fs::File::create(&test_file) {
                    Ok(_) => {
                        // テストファイルを削除
                        let _ = std::fs::remove_file(&test_file);
                    }
                    Err(_) => {
                        return Ok(Some(Conflict::Permission {
                            required: Permission::FileSystemWrite,
                        }));
                    }
                }
            }
        }
        
        Ok(None)
    }

    #[cfg(windows)]
    fn get_volume_root(&self, path: &Path) -> PathBuf {
        // パスからボリュームルートを抽出
        if let Some(prefix) = path.components().next() {
            if let std::path::Component::Prefix(prefix_component) = prefix {
                return PathBuf::from(format!("{}\\", prefix_component.as_os_str().to_string_lossy()));
            }
        }
        PathBuf::from("C:\\") // デフォルト
    }

    pub fn reset(&mut self) {
        self.existing_paths.clear();
        self.auto_rename_counters.clear();
    }

    pub fn add_existing_path(&mut self, path: PathBuf) {
        self.existing_paths.insert(path);
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_auto_rename() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_folder");
        
        // 既存ファイルを作成
        std::fs::create_dir(&test_path).unwrap();
        
        let mut resolver = ConflictResolver::new();
        let (renamed_path, conflicts) = resolver
            .resolve_conflicts(&test_path, ConflictPolicy::AutoRename)
            .unwrap();
        
        assert_ne!(renamed_path, test_path);
        assert!(renamed_path.to_string_lossy().contains("_1"));
        assert!(conflicts.is_empty()); // 衝突は解決されているはず
    }

    #[test]
    fn test_dest_inside_source_detection() {
        let resolver = ConflictResolver::new();
        
        let source = Path::new("C:\\Source\\Folder");
        let dest_inside = Path::new("C:\\Source\\Folder\\Subfolder");
        let dest_outside = Path::new("C:\\Destination\\Folder");
        
        assert!(resolver.is_dest_inside_source(source, dest_inside));
        assert!(!resolver.is_dest_inside_source(source, dest_outside));
    }

    #[test]
    fn test_skip_policy() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_folder");
        
        // 既存ディレクトリを作成
        std::fs::create_dir(&test_path).unwrap();
        
        let mut resolver = ConflictResolver::new();
        let (resolved_path, conflicts) = resolver
            .resolve_conflicts(&test_path, ConflictPolicy::Skip)
            .unwrap();
        
        assert_eq!(resolved_path, test_path); // パスは変更されない
        assert!(!conflicts.is_empty()); // 衝突は残る
        assert!(matches!(conflicts[0], Conflict::NameExists { .. }));
    }

    #[test]
    fn test_overwrite_policy() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_folder");
        
        // 既存ディレクトリを作成
        std::fs::create_dir(&test_path).unwrap();
        
        let mut resolver = ConflictResolver::new();
        let (resolved_path, conflicts) = resolver
            .resolve_conflicts(&test_path, ConflictPolicy::Overwrite)
            .unwrap();
        
        assert_eq!(resolved_path, test_path); // パスは変更されない
        assert!(!conflicts.is_empty()); // 衝突情報は残る
        assert!(matches!(conflicts[0], Conflict::NameExists { .. }));
    }
}