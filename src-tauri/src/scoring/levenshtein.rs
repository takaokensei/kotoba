//! Normalized Levenshtein distance for pronunciation scoring (V1).

/// Computes the Levenshtein edit distance between two strings.
pub fn distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Returns a similarity score in [0.0, 1.0] where 1.0 is an exact match.
pub fn normalized_similarity(target: &str, spoken: &str) -> f64 {
    let target_norm = normalize_for_compare(target);
    let spoken_norm = normalize_for_compare(spoken);

    if target_norm.is_empty() && spoken_norm.is_empty() {
        return 1.0;
    }

    let max_len = target_norm.chars().count().max(spoken_norm.chars().count());
    if max_len == 0 {
        return 1.0;
    }

    let dist = distance(&target_norm, &spoken_norm);
    1.0 - (dist as f64 / max_len as f64)
}

/// Strips Japanese and Western punctuation before comparison so that
/// Whisper voice-inflection artifacts (e.g. "ありがとう！") do not
/// penalise otherwise perfect matches.
fn katakana_to_hiragana(c: char) -> char {
    let cp = c as u32;
    if (0x30A1..=0x30F6).contains(&cp) {
        std::char::from_u32(cp - 0x60).unwrap_or(c)
    } else {
        c
    }
}

/// Strips Japanese and Western punctuation, and normalises Katakana to Hiragana
/// before comparison so that Whisper voice-inflection artifacts (e.g. "ありがとう！")
/// and kana variation (e.g. "キップ" vs "きっぷ") do not penalise matches.
fn normalize_for_compare(s: &str) -> String {
    const STRIP: &[char] = &[
        // Japanese punctuation
        '。', '、', '！', '？', '「', '」', '『', '』', '・', '…', '〜',
        '—', '–',
        // Western punctuation
        '!', '?', '.', ',', ':', ';', '"', '\'', '(', ')', '-',
    ];
    s.trim()
        .to_lowercase()
        .chars()
        .filter(|c| !STRIP.contains(c))
        .map(katakana_to_hiragana)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_strings_score_one() {
        assert!((normalized_similarity("hello", "hello") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn case_insensitive_match() {
        assert!((normalized_similarity("Hello", "hello") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn single_substitution() {
        let score = normalized_similarity("cat", "bat");
        assert!((score - 0.666666).abs() < 0.01);
    }

    #[test]
    fn empty_spoken_is_zero() {
        assert!((normalized_similarity("hello", "") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn distance_known_values() {
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("kitten", "sitting"), 3);
    }

    #[test]
    fn japanese_punctuation_stripped_perfect_match() {
        // Whisper often appends 。or！based on voice inflection — must not penalise
        assert!(
            (normalized_similarity("ありがとう", "ありがとう！") - 1.0).abs() < f64::EPSILON,
            "trailing ! should be stripped before comparison"
        );
        assert!(
            (normalized_similarity("ありがとう", "ありがとう。") - 1.0).abs() < f64::EPSILON,
            "trailing 。 should be stripped before comparison"
        );
    }

    #[test]
    fn western_punctuation_stripped_perfect_match() {
        assert!(
            (normalized_similarity("hello", "hello!") - 1.0).abs() < f64::EPSILON,
            "trailing Western ! should be stripped"
        );
        assert!(
            (normalized_similarity("water", "water.") - 1.0).abs() < f64::EPSILON,
            "trailing period should be stripped"
        );
    }

    #[test]
    fn katakana_to_hiragana_perfect_match() {
        assert!(
            (normalized_similarity("きっぷ", "キップ") - 1.0).abs() < f64::EPSILON,
            "katakana and hiragana should match perfectly"
        );
        assert!(
            (normalized_similarity("ありがとう", "アリガトウ") - 1.0).abs() < f64::EPSILON,
            "katakana and hiragana should match perfectly"
        );
    }
}
