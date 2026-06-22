//! G2P (Grapheme-to-Phoneme) bifurcation dispatcher — ADR-008.
//!
//! Routes text phonemization by language:
//!   - `ja`: MeCab + UniDic sidecar → Katakana → IPA static table.
//!         Fallback: direct Kana→IPA mapping when MeCab is unavailable.
//!   - `en`: Static IPA vocabulary table + grapheme fallback.
//!
//! # Architecture note (ADR-008 / Section 8 Performance Budget)
//! All functions in this module are **purely synchronous**. Async DB lookups
//! for sidecar paths are performed at the Tauri command layer (practice.rs)
//! and injected here as `mecab_path_override: Option<&str>`.

use std::path::{Path, PathBuf};
use tracing::{info, warn};

// ─── Sidecar path helpers ────────────────────────────────────────────────────

fn resolve_models_dir() -> std::path::PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("kotoba").join("models")
}

/// Resolves the path to the MeCab executable and its dictionary directory.
///
/// `mecab_path_override` is the path string pre-fetched from `model_manifest`
/// via an async `.await` at the command layer. Passing `None` triggers the
/// default AppData fallback — used in unit tests and when the DB record is absent.
fn resolve_mecab_paths(mecab_path_override: Option<&str>) -> Option<(PathBuf, PathBuf)> {
    // 1. Use the caller-supplied path (queried asynchronously before entering this fn)
    if let Some(path_str) = mecab_path_override {
        let exe_path = PathBuf::from(path_str);
        if exe_path.exists() {
            if let Some(dir) = exe_path.parent().map(|p| p.to_path_buf()) {
                return Some((exe_path, dir));
            }
        }
    }

    // 2. Fallback: check default location in AppData
    let mecab_dir = resolve_models_dir().join("mecab-unidic");
    let mecab_exe = if cfg!(target_os = "windows") {
        mecab_dir.join("mecab.exe")
    } else {
        mecab_dir.join("mecab")
    };

    if mecab_exe.exists() {
        return Some((mecab_exe, mecab_dir));
    }

    None
}

/// Returns `true` if a usable MeCab installation can be located.
pub fn is_mecab_available(mecab_path_override: Option<&str>) -> bool {
    resolve_mecab_paths(mecab_path_override).is_some()
}

// ─── MeCab sidecar invocation (Section 7-F lifecycle) ───────────────────────

fn run_mecab(text: &str, exe_path: &Path, dict_dir: &Path) -> Option<String> {
    info!(exe = %exe_path.display(), dict = %dict_dir.display(), text, "sidecar lifecycle: loading MeCab");

    let mut child = std::process::Command::new(exe_path)
        .current_dir(dict_dir)
        .arg("-d")
        .arg(dict_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(text.as_bytes());
        // drop stdin → sends EOF to MeCab
    }

    let output = child.wait_with_output().ok()?;
    info!("sidecar lifecycle: unloading MeCab");

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(stderr = %stderr, "MeCab process failed");
        None
    }
}

/// Parses MeCab UniDic tab-separated output to extract the Katakana 読み field.
/// UniDic format (simplified): surface \t pos,pos2,...,orthBase,read,...
fn parse_mecab_reading(mecab_output: &str) -> String {
    let mut reading = String::new();
    for line in mecab_output.lines() {
        let trimmed = line.trim();
        if trimmed == "EOS" || trimmed.is_empty() {
            continue;
        }
        if let Some(tab_idx) = trimmed.find('\t') {
            let features: Vec<&str> = trimmed[tab_idx + 1..].split(',').collect();
            // UniDic field indices: 7 = lemmaReadingForm, 6 = orthBaseForm, 9 = pronunciationForm
            let reading_token = if features.len() > 7 && features[7] != "*" && !features[7].is_empty() {
                features[7]
            } else if features.len() > 9 && features[9] != "*" && !features[9].is_empty() {
                features[9]
            } else {
                &trimmed[..tab_idx] // surface form fallback
            };
            reading.push_str(reading_token);
        }
    }
    reading
}

// ─── Sokuon helper ──────────────────────────────────────────────────────────

