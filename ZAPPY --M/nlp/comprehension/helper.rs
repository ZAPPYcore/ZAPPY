use regex::Regex;

/// Normalizes whitespace and lowercases content.
#[must_use]
pub fn normalize(text: &str) -> String {
    let mut normalized = text.trim().to_lowercase();
    normalized = normalized.replace('\n', " ");
    Regex::new(r"\s+")
        .unwrap()
        .replace_all(&normalized, " ")
        .into_owned()
}

/// Splits text into sentences using punctuation heuristics.
#[must_use]
pub fn split_sentences(text: &str) -> Vec<String> {
    let sentence_re = Regex::new(r"(?m)([^.!?]+[.!?])").unwrap();
    sentence_re
        .captures_iter(text)
        .map(|cap| cap[1].trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_reduces_whitespace() {
        let result = normalize("Hello   WORLD\nExample");
        assert_eq!(result, "hello world example");
    }

    #[test]
    fn split_sentences_detects_boundaries() {
        let sentences = split_sentences("One. Two! Three?");
        assert_eq!(sentences.len(), 3);
    }
}
