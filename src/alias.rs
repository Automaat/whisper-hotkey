use crate::config::AliasesConfig;
use tracing::{debug, info};

/// Apply alias matching to transcribed text
///
/// Performs case-insensitive fuzzy matching against configured aliases.
/// Returns the best matching alias value if similarity >= threshold,
/// otherwise returns the original text.
///
/// # Performance
/// For typical usage (<10 aliases), string allocations are negligible as this
/// runs in a background thread. For 50+ aliases, consider caching normalized
/// triggers in a preprocessed `HashMap` to reduce allocations.
///
/// # Arguments
/// * `text` - Transcribed text from Whisper
/// * `config` - Alias configuration with entries and threshold
///
/// # Returns
/// Matched alias output or original text
pub fn apply_aliases(text: &str, config: &AliasesConfig) -> String {
    // Return original if disabled or no aliases configured
    if !config.enabled || config.entries.is_empty() {
        return text.to_owned();
    }

    let normalized_text = text.to_lowercase();
    let mut best_match: Option<(&str, f64)> = None;

    // Find best matching alias
    for (trigger, output) in &config.entries {
        let normalized_trigger = trigger.to_lowercase();
        let similarity = strsim::jaro_winkler(&normalized_text, &normalized_trigger);

        debug!(
            trigger = trigger,
            similarity = %similarity,
            threshold = %config.threshold,
            "alias match check"
        );

        if similarity >= config.threshold {
            if let Some((_, best_score)) = best_match {
                if similarity > best_score {
                    best_match = Some((output.as_str(), similarity));
                }
            } else {
                best_match = Some((output.as_str(), similarity));
            }
        }
    }

    // Return best match or original text
    if let Some((output, score)) = best_match {
        info!(
            original = text,
            output = output,
            similarity = %score,
            "alias matched"
        );
        output.to_owned()
    } else {
        debug!(text = text, "no alias match, using original");
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_disabled_returns_original() {
        let mut entries = HashMap::new();
        entries.insert("run tests".to_owned(), "make test".to_owned());

        let config = AliasesConfig {
            enabled: false,
            threshold: 0.8,
            entries,
        };

        assert_eq!(apply_aliases("run tests", &config), "run tests");
    }

    #[test]
    fn test_no_entries_returns_original() {
        let config = AliasesConfig {
            enabled: true,
            threshold: 0.8,
            entries: HashMap::new(),
        };

        assert_eq!(apply_aliases("run tests", &config), "run tests");
    }

    #[test]
    fn test_exact_match() {
        let mut entries = HashMap::new();
        entries.insert(
            "run tests".to_owned(),
            "make test UPDATE_GOLDEN_FILES=true".to_owned(),
        );

        let config = AliasesConfig {
            enabled: true,
            threshold: 0.8,
            entries,
        };

        assert_eq!(
            apply_aliases("run tests", &config),
            "make test UPDATE_GOLDEN_FILES=true"
        );
    }

    #[test]
    fn test_case_insensitive() {
        let mut entries = HashMap::new();
        entries.insert("run tests".to_owned(), "make test".to_owned());

        let config = AliasesConfig {
            enabled: true,
            threshold: 0.8,
            entries,
        };

        assert_eq!(apply_aliases("Run Tests", &config), "make test");
        assert_eq!(apply_aliases("RUN TESTS", &config), "make test");
        assert_eq!(apply_aliases("run TESTS", &config), "make test");
    }

    #[test]
    fn test_fuzzy_match_typo() {
        let mut entries = HashMap::new();
        entries.insert("run tests".to_owned(), "make test".to_owned());

        let config = AliasesConfig {
            enabled: true,
            threshold: 0.8,
            entries,
        };

        // "runtests" should match "run tests" with high similarity
        assert_eq!(apply_aliases("runtests", &config), "make test");
    }

    #[test]
    fn test_below_threshold_returns_original() {
        let mut entries = HashMap::new();
        entries.insert("run tests".to_owned(), "make test".to_owned());

        let config = AliasesConfig {
            enabled: true,
            threshold: 0.9, // High threshold
            entries,
        };

        // "testing" should not match "run tests" at 0.9 threshold
        assert_eq!(apply_aliases("testing", &config), "testing");
    }

    #[test]
    fn test_best_match_wins() {
        let mut entries = HashMap::new();
        entries.insert("run tests".to_owned(), "make test".to_owned());
        entries.insert("run all tests".to_owned(), "make test-all".to_owned());
        entries.insert("commit".to_owned(), "git commit -s -S".to_owned());

        let config = AliasesConfig {
            enabled: true,
            threshold: 0.5,
            entries,
        };

        // "run tests" should match "run tests" better than "run all tests"
        assert_eq!(apply_aliases("run tests", &config), "make test");

        // "commit" should match only "commit"
        assert_eq!(apply_aliases("commit", &config), "git commit -s -S");
    }

    #[test]
    fn test_multiple_aliases() {
        let mut entries = HashMap::new();
        entries.insert("run tests".to_owned(), "make test".to_owned());
        entries.insert("commit".to_owned(), "git commit -s -S".to_owned());
        entries.insert("push".to_owned(), "git push".to_owned());

        let config = AliasesConfig {
            enabled: true,
            threshold: 0.8,
            entries,
        };

        assert_eq!(apply_aliases("run tests", &config), "make test");
        assert_eq!(apply_aliases("commit", &config), "git commit -s -S");
        assert_eq!(apply_aliases("push", &config), "git push");
    }

    #[test]
    fn test_no_match_returns_original() {
        let mut entries = HashMap::new();
        entries.insert("run tests".to_owned(), "make test".to_owned());

        let config = AliasesConfig {
            enabled: true,
            threshold: 0.8,
            entries,
        };

        assert_eq!(apply_aliases("hello world", &config), "hello world");
    }

    #[test]
    fn test_empty_text() {
        let mut entries = HashMap::new();
        entries.insert("run tests".to_owned(), "make test".to_owned());

        let config = AliasesConfig {
            enabled: true,
            threshold: 0.8,
            entries,
        };

        assert_eq!(apply_aliases("", &config), "");
    }

    #[test]
    fn test_similarity_score_boundary() {
        let mut entries = HashMap::new();
        entries.insert("test".to_owned(), "output".to_owned());

        // Test at exact threshold
        let config = AliasesConfig {
            enabled: true,
            threshold: 0.0, // Accept any match
            entries,
        };

        // Even very different strings should match at threshold 0.0
        assert_eq!(apply_aliases("completely different", &config), "output");
    }
}