fn sokuon_consonant(next: char) -> &'static str {
    match next {
        'か' | 'き' | 'く' | 'け' | 'こ' | 'カ' | 'キ' | 'ク' | 'ケ' | 'コ' => "k",
        'さ' | 'す' | 'せ' | 'そ' | 'サ' | 'ス' | 'セ' | 'ソ' => "s",
        'し' | 'シ' => "s",
        'た' | 'て' | 'と' | 'タ' | 'テ' | 'ト' => "t",
        'ち' | 'チ' => "t",
        'つ' | 'ツ' => "t",
        'ぱ' | 'ぴ' | 'ぷ' | 'ぺ' | 'ぽ' | 'パ' | 'ピ' | 'プ' | 'ペ' | 'ポ' => "p",
        'ば' | 'び' | 'ぶ' | 'べ' | 'ぼ' | 'バ' | 'ビ' | 'ブ' | 'ベ' | 'ボ' => "b",
        'だ' | 'で' | 'ど' | 'ダ' | 'デ' | 'ド' => "d",
        'が' | 'ぎ' | 'ぐ' | 'げ' | 'ご' | 'ガ' | 'ギ' | 'グ' | 'ゲ' | 'ゴ' => "g",
        _ => "",
    }
}

// ─── Katakana/Hiragana → IPA mapping ────────────────────────────────────────
//
// ~100-entry static table covering the complete Japanese phonological inventory.
// Critical pt-BR interference points handled:
//   • Long vowels:  アー → /aː/
//   • Geminates/sokuon: っ/ッ → duplicates following consonant
//   • Alveolar tap: ら行 → /ɾ/ (not /l/ or /r/)
//   • Palatal fricative: し → /ʃi/, ひ → /çi/
//   • Bilabial fricative: ふ → /ɸu/

