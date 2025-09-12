use unicode_normalization::{UnicodeNormalization, is_nfc};
use filemover_types::{NormalizationOptions, FileMoverError};

pub struct TextNormalizer {
    options: NormalizationOptions,
}

impl TextNormalizer {
    pub fn new(options: NormalizationOptions) -> Self {
        Self { options }
    }

    pub fn normalize(&self, text: &str) -> Result<String, FileMoverError> {
        let mut result = text.to_string();

        // Unicode正規化 (NFC)
        if self.options.normalize_unicode {
            if !is_nfc(&result) {
                result = result.nfc().collect();
            }
        }

        // 全角半角正規化
        if self.options.normalize_width {
            result = self.normalize_width(&result);
        }

        // ダイアクリティクス除去
        if self.options.strip_diacritics {
            result = self.strip_diacritics(&result);
        }

        // 大文字小文字正規化
        if self.options.normalize_case {
            result = result.to_lowercase();
        }

        Ok(result)
    }

    fn normalize_width(&self, text: &str) -> String {
        text.chars()
            .map(|c| match c {
                // 全角英数字を半角に
                '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap_or(c),
                'Ａ'..='Ｚ' => char::from_u32(c as u32 - 'Ａ' as u32 + 'A' as u32).unwrap_or(c),
                'ａ'..='ｚ' => char::from_u32(c as u32 - 'ａ' as u32 + 'a' as u32).unwrap_or(c),
                // 全角スペースを半角に
                '　' => ' ',
                _ => c,
            })
            .collect()
    }

    fn strip_diacritics(&self, text: &str) -> String {
        text.nfd()
            .filter(|&c| !unicode_normalization::char::is_combining_mark(c))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_width() {
        let options = NormalizationOptions {
            normalize_unicode: false,
            normalize_width: true,
            strip_diacritics: false,
            normalize_case: false,
        };
        let normalizer = TextNormalizer::new(options);
        
        let result = normalizer.normalize("Ｈｅｌｌｏ１２３").unwrap();
        assert_eq!(result, "Hello123");
    }

    #[test]
    fn test_strip_diacritics() {
        let options = NormalizationOptions {
            normalize_unicode: true,
            normalize_width: false,
            strip_diacritics: true,
            normalize_case: false,
        };
        let normalizer = TextNormalizer::new(options);
        
        let result = normalizer.normalize("café naïve résumé").unwrap();
        assert_eq!(result, "cafe naive resume");
    }

    #[test]
    fn test_full_normalization() {
        let options = NormalizationOptions::default();
        let normalizer = TextNormalizer::new(options);
        
        let result = normalizer.normalize("Ｃａｆé　Ｎａïｖｅ").unwrap();
        assert_eq!(result, "cafe naive");
    }
}