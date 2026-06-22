//! Honesty Gate — programmatic interceptor that prevents the LLM from issuing
//! false praise when the deterministic score does not support it.
//!
//! ## Design rationale
//! The LLM may hallucinate encouragement ("Excellent work!", "Perfect!") even when
//! the similarity score is objectively low. Relying solely on a system prompt
//! instruction is insufficient — this module provides a hard, code-level guardrail
//! that cannot be bypassed by model drift or adversarial prompts.
//!
//! ## Thresholds
//! | Score range   | Allowed sentiment       |
//! |---------------|-------------------------|
//! | 0 – 49        | corrective only         |
//! | 50 – 74       | neutral / constructive  |
//! | 75 – 100      | praise is acceptable    |
//!
//! If the LLM text contains banned praise tokens for a given score bucket, the
//! entire response is replaced by a deterministic fallback template.

/// Praise-laden keywords that are not acceptable for low-scoring attempts.
/// This list targets both Portuguese and English since the LLM may mix languages.
const HIGH_PRAISE_TOKENS_LOW: &[&str] = &[
    "perfeito",
    "excelente",
    "perfeita",
    "parabéns",
    "incrível",
    "ótimo",
    "muito bem",
    "perfeição",
    "perfect",
    "excellent",
    "amazing",
    "great job",
    "well done",
    "outstanding",
    "flawless",
];

/// Mid-tier praise keywords that are unacceptable for very-low-scoring attempts (< 50).
const HIGH_PRAISE_TOKENS_MID: &[&str] = &[
    "bom trabalho",
    "quase lá",
    "quase perfeito",
    "bom",
    "good job",
    "nearly there",
    "almost perfect",
];

/// Score threshold below which *any* praise is considered false.
const SCORE_THRESHOLD_STRICT: f64 = 50.0;

/// Score threshold below which mid-tier praise is also considered false.
const SCORE_THRESHOLD_MODERATE: f64 = 75.0;

/// Result of the gate evaluation.
#[derive(Debug, PartialEq)]
pub enum GateVerdict {
    /// LLM output passes — return it unchanged.
    Pass,
    /// LLM output fails the honesty check — a fallback was applied.
    FallbackApplied,
}

/// Applies the Honesty Gate to an LLM-generated feedback string.
///
/// # Arguments
/// * `score`   – The deterministic similarity score (0.0 – 100.0).
/// * `text`    – The raw LLM response text.
/// * `word`    – The target vocabulary word (used in the fallback template).
/// * `reading` – The romanised / hiragana reading (used in the fallback template).
///
/// # Returns
/// A `(String, GateVerdict)` tuple: the (possibly replaced) text and a verdict
/// indicating whether the fallback was triggered.
pub fn validate_feedback(
    score: f64,
    text: &str,
    word: &str,
    reading: &str,
) -> (String, GateVerdict) {
    let lower = text.to_lowercase();

    let contains_high_praise = HIGH_PRAISE_TOKENS_LOW
        .iter()
        .any(|token| lower.contains(token));

    let contains_mid_praise = HIGH_PRAISE_TOKENS_MID
        .iter()
        .any(|token| lower.contains(token));

    // Determine if the LLM output violates the honesty contract.
    let is_dishonest = if score < SCORE_THRESHOLD_STRICT {
        // Any praise is forbidden.
        contains_high_praise || contains_mid_praise
    } else if score < SCORE_THRESHOLD_MODERATE {
        // Only excessive praise is forbidden.
        contains_high_praise
    } else {
        // Score ≥ 75: praise is earned — always pass.
        false
    };

    if !is_dishonest {
        return (text.to_string(), GateVerdict::Pass);
    }

    // --- Fallback template ---
    let feedback = build_fallback(score, word, reading);
    (feedback, GateVerdict::FallbackApplied)
}

/// Builds a deterministic fallback feedback message calibrated to the score bucket.
fn build_fallback(score: f64, word: &str, reading: &str) -> String {
    if score < 30.0 {
        format!(
            "A pronúncia registrada está muito distante de **{}** ({}). \
             Tente praticar cada sílaba individualmente em voz alta antes de gravar. \
             Score: {:.0}/100.",
            word, reading, score
        )
    } else if score < 50.0 {
        format!(
            "Há diferenças significativas em relação a **{}** ({}). \
             Preste atenção à sequência de sons e ao ritmo da palavra. \
             Score: {:.0}/100.",
            word, reading, score
        )
    } else {
        format!(
            "Sua pronúncia de **{}** ({}) está na direção certa, mas ainda precisa de ajustes. \
             Concentre-se na clareza de cada vogal e na duração das sílabas. \
             Score: {:.0}/100.",
            word, reading, score
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_valid_corrective_feedback() {
        let (text, verdict) = validate_feedback(
            30.0,
            "Sua pronúncia precisa melhorar bastante. Tente ouvir o áudio novamente.",
            "水",
            "みず",
        );
        assert_eq!(verdict, GateVerdict::Pass);
        assert!(text.contains("precisa melhorar"));
    }

    #[test]
    fn blocks_false_praise_at_low_score() {
        let (text, verdict) = validate_feedback(
            25.0,
            "Excelente trabalho! Sua pronúncia está perfeita.",
            "水",
            "みず",
        );
        assert_eq!(verdict, GateVerdict::FallbackApplied);
        assert!(text.contains("25"));
    }

    #[test]
    fn blocks_mid_praise_below_50() {
        let (text, verdict) = validate_feedback(
            40.0,
            "Bom trabalho, quase lá!",
            "橋",
            "はし",
        );
        assert_eq!(verdict, GateVerdict::FallbackApplied);
        assert!(text.contains("40"));
    }

    #[test]
    fn allows_mild_praise_between_50_and_75() {
        let (text, verdict) = validate_feedback(
            60.0,
            "Bom trabalho, continue praticando!",
            "橋",
            "はし",
        );
        assert_eq!(verdict, GateVerdict::Pass);
        assert!(text.contains("Bom trabalho"));
    }

    #[test]
    fn blocks_excessive_praise_between_50_and_75() {
        let (text, verdict) = validate_feedback(
            65.0,
            "Perfeito! Sua pronúncia está excelente.",
            "橋",
            "はし",
        );
        assert_eq!(verdict, GateVerdict::FallbackApplied);
        assert!(text.contains("65"));
    }

    #[test]
    fn allows_all_praise_above_75() {
        let (text, verdict) = validate_feedback(
            90.0,
            "Excelente! Pronúncia quase perfeita.",
            "ありがとう",
            "ありがとう",
        );
        assert_eq!(verdict, GateVerdict::Pass);
        assert!(text.contains("Excelente"));
    }
}