pub fn kana_to_ipa(text: &str) -> Vec<String> {
    let mut ipa: Vec<String> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // ── Digraphs (consonant + small vowel) ──────────────────────────────
        if i + 1 < chars.len() {
            let digraph: Option<&[&str]> = match (chars[i], chars[i + 1]) {
                // きゃ行
                ('き', 'ゃ') | ('キ', 'ャ') => Some(&["k", "j", "a"]),
                ('き', 'ゅ') | ('キ', 'ュ') => Some(&["k", "j", "u"]),
                ('き', 'ょ') | ('キ', 'ョ') => Some(&["k", "j", "o"]),
                // しゃ行
                ('し', 'ゃ') | ('シ', 'ャ') => Some(&["ʃ", "a"]),
                ('し', 'ゅ') | ('シ', 'ュ') => Some(&["ʃ", "u"]),
                ('し', 'ょ') | ('シ', 'ョ') => Some(&["ʃ", "o"]),
                // ちゃ行
                ('ち', 'ゃ') | ('チ', 'ャ') => Some(&["tʃ", "a"]),
                ('ち', 'ゅ') | ('チ', 'ュ') => Some(&["tʃ", "u"]),
                ('ち', 'ょ') | ('チ', 'ョ') => Some(&["tʃ", "o"]),
                // にゃ行
                ('に', 'ゃ') | ('ニ', 'ャ') => Some(&["ɲ", "a"]),
                ('に', 'ゅ') | ('ニ', 'ュ') => Some(&["ɲ", "u"]),
                ('に', 'ょ') | ('ニ', 'ョ') => Some(&["ɲ", "o"]),
                // ひゃ行
                ('ひ', 'ゃ') | ('ヒ', 'ャ') => Some(&["ç", "a"]),
                ('ひ', 'ゅ') | ('ヒ', 'ュ') => Some(&["ç", "u"]),
                ('ひ', 'ょ') | ('ヒ', 'ョ') => Some(&["ç", "o"]),
                // みゃ行
                ('み', 'ゃ') | ('ミ', 'ャ') => Some(&["m", "j", "a"]),
                ('み', 'ゅ') | ('ミ', 'ュ') => Some(&["m", "j", "u"]),
                ('み', 'ょ') | ('ミ', 'ョ') => Some(&["m", "j", "o"]),
                // りゃ行
                ('り', 'ゃ') | ('リ', 'ャ') => Some(&["ɾ", "j", "a"]),
                ('り', 'ゅ') | ('リ', 'ュ') => Some(&["ɾ", "j", "u"]),
                ('り', 'ょ') | ('リ', 'ョ') => Some(&["ɾ", "j", "o"]),
                // ぎゃ行
                ('ぎ', 'ゃ') | ('ギ', 'ャ') => Some(&["ɡ", "j", "a"]),
                ('ぎ', 'ゅ') | ('ギ', 'ュ') => Some(&["ɡ", "j", "u"]),
                ('ぎ', 'ょ') | ('ギ', 'ョ') => Some(&["ɡ", "j", "o"]),
                // じゃ行
                ('じ', 'ゃ') | ('ジ', 'ャ') => Some(&["dʒ", "a"]),
                ('じ', 'ゅ') | ('ジ', 'ュ') => Some(&["dʒ", "u"]),
                ('じ', 'ょ') | ('ジ', 'ョ') => Some(&["dʒ", "o"]),
                // びゃ行
                ('び', 'ゃ') | ('ビ', 'ャ') => Some(&["b", "j", "a"]),
                ('び', 'ゅ') | ('ビ', 'ュ') => Some(&["b", "j", "u"]),
                ('び', 'ょ') | ('ビ', 'ョ') => Some(&["b", "j", "o"]),
                // ぴゃ行
                ('ぴ', 'ゃ') | ('ピ', 'ャ') => Some(&["p", "j", "a"]),
                ('ぴ', 'ゅ') | ('ピ', 'ュ') => Some(&["p", "j", "u"]),
                ('ぴ', 'ょ') | ('ピ', 'ョ') => Some(&["p", "j", "o"]),
                _ => None,
            };

            if let Some(phonemes) = digraph {
                for p in phonemes {
                    ipa.push(p.to_string());
                }
                i += 2;
                continue;
            }
        }

        // ── Single character mappings ────────────────────────────────────────
        match chars[i] {
            // Vowels
            'あ' | 'ア' => { ipa.push("a".into()); }
            'い' | 'イ' => { ipa.push("i".into()); }
            'う' | 'ウ' => { ipa.push("u".into()); }
            'え' | 'エ' => { ipa.push("e".into()); }
            'お' | 'オ' => { ipa.push("o".into()); }
            // か行
            'か' | 'カ' => { ipa.extend_from_slice(&["k".into(), "a".into()]); }
            'き' | 'キ' => { ipa.extend_from_slice(&["k".into(), "i".into()]); }
            'く' | 'ク' => { ipa.extend_from_slice(&["k".into(), "u".into()]); }
            'け' | 'ケ' => { ipa.extend_from_slice(&["k".into(), "e".into()]); }
            'こ' | 'コ' => { ipa.extend_from_slice(&["k".into(), "o".into()]); }
            // さ行
            'さ' | 'サ' => { ipa.extend_from_slice(&["s".into(), "a".into()]); }
            'し' | 'シ' => { ipa.extend_from_slice(&["ʃ".into(), "i".into()]); }
            'す' | 'ス' => { ipa.extend_from_slice(&["s".into(), "u".into()]); }
            'せ' | 'セ' => { ipa.extend_from_slice(&["s".into(), "e".into()]); }
            'そ' | 'ソ' => { ipa.extend_from_slice(&["s".into(), "o".into()]); }
            // た行
            'た' | 'タ' => { ipa.extend_from_slice(&["t".into(), "a".into()]); }
            'ち' | 'チ' => { ipa.extend_from_slice(&["tʃ".into(), "i".into()]); }
            'つ' | 'ツ' => { ipa.extend_from_slice(&["ts".into(), "u".into()]); }
            'て' | 'テ' => { ipa.extend_from_slice(&["t".into(), "e".into()]); }
            'と' | 'ト' => { ipa.extend_from_slice(&["t".into(), "o".into()]); }
            // な行
            'な' | 'ナ' => { ipa.extend_from_slice(&["n".into(), "a".into()]); }
            'に' | 'ニ' => { ipa.extend_from_slice(&["ɲ".into(), "i".into()]); }
            'ぬ' | 'ヌ' => { ipa.extend_from_slice(&["n".into(), "u".into()]); }
            'ね' | 'ネ' => { ipa.extend_from_slice(&["n".into(), "e".into()]); }
            'の' | 'ノ' => { ipa.extend_from_slice(&["n".into(), "o".into()]); }
            // は行
            'は' | 'ハ' => { ipa.extend_from_slice(&["h".into(), "a".into()]); }
            'ひ' | 'ヒ' => { ipa.extend_from_slice(&["ç".into(), "i".into()]); }
            'ふ' | 'フ' => { ipa.extend_from_slice(&["ɸ".into(), "u".into()]); }
            'へ' | 'ヘ' => { ipa.extend_from_slice(&["h".into(), "e".into()]); }
            'ほ' | 'ホ' => { ipa.extend_from_slice(&["h".into(), "o".into()]); }
            // ま行
            'ま' | 'マ' => { ipa.extend_from_slice(&["m".into(), "a".into()]); }
            'み' | 'ミ' => { ipa.extend_from_slice(&["m".into(), "i".into()]); }
            'む' | 'ム' => { ipa.extend_from_slice(&["m".into(), "u".into()]); }
            'め' | 'メ' => { ipa.extend_from_slice(&["m".into(), "e".into()]); }
            'も' | 'モ' => { ipa.extend_from_slice(&["m".into(), "o".into()]); }
            // や行
            'や' | 'ヤ' => { ipa.extend_from_slice(&["j".into(), "a".into()]); }
            'ゆ' | 'ユ' => { ipa.extend_from_slice(&["j".into(), "u".into()]); }
            'よ' | 'ヨ' => { ipa.extend_from_slice(&["j".into(), "o".into()]); }
            // ら行 — alveolar tap /ɾ/ (critical pt-BR interference edge case)
            'ら' | 'ラ' => { ipa.extend_from_slice(&["ɾ".into(), "a".into()]); }
            'り' | 'リ' => { ipa.extend_from_slice(&["ɾ".into(), "i".into()]); }
            'る' | 'ル' => { ipa.extend_from_slice(&["ɾ".into(), "u".into()]); }
            'れ' | 'レ' => { ipa.extend_from_slice(&["ɾ".into(), "e".into()]); }
            'ろ' | 'ロ' => { ipa.extend_from_slice(&["ɾ".into(), "o".into()]); }
            // わ行
            'わ' | 'ワ' => { ipa.extend_from_slice(&["w".into(), "a".into()]); }
            'を' | 'ヲ' => { ipa.push("o".into()); }
            'ん' | 'ン' => { ipa.push("ɲ".into()); }
            // が行 (voiced velars)
            'が' | 'ガ' => { ipa.extend_from_slice(&["ɡ".into(), "a".into()]); }
            'ぎ' | 'ギ' => { ipa.extend_from_slice(&["ɡ".into(), "i".into()]); }
            'ぐ' | 'グ' => { ipa.extend_from_slice(&["ɡ".into(), "u".into()]); }
            'げ' | 'ゲ' => { ipa.extend_from_slice(&["ɡ".into(), "e".into()]); }
            'ご' | 'ゴ' => { ipa.extend_from_slice(&["ɡ".into(), "o".into()]); }
            // ざ行 (voiced sibilants)
            'ざ' | 'ザ' => { ipa.extend_from_slice(&["z".into(), "a".into()]); }
            'じ' | 'ジ' => { ipa.extend_from_slice(&["dʒ".into(), "i".into()]); }
            'ず' | 'ズ' => { ipa.extend_from_slice(&["z".into(), "u".into()]); }
            'ぜ' | 'ゼ' => { ipa.extend_from_slice(&["z".into(), "e".into()]); }
            'ぞ' | 'ゾ' => { ipa.extend_from_slice(&["z".into(), "o".into()]); }
            // だ行
            'だ' | 'ダ' => { ipa.extend_from_slice(&["d".into(), "a".into()]); }
            'ぢ' | 'ヂ' => { ipa.extend_from_slice(&["dʒ".into(), "i".into()]); }
            'づ' | 'ヅ' => { ipa.extend_from_slice(&["z".into(), "u".into()]); }
            'で' | 'デ' => { ipa.extend_from_slice(&["d".into(), "e".into()]); }
            'ど' | 'ド' => { ipa.extend_from_slice(&["d".into(), "o".into()]); }
            // ば行
            'ば' | 'バ' => { ipa.extend_from_slice(&["b".into(), "a".into()]); }
            'び' | 'ビ' => { ipa.extend_from_slice(&["b".into(), "i".into()]); }
            'ぶ' | 'ブ' => { ipa.extend_from_slice(&["b".into(), "u".into()]); }
            'べ' | 'ベ' => { ipa.extend_from_slice(&["b".into(), "e".into()]); }
            'ぼ' | 'ボ' => { ipa.extend_from_slice(&["b".into(), "o".into()]); }
            // ぱ行
            'ぱ' | 'パ' => { ipa.extend_from_slice(&["p".into(), "a".into()]); }
            'ぴ' | 'ピ' => { ipa.extend_from_slice(&["p".into(), "i".into()]); }
            'ぷ' | 'プ' => { ipa.extend_from_slice(&["p".into(), "u".into()]); }
            'ぺ' | 'ペ' => { ipa.extend_from_slice(&["p".into(), "e".into()]); }
            'ぽ' | 'ポ' => { ipa.extend_from_slice(&["p".into(), "o".into()]); }
            // ─── Special phonological cases ──────────────────────────────────
            // Long vowel mark: アー → /aː/ etc.
            'ー' => {
                if let Some(last) = ipa.last_mut() {
                    if ["a", "i", "u", "e", "o"].contains(&last.as_str()) {
                        *last = format!("{}ː", last);
                    } else {
                        ipa.push("ː".into());
                    }
                } else {
                    ipa.push("ː".into());
                }
            }
            // Sokuon (っ/ッ): double the consonant of the following syllable
            'っ' | 'ッ' => {
                if i + 1 < chars.len() {
                    let c = sokuon_consonant(chars[i + 1]);
                    if !c.is_empty() {
                        ipa.push(c.into());
                    }
                }
            }
            // Small vowels (ya, yu, yo) handled by digraph above; stand-alone mapping
            'ぁ' | 'ァ' => { ipa.push("a".into()); }
            'ぃ' | 'ィ' => { ipa.push("i".into()); }
            'ぅ' | 'ゥ' => { ipa.push("u".into()); }
            'ぇ' | 'ェ' => { ipa.push("e".into()); }
            'ぉ' | 'ォ' => { ipa.push("o".into()); }
            'ゃ' | 'ャ' => { ipa.push("a".into()); }
            'ゅ' | 'ュ' => { ipa.push("u".into()); }
            'ょ' | 'ョ' => { ipa.push("o".into()); }
            _ => { /* ignore punctuation, whitespace, unknowns */ }
        }

        i += 1;
    }

    ipa
}

