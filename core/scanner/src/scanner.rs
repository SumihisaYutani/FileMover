use std::path::{Path, PathBuf};
use std::sync::Arc;
use rayon::prelude::*;
use tracing::{debug, warn, error};
use filemover_types::{
    ScanOptions, Rule, FolderHit, Warning, FileMoverError, NormalizationOptions
};
use filemover_matcher::MatchingEngine;
use crate::walker::DirectoryWalker;

pub struct FolderScanner {
    matching_engine: Arc<MatchingEngine>,
    options: ScanOptions,
}

impl FolderScanner {
    pub fn new(rules: Vec<Rule>, options: ScanOptions) -> Result<Self, FileMoverError> {
        let matching_engine = Arc::new(
            MatchingEngine::new(rules, options.normalization.clone())?
        );

        Ok(Self {
            matching_engine,
            options,
        })
    }

    pub fn scan_roots(&self, roots: &[PathBuf]) -> Result<Vec<FolderHit>, FileMoverError> {
        debug!("Starting scan of {} root directories", roots.len());
        
        let results: Result<Vec<Vec<FolderHit>>, FileMoverError> = roots
            .par_iter()
            .map(|root| self.scan_single_root(root))
            .collect();

        let all_hits: Vec<FolderHit> = results?.into_iter().flatten().collect();
        debug!("Scan completed, found {} folder hits", all_hits.len());
        
        Ok(all_hits)
    }

    fn scan_single_root(&self, root: &Path) -> Result<Vec<FolderHit>, FileMoverError> {
        if !root.exists() {
            warn!("Root path does not exist: {}", root.display());
            return Ok(vec![]);
        }

        if !root.is_dir() {
            warn!("Root path is not a directory: {}", root.display());
            return Ok(vec![]);
        }

        debug!("Scanning root: {}", root.display());
        
        let walker = DirectoryWalker::new(self.options.clone());
        let entries = walker.walk(root)?;
        
        let hits: Result<Vec<FolderHit>, FileMoverError> = entries
            .into_par_iter()
            .filter_map(|entry| {
                match self.process_entry(entry) {
                    Ok(Some(hit)) => Some(Ok(hit)),
                    Ok(None) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .collect();

        hits
    }

    fn process_entry(&self, entry: DirectoryEntry) -> Result<Option<FolderHit>, FileMoverError> {
        if !entry.is_directory {
            return Ok(None);
        }

        let folder_name = entry.path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| FileMoverError::Scan {
                path: entry.path.clone(),
                message: "Invalid folder name encoding".to_string(),
            })?;

        // マッチングルールを確認
        match self.matching_engine.find_matching_rule(folder_name)? {
            Some(rule) => {
                let dest_preview = self.generate_destination_preview(rule, &entry.path)?;
                let warnings = self.analyze_warnings(&entry);

                let hit = FolderHit {
                    path: entry.path.clone(),
                    name: folder_name.to_string(),
                    matched_rule: Some(rule.id),
                    dest_preview: Some(dest_preview),
                    warnings,
                    size_bytes: entry.size_bytes,
                };

                Ok(Some(hit))
            }
            None => Ok(None),
        }
    }

    fn generate_destination_preview(&self, rule: &Rule, source_path: &Path) -> Result<PathBuf, FileMoverError> {
        let template = &rule.template;
        let folder_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // テンプレート変数を展開
        let expanded = template
            .replace("{name}", folder_name)
            .replace("{yyyy}", &chrono::Utc::now().format("%Y").to_string())
            .replace("{yyyyMM}", &chrono::Utc::now().format("%Y%m").to_string())
            .replace("{drive}", &self.extract_drive_letter(source_path))
            .replace("{parent}", &self.extract_parent_name(source_path))
            .replace("{label}", &rule.label.as_deref().unwrap_or(""));

        Ok(rule.dest_root.join(expanded))
    }

    fn extract_drive_letter(&self, path: &Path) -> String {
        path.components()
            .next()
            .and_then(|c| {
                if let std::path::Component::Prefix(prefix) = c {
                    prefix.as_os_str().to_str()
                } else {
                    None
                }
            })
            .unwrap_or("")
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_default()
    }

    fn extract_parent_name(&self, path: &Path) -> String {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    fn analyze_warnings(&self, entry: &DirectoryEntry) -> Vec<Warning> {
        let mut warnings = Vec::new();

        // 長パス警告
        if entry.path.to_string_lossy().len() > 260 {
            warnings.push(Warning::LongPath);
        }

        // ジャンクション警告
        if entry.is_junction {
            warnings.push(Warning::Junction);
        }

        // アクセス拒否警告
        if entry.access_denied {
            warnings.push(Warning::AccessDenied);
        }

        // OneDriveオフライン警告（Windows特有）
        #[cfg(windows)]
        if self.is_onedrive_offline(&entry.path) {
            warnings.push(Warning::Offline);
        }

        warnings
    }

    #[cfg(windows)]
    fn is_onedrive_offline(&self, path: &Path) -> bool {
        // OneDriveパスパターンの簡易チェック
        let path_str = path.to_string_lossy().to_lowercase();
        path_str.contains("onedrive") && 
        (path_str.contains("personal") || path_str.contains("business"))
    }
}

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub path: PathBuf,
    pub is_directory: bool,
    pub is_junction: bool,
    pub access_denied: bool,
    pub size_bytes: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use filemover_types::{PatternSpec, ConflictPolicy};
    use tempfile::TempDir;

    fn create_test_scanner() -> FolderScanner {
        let rules = vec![
            Rule::new(
                PatternSpec::new_glob("test*"),
                PathBuf::from("D:\\Archive"),
                "{name}".to_string(),
            ),
        ];
        let options = ScanOptions::default();
        FolderScanner::new(rules, options).unwrap()
    }

    #[test]
    fn test_template_expansion() {
        let scanner = create_test_scanner();
        let rule = Rule::new(
            PatternSpec::new_glob("*"),
            PathBuf::from("D:\\Archive"),
            "{yyyy}\\{name}".to_string(),
        );

        let source_path = PathBuf::from("C:\\Source\\test_folder");
        let result = scanner.generate_destination_preview(&rule, &source_path).unwrap();
        
        let year = chrono::Utc::now().format("%Y").to_string();
        let expected = PathBuf::from(format!("D:\\Archive\\{}\\test_folder", year));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_drive_extraction() {
        let scanner = create_test_scanner();
        let path = PathBuf::from("C:\\Users\\Test");
        let drive = scanner.extract_drive_letter(&path);
        assert_eq!(drive, "C");
    }
}