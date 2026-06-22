//! Phonemic edit distance calculations.
//!
//! Calculates distance and similarity metrics over vectors of IPA phonemes.

/// Computes the Levenshtein edit distance between two slices of phoneme tokens.
pub fn distance(a: &[String], b: &[String]) -> usize {
    let len_a = a.len();
    let len_b = b.len();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut dp = vec![vec![0; len_b + 1]; len_a + 1];

    for i in 0..=len_a {
        dp[i][0] = i;
    }
    for j in 0..=len_b {
        dp[0][j] = j;
    }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = std::cmp::min(
                std::cmp::min(dp[i - 1][j] + 1, dp[i][j - 1] + 1),
                dp[i - 1][j - 1] + cost,
            );
        }
    }

    dp[len_a][len_b]
}

/// Computes a normalized phoneme similarity score in the range [0.0, 1.0].
pub fn normalized_similarity(a: &[String], b: &[String]) -> f64 {
    let max_len = std::cmp::max(a.len(), b.len());
    if max_len == 0 {
        return 1.0;
    }
    let dist = distance(a, b);
    1.0 - (dist as f64 / max_len as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_vec(slice: &[&str]) -> Vec<String> {
        slice.iter().map(|&s| s.to_string()).collect()
    }

    #[test]
    fn test_identical_phonemes() {
        let a = to_vec(&["h", "ə", "l", "oʊ"]);
        let b = to_vec(&["h", "ə", "l", "oʊ"]);
        assert_eq!(distance(&a, &b), 0);
        assert_eq!(normalized_similarity(&a, &b), 1.0);
    }

    #[test]
    fn test_phonetic_omission() {
        // e.g. speaker drops final 'l' from "beautiful"
        let target = to_vec(&["b", "j", "uː", "t", "ɪ", "f", "ʊ", "l"]);
        let spoken = to_vec(&["b", "j", "uː", "t", "ɪ", "f", "ʊ"]);
        assert_eq!(distance(&target, &spoken), 1);
        assert_eq!(normalized_similarity(&target, &spoken), 0.875);
    }

    #[test]
    fn test_phonetic_substitution() {
        // e.g. speaker replaces alveolar tap 'ɾ' with uvular 'ʁ' (pt-BR interference)
        let target = to_vec(&["a", "ɾ", "i", "ɡ", "a", "t", "o", "u"]);
        let spoken = to_vec(&["a", "ʁ", "i", "ɡ", "a", "t", "o", "u"]);
        assert_eq!(distance(&target, &spoken), 1);
        assert_eq!(normalized_similarity(&target, &spoken), 0.875);
    }

    #[test]
    fn test_phonetic_insertion() {
        // e.g. speaker inserts 'i' at the end (typical brazilian adding epenthetic i)
        let target = to_vec(&["b", "u", "k"]);
        let spoken = to_vec(&["b", "u", "k", "i"]);
        assert_eq!(distance(&target, &spoken), 1);
        assert_eq!(normalized_similarity(&target, &spoken), 0.75);
    }

    #[test]
    fn test_empty_slices() {
        let empty: Vec<String> = vec![];
        assert_eq!(distance(&empty, &empty), 0);
        assert_eq!(normalized_similarity(&empty, &empty), 1.0);
    }
}
