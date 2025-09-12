use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use filemover_types::{Rule, FileMoverError};

pub struct TemplateEngine {
    variables: HashMap<String, String>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn expand_template(&mut self, rule: &Rule, source_path: &Path) -> Result<PathBuf, FileMoverError> {
        self.prepare_variables(source_path)?;
        
        let mut result = rule.template.clone();
        
        // 基本変数の置換
        for (key, value) in &self.variables {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }

        // ルール固有の変数
        if let Some(label) = &rule.label {
            result = result.replace("{label}", label);
        } else {
            result = result.replace("{label}", "");
        }

        Ok(rule.dest_root.join(result))
    }

    fn prepare_variables(&mut self, source_path: &Path) -> Result<(), FileMoverError> {
        self.variables.clear();

        // ファイル/フォルダ名
        if let Some(name) = source_path.file_name().and_then(|n| n.to_str()) {
            self.variables.insert("name".to_string(), name.to_string());
        }

        // 現在日時
        let now = Utc::now();
        self.variables.insert("yyyy".to_string(), now.format("%Y").to_string());
        self.variables.insert("yy".to_string(), now.format("%y").to_string());
        self.variables.insert("MM".to_string(), now.format("%m").to_string());
        self.variables.insert("dd".to_string(), now.format("%d").to_string());
        self.variables.insert("yyyyMM".to_string(), now.format("%Y%m").to_string());
        self.variables.insert("yyyyMMdd".to_string(), now.format("%Y%m%d").to_string());

        // ドライブレター
        self.variables.insert("drive".to_string(), self.extract_drive_letter(source_path));

        // 親フォルダ名
        self.variables.insert("parent".to_string(), self.extract_parent_name(source_path));

        // パスの深度
        let depth = source_path.components().count();
        self.variables.insert("depth".to_string(), depth.to_string());

        // ファイル拡張子（フォルダの場合は空）
        if let Some(ext) = source_path.extension().and_then(|e| e.to_str()) {
            self.variables.insert("ext".to_string(), ext.to_string());
        } else {
            self.variables.insert("ext".to_string(), String::new());
        }

        Ok(())
    }

    fn extract_drive_letter(&self, path: &Path) -> String {
        path.components()
            .next()
            .and_then(|c| {
                if let std::path::Component::Prefix(prefix) = c {
                    let prefix_str = prefix.as_os_str().to_string_lossy();
                    // "C:" から "C" を抽出
                    prefix_str.chars().next().map(|c| c.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    fn extract_parent_name(&self, path: &Path) -> String {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn validate_template(template: &str) -> Result<Vec<String>, FileMoverError> {
        let mut variables = Vec::new();
        let mut chars = template.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '{' {
                let mut var_name = String::new();
                let mut found_closing = false;
                
                while let Some(ch) = chars.next() {
                    if ch == '}' {
                        found_closing = true;
                        break;
                    } else if ch.is_alphanumeric() || ch == '_' {
                        var_name.push(ch);
                    } else {
                        return Err(FileMoverError::Config {
                            message: format!("Invalid character '{}' in template variable", ch),
                        });
                    }
                }
                
                if !found_closing {
                    return Err(FileMoverError::Config {
                        message: "Unclosed template variable".to_string(),
                    });
                }
                
                if !var_name.is_empty() {
                    variables.push(var_name);
                }
            }
        }
        
        // サポートされている変数のリスト
        let supported_vars = [
            "name", "yyyy", "yy", "MM", "dd", "yyyyMM", "yyyyMMdd",
            "drive", "parent", "depth", "ext", "label"
        ];
        
        for var in &variables {
            if !supported_vars.contains(&var.as_str()) {
                return Err(FileMoverError::Config {
                    message: format!("Unsupported template variable: {}", var),
                });
            }
        }
        
        Ok(variables)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filemover_types::{PatternSpec, ConflictPolicy};

    #[test]
    fn test_basic_template_expansion() {
        let mut engine = TemplateEngine::new();
        let rule = Rule::new(
            PatternSpec::new_glob("*"),
            PathBuf::from("D:\\Archive"),
            "{yyyy}\\{name}".to_string(),
        );

        let source_path = PathBuf::from("C:\\Source\\test_folder");
        let result = engine.expand_template(&rule, &source_path).unwrap();
        
        let year = Utc::now().format("%Y").to_string();
        let expected = PathBuf::from(format!("D:\\Archive\\{}\\test_folder", year));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_drive_extraction() {
        let engine = TemplateEngine::new();
        
        let path1 = PathBuf::from("C:\\Users\\Test");
        assert_eq!(engine.extract_drive_letter(&path1), "C");
        
        let path2 = PathBuf::from("D:\\Data\\Backup");
        assert_eq!(engine.extract_drive_letter(&path2), "D");
    }

    #[test]
    fn test_parent_extraction() {
        let engine = TemplateEngine::new();
        
        let path = PathBuf::from("C:\\Users\\TestUser\\Documents");
        assert_eq!(engine.extract_parent_name(&path), "TestUser");
    }

    #[test]
    fn test_template_validation() {
        // 有効なテンプレート
        let valid_vars = TemplateEngine::validate_template("{yyyy}\\{name}").unwrap();
        assert_eq!(valid_vars, vec!["yyyy", "name"]);
        
        // 無効なテンプレート（サポートされていない変数）
        let result = TemplateEngine::validate_template("{invalid_var}");
        assert!(result.is_err());
        
        // 無効なテンプレート（閉じ括弧なし）
        let result = TemplateEngine::validate_template("{unclosed");
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_template() {
        let mut engine = TemplateEngine::new();
        let rule = Rule::new(
            PatternSpec::new_glob("*"),
            PathBuf::from("D:\\Archive"),
            "{drive}\\{yyyy}\\{MM}\\{parent}\\{name}".to_string(),
        ).with_label("Photos".to_string());

        let source_path = PathBuf::from("E:\\Camera\\2024\\vacation_photos");
        let result = engine.expand_template(&rule, &source_path).unwrap();
        
        let now = Utc::now();
        let expected = PathBuf::from(format!(
            "D:\\Archive\\E\\{}\\{}\\2024\\vacation_photos",
            now.format("%Y"),
            now.format("%m")
        ));
        assert_eq!(result, expected);
    }
}