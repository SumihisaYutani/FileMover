use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternKind {
    Glob,
    Regex,
    Contains,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternSpec {
    pub kind: PatternKind,
    pub value: String,
    pub is_exclude: bool,
    pub case_insensitive: bool,
}

impl PatternSpec {
    pub fn new_glob(pattern: &str) -> Self {
        Self {
            kind: PatternKind::Glob,
            value: pattern.to_string(),
            is_exclude: false,
            case_insensitive: true,
        }
    }

    pub fn new_regex(pattern: &str) -> Self {
        Self {
            kind: PatternKind::Regex,
            value: pattern.to_string(),
            is_exclude: false,
            case_insensitive: true,
        }
    }

    pub fn new_contains(pattern: &str) -> Self {
        Self {
            kind: PatternKind::Contains,
            value: pattern.to_string(),
            is_exclude: false,
            case_insensitive: true,
        }
    }

    pub fn exclude(mut self) -> Self {
        self.is_exclude = true;
        self
    }

    pub fn case_sensitive(mut self) -> Self {
        self.case_insensitive = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizationOptions {
    pub normalize_unicode: bool,
    pub normalize_width: bool,
    pub strip_diacritics: bool,
    pub normalize_case: bool,
}

impl Default for NormalizationOptions {
    fn default() -> Self {
        Self {
            normalize_unicode: true,
            normalize_width: true,
            strip_diacritics: false,
            normalize_case: true,
        }
    }
}