// ─── MeCab→IPA pipeline ─────────────────────────────────────────────────────

fn mecab_g2p(text: &str, mecab_path_override: Option<&str>) -> Option<Vec<String>> {
    let (mecab_exe, mecab_dir) = resolve_mecab_paths(mecab_path_override)?;

    let raw_out = run_mecab(text, &mecab_exe, &mecab_dir)?;
    let reading = parse_mecab_reading(&raw_out);

    if reading.is_empty() {
        return None;
    }

    Some(kana_to_ipa(&reading))
}

// ─── English static IPA dictionary ──────────────────────────────────────────

fn english_word_to_ipa(word: &str) -> Option<Vec<String>> {
    // Static dictionary covering the seed vocabulary + common pronunciation errors.
    let ipa: &[&str] = match word {
        "hello" => &["h", "ə", "l", "oʊ"],
        "water" => &["w", "ɔː", "t", "ə"],
        "thank" => &["θ", "æ", "ŋ", "k"],
        "thank you" => &["θ", "æ", "ŋ", "k", "j", "uː"],
        "beautiful" => &["b", "j", "uː", "t", "ɪ", "f", "ʊ", "l"],
        "pronunciation" => &["p", "ɹ", "ə", "n", "ʌ", "n", "s", "i", "eɪ", "ʃ", "ə", "n"],
        "apple" => &["æ", "p", "ə", "l"],
        "book" => &["b", "ʊ", "k"],
        "cat" => &["k", "æ", "t"],
        "dog" => &["d", "ɔː", "g"],
        _ => return None,
    };
    Some(ipa.iter().map(|s| s.to_string()).collect())
}

fn english_grapheme_fallback(word: &str) -> Vec<String> {
    word.chars()
        .filter(|c| c.is_alphabetic())
        .map(|c| match c {
            'a' => "æ",  'b' => "b",  'c' => "k",  'd' => "d",
            'e' => "ɛ",  'f' => "f",  'g' => "g",  'h' => "h",
            'i' => "ɪ",  'j' => "dʒ", 'k' => "k",  'l' => "l",
            'm' => "m",  'n' => "n",  'o' => "ɔ",  'p' => "p",
            'q' => "k",  'r' => "ɹ",  's' => "s",  't' => "t",
            'u' => "ʌ",  'v' => "v",  'w' => "w",  'x' => "ks",
            'y' => "j",  'z' => "z",  _   => return "",
        })
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

// ─── Public dispatcher ───────────────────────────────────────────────────────

/// Converts an orthographic string into a vector of clean IPA phoneme tokens.
///
/// # Parameters
/// - `text`                – The target word or phrase.
/// - `lang`                – ISO 639-1 language code (`"ja"` | `"en"`).
/// - `mecab_path_override` – Optional path to the MeCab binary, resolved
///                           asynchronously by the caller before entering this fn.
///                           `None` triggers the AppData default-path fallback.
///
/// # Japanese pipeline
/// 1. Try MeCab + UniDic sidecar (load on demand, unload immediately after).
/// 2. Fallback to direct Kana→IPA when MeCab is not installed.
///
/// # English pipeline
/// 1. Static IPA vocabulary lookup.
/// 2. Grapheme-level approximate fallback.
pub fn g2p(text: &str, lang: &str, mecab_path_override: Option<&str>) -> Vec<String> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return Vec::new();
    }

    match lang {
        "ja" => mecab_g2p(cleaned, mecab_path_override).unwrap_or_else(|| kana_to_ipa(cleaned)),
        "en" => {
            let lower = cleaned.to_lowercase();
            english_word_to_ipa(&lower)
                .unwrap_or_else(|| english_grapheme_fallback(&lower))
        }
        other => {
            warn!(lang = other, "G2P: unknown language — returning empty phoneme list");
            Vec::new()
        }
    }
}

/// Returns `"v2-ipa"` or `"v2-ipa-espeak-fallback"` based on MeCab availability.
/// Used to set `scoring_version` so heterogeneous comparisons are flagged in history.
pub fn scoring_version_tag(lang: &str, mecab_path_override: Option<&str>) -> &'static str {
    if lang == "ja" {
        if is_mecab_available(mecab_path_override) {
            "v2-ipa"
        } else {
            "v2-ipa-espeak-fallback"
        }
    } else {
        "v2-ipa-en-dict"
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // All tests pass `None` for mecab_path_override — no DB available in unit tests.
    // The fallback kana→IPA path is exercised directly.

    #[test]
    fn test_en_hello() {
        assert_eq!(g2p("hello", "en", None), vec!["h", "ə", "l", "oʊ"]);
    }

    #[test]
    fn test_ja_arigato_direct_kana() {
        // ありがとう → a ɾ i ɡ a t o u
        let ipa = kana_to_ipa("ありがとう");
        assert_eq!(ipa, vec!["a", "ɾ", "i", "ɡ", "a", "t", "o", "u"]);
    }

    #[test]
    fn test_ja_long_vowel() {
        // アー → aː
        let ipa = kana_to_ipa("アー");
        assert_eq!(ipa, vec!["aː"]);
    }

    #[test]
    fn test_ja_geminate_sokuon() {
        // きっぷ → k i p p u
        let ipa = kana_to_ipa("きっぷ");
        assert_eq!(ipa, vec!["k", "i", "p", "p", "u"]);
    }

    #[test]
    fn test_ja_alveolar_tap() {
        // ら → ɾ a (not r or l)
        let ipa = kana_to_ipa("ら");
        assert_eq!(ipa, vec!["ɾ", "a"]);
    }

    #[test]
    fn test_ja_digraph_sha() {
        // しゃ → ʃ a
        let ipa = kana_to_ipa("しゃ");
        assert_eq!(ipa, vec!["ʃ", "a"]);
    }

    #[test]
    fn test_ja_water_mizu() {
        // みず → m i z u
        let ipa = kana_to_ipa("みず");
        assert_eq!(ipa, vec!["m", "i", "z", "u"]);
    }

    #[test]
    fn test_unknown_lang() {
        // None = no MeCab override; must still return empty vec for unknown lang
        let ipa = g2p("foo", "fr", None);
        assert!(ipa.is_empty());
    }
}
