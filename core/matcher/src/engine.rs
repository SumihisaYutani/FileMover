use std::collections::HashMap;
use filemover_types::{Rule, PatternSpec, PatternKind, FileMoverError, NormalizationOptions};
use crate::normalizer::TextNormalizer;
use crate::matcher::{PatternMatcher, GlobMatcher, RegexMatcher, ContainsMatcher};

pub struct MatchingEngine {
    rules: Vec<Rule>,
    matchers: HashMap<PatternKind, Box<dyn PatternMatcher>>,
    normalizer: TextNormalizer,
}

impl MatchingEngine {
    pub fn new(rules: Vec<Rule>, normalization: NormalizationOptions) -> Result<Self, FileMoverError> {
        let normalizer = TextNormalizer::new(normalization.clone());
        let mut matchers: HashMap<PatternKind, Box<dyn PatternMatcher>> = HashMap::new();

        // パターン種類別にグループ化
        let glob_patterns: Vec<_> = rules.iter()
            .map(|r| &r.pattern)
            .filter(|p| matches!(p.kind, PatternKind::Glob))
            .collect();
        
        let regex_patterns: Vec<_> = rules.iter()
            .map(|r| &r.pattern)
            .filter(|p| matches!(p.kind, PatternKind::Regex))
            .collect();
        
        let contains_patterns: Vec<_> = rules.iter()
            .map(|r| &r.pattern)
            .filter(|p| matches!(p.kind, PatternKind::Contains))
            .collect();

        // 各種マッチャーを作成
        if !glob_patterns.is_empty() {
            let glob_specs: Vec<PatternSpec> = glob_patterns.into_iter().cloned().collect();
            let matcher = GlobMatcher::new(&glob_specs, TextNormalizer::new(normalization.clone()))?;
            matchers.insert(PatternKind::Glob, Box::new(matcher));
        }

        if !regex_patterns.is_empty() {
            let regex_specs: Vec<PatternSpec> = regex_patterns.into_iter().cloned().collect();
            let matcher = RegexMatcher::new(&regex_specs, TextNormalizer::new(normalization.clone()))?;
            matchers.insert(PatternKind::Regex, Box::new(matcher));
        }

        if !contains_patterns.is_empty() {
            let contains_specs: Vec<PatternSpec> = contains_patterns.into_iter().cloned().collect();
            let matcher = ContainsMatcher::new(&contains_specs, TextNormalizer::new(normalization.clone()))?;
            matchers.insert(PatternKind::Contains, Box::new(matcher));
        }

        Ok(Self {
            rules,
            matchers,
            normalizer,
        })
    }

    pub fn find_matching_rule(&self, folder_name: &str) -> Result<Option<&Rule>, FileMoverError> {
        // 除外ルールを最初にチェック
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if rule.pattern.is_exclude {
                if self.is_pattern_match(&rule.pattern, folder_name)? {
                    return Ok(None); // 除外対象
                }
            }
        }

        // 優先度順でマッチングルールを検索
        let mut sorted_rules: Vec<&Rule> = self.rules.iter()
            .filter(|r| r.enabled && !r.pattern.is_exclude)
            .collect();
        sorted_rules.sort_by_key(|r| r.priority);

        for rule in sorted_rules {
            if self.is_pattern_match(&rule.pattern, folder_name)? {
                return Ok(Some(rule));
            }
        }

        Ok(None)
    }

    fn is_pattern_match(&self, pattern: &PatternSpec, text: &str) -> Result<bool, FileMoverError> {
        match self.matchers.get(&pattern.kind) {
            Some(matcher) => matcher.is_match(text),
            None => {
                // フォールバック: 個別マッチング
                self.match_individual_pattern(pattern, text)
            }
        }
    }

    fn match_individual_pattern(&self, pattern: &PatternSpec, text: &str) -> Result<bool, FileMoverError> {
        let normalized = self.normalizer.normalize(text)?;
        let pattern_normalized = self.normalizer.normalize(&pattern.value)?;

        match pattern.kind {
            PatternKind::Glob => {
                use globset::Glob;
                let glob = Glob::new(&pattern_normalized)
                    .map_err(|e| FileMoverError::Pattern {
                        message: format!("Invalid glob pattern: {}", e),
                    })?;
                Ok(glob.compile_matcher().is_match(&normalized))
            }
            PatternKind::Regex => {
                use regex::RegexBuilder;
                let regex = RegexBuilder::new(&pattern_normalized)
                    .case_insensitive(pattern.case_insensitive)
                    .build()
                    .map_err(|e| FileMoverError::Pattern {
                        message: format!("Invalid regex pattern: {}", e),
                    })?;
                Ok(regex.is_match(&normalized))
            }
            PatternKind::Contains => {
                let haystack = if pattern.case_insensitive {
                    normalized
                } else {
                    text.to_string()
                };
                let needle = if pattern.case_insensitive {
                    pattern_normalized
                } else {
                    pattern.value.clone()
                };
                Ok(haystack.contains(&needle))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filemover_types::{ConflictPolicy, PatternSpec};
    use std::path::PathBuf;

    #[test]
    fn test_rule_matching_priority() {
        let rules = vec![
            Rule::new(
                PatternSpec::new_glob("*report*"),
                PathBuf::from("D:\\Archive\\Reports"),
                "{yyyy}\\{name}".to_string(),
            ).with_priority(10),
            Rule::new(
                PatternSpec::new_glob("*"),
                PathBuf::from("D:\\Archive\\General"),
                "{name}".to_string(),
            ).with_priority(100),
        ];

        let engine = MatchingEngine::new(rules, NormalizationOptions::default()).unwrap();
        
        let matched_rule = engine.find_matching_rule("monthly_report").unwrap();
        assert!(matched_rule.is_some());
        assert_eq!(matched_rule.unwrap().priority, 10);
    }

    #[test]
    fn test_exclude_patterns() {
        let rules = vec![
            Rule::new(
                PatternSpec::new_glob("temp*").exclude(),
                PathBuf::from(""),
                "".to_string(),
            ),
            Rule::new(
                PatternSpec::new_glob("*"),
                PathBuf::from("D:\\Archive"),
                "{name}".to_string(),
            ),
        ];

        let engine = MatchingEngine::new(rules, NormalizationOptions::default()).unwrap();
        
        // temp* は除外される
        assert!(engine.find_matching_rule("temp_folder").unwrap().is_none());
        // その他はマッチする
        assert!(engine.find_matching_rule("normal_folder").unwrap().is_some());
    }
}