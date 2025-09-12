use std::collections::HashMap;
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::{Regex, RegexBuilder};
use aho_corasick::AhoCorasick;
use filemover_types::{PatternSpec, PatternKind, FileMoverError};
use crate::normalizer::TextNormalizer;

pub trait PatternMatcher: Send + Sync {
    fn is_match(&self, text: &str) -> Result<bool, FileMoverError>;
}

pub struct GlobMatcher {
    glob_set: GlobSet,
    normalizer: TextNormalizer,
}

impl GlobMatcher {
    pub fn new(patterns: &[PatternSpec], normalizer: TextNormalizer) -> Result<Self, FileMoverError> {
        let mut builder = GlobSetBuilder::new();
        
        for pattern in patterns {
            if let PatternKind::Glob = pattern.kind {
                let glob = Glob::new(&pattern.value).map_err(|e| FileMoverError::Pattern {
                    message: format!("Invalid glob pattern '{}': {}", pattern.value, e),
                })?;
                builder.add(glob);
            }
        }

        let glob_set = builder.build()
            .map_err(|e| FileMoverError::Pattern {
                message: format!("Failed to build glob set: {}", e),
            })?;

        Ok(Self { glob_set, normalizer })
    }
}

impl PatternMatcher for GlobMatcher {
    fn is_match(&self, text: &str) -> Result<bool, FileMoverError> {
        let normalized = self.normalizer.normalize(text)?;
        Ok(self.glob_set.is_match(&normalized))
    }
}

pub struct RegexMatcher {
    regexes: Vec<Regex>,
    normalizer: TextNormalizer,
}

impl RegexMatcher {
    pub fn new(patterns: &[PatternSpec], normalizer: TextNormalizer) -> Result<Self, FileMoverError> {
        let mut regexes = Vec::new();
        
        for pattern in patterns {
            if let PatternKind::Regex = pattern.kind {
                let regex = RegexBuilder::new(&pattern.value)
                    .case_insensitive(pattern.case_insensitive)
                    .build()
                    .map_err(|e| FileMoverError::Pattern {
                        message: format!("Invalid regex pattern '{}': {}", pattern.value, e),
                    })?;
                regexes.push(regex);
            }
        }

        Ok(Self { regexes, normalizer })
    }
}

impl PatternMatcher for RegexMatcher {
    fn is_match(&self, text: &str) -> Result<bool, FileMoverError> {
        let normalized = self.normalizer.normalize(text)?;
        Ok(self.regexes.iter().any(|regex| regex.is_match(&normalized)))
    }
}

pub struct ContainsMatcher {
    aho_corasick: AhoCorasick,
    normalizer: TextNormalizer,
}

impl ContainsMatcher {
    pub fn new(patterns: &[PatternSpec], normalizer: TextNormalizer) -> Result<Self, FileMoverError> {
        let mut keywords = Vec::new();
        
        for pattern in patterns {
            if let PatternKind::Contains = pattern.kind {
                let keyword = if pattern.case_insensitive {
                    normalizer.normalize(&pattern.value)?
                } else {
                    pattern.value.clone()
                };
                keywords.push(keyword);
            }
        }

        let aho_corasick = AhoCorasick::new(&keywords)
            .map_err(|e| FileMoverError::Pattern {
                message: format!("Failed to build Aho-Corasick automaton: {}", e),
            })?;

        Ok(Self { aho_corasick, normalizer })
    }
}

impl PatternMatcher for ContainsMatcher {
    fn is_match(&self, text: &str) -> Result<bool, FileMoverError> {
        let normalized = self.normalizer.normalize(text)?;
        Ok(self.aho_corasick.is_match(&normalized))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filemover_types::NormalizationOptions;

    fn create_test_normalizer() -> TextNormalizer {
        TextNormalizer::new(NormalizationOptions::default())
    }

    #[test]
    fn test_glob_matcher() {
        let patterns = vec![
            PatternSpec::new_glob("report*"),
            PatternSpec::new_glob("*2024*"),
        ];
        let normalizer = create_test_normalizer();
        let matcher = GlobMatcher::new(&patterns, normalizer).unwrap();
        
        assert!(matcher.is_match("report_january").unwrap());
        assert!(matcher.is_match("backup_2024_03").unwrap());
        assert!(!matcher.is_match("document").unwrap());
    }

    #[test]
    fn test_regex_matcher() {
        let patterns = vec![
            PatternSpec::new_regex(r"report_\d{4}"),
        ];
        let normalizer = create_test_normalizer();
        let matcher = RegexMatcher::new(&patterns, normalizer).unwrap();
        
        assert!(matcher.is_match("report_2024").unwrap());
        assert!(!matcher.is_match("report_abc").unwrap());
    }

    #[test]
    fn test_contains_matcher() {
        let patterns = vec![
            PatternSpec::new_contains("photo"),
            PatternSpec::new_contains("image"),
        ];
        let normalizer = create_test_normalizer();
        let matcher = ContainsMatcher::new(&patterns, normalizer).unwrap();
        
        assert!(matcher.is_match("my_photo_album").unwrap());
        assert!(matcher.is_match("image_backup").unwrap());
        assert!(!matcher.is_match("document_folder").unwrap());
    }
}