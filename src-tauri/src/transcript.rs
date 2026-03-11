use chrono::Utc;
use regex::{Regex, RegexBuilder};
use tauri::AppHandle;

use crate::constants::{
    DEFAULT_CLEANUP_TERMS, LIVE_PREVIEW_MAX_WORDS, LIVE_PREVIEW_RESET_AFTER_DIVERGENCE,
};
use crate::state::{AppPhase, PreviewControl, PreviewStabilizer, SharedState};
use crate::storage::{append_live_preview_log, emit_snapshot};

pub(crate) fn default_cleanup_terms() -> Vec<String> {
    DEFAULT_CLEANUP_TERMS
        .iter()
        .map(|term| term.to_string())
        .collect()
}

pub(crate) fn normalize_cleanup_term(term: &str) -> Option<String> {
    let normalized = term
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub(crate) fn normalize_cleanup_terms(terms: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for term in terms {
        let Some(term) = normalize_cleanup_term(term) else {
            continue;
        };

        if normalized.iter().any(|existing| existing == &term) {
            continue;
        }

        normalized.push(term);
    }

    normalized
}

fn condense_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_live_preview_line(text: &str) -> String {
    let normalized = text
        .chars()
        .map(|character| match character {
            '_' | '▁' | 'Ġ' => ' ',
            _ => character,
        })
        .collect::<String>();

    condense_whitespace(&normalized)
}

fn cleanup_patterns_from_terms(terms: &[String]) -> Vec<Regex> {
    let mut ordered = normalize_cleanup_terms(terms);
    ordered.sort_by(|left, right| right.len().cmp(&left.len()));

    ordered
        .into_iter()
        .filter_map(|term| {
            let escaped = regex::escape(&term).replace("\\ ", r"\s+");
            let pattern =
                format!(r#"(?i)(^|[\s\(\[\{{"'“”‘’,.;:!?]+){escaped}([\s\)\]\}}"'“”‘’,.;:!?]+|$)"#);
            RegexBuilder::new(&pattern)
                .case_insensitive(true)
                .build()
                .ok()
        })
        .collect()
}

pub(crate) fn cleanup_transcript_text(
    text: &str,
    cleanup_enabled: bool,
    cleanup_terms: &[String],
) -> String {
    let mut cleaned = condense_whitespace(text);
    if cleaned.is_empty() || !cleanup_enabled {
        return cleaned;
    }

    for pattern in cleanup_patterns_from_terms(cleanup_terms) {
        cleaned = pattern.replace_all(&cleaned, " ").into_owned();
    }

    let cleaned = condense_whitespace(&cleaned);
    let Ok(punctuation_spacing) = Regex::new(r#"\s+([,.;:!?])"#) else {
        return cleaned.trim().to_string();
    };
    let cleaned = punctuation_spacing.replace_all(&cleaned, "$1").into_owned();
    let Ok(repeated_commas) = Regex::new(r#"(,\s*){2,}"#) else {
        return cleaned.trim().to_string();
    };
    let cleaned = repeated_commas.replace_all(&cleaned, ", ").into_owned();

    cleaned
        .trim_matches(|character: char| {
            character.is_whitespace() || [',', ';', ':'].contains(&character)
        })
        .trim()
        .to_string()
}

pub(crate) fn live_preview_text(
    text: &str,
    cleanup_enabled: bool,
    cleanup_terms: &[String],
) -> String {
    let mut lines = text
        .lines()
        .map(normalize_live_preview_line)
        .map(|line| cleanup_transcript_text(&line, cleanup_enabled, cleanup_terms))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.is_empty() {
        let cleaned = cleanup_transcript_text(
            &normalize_live_preview_line(text),
            cleanup_enabled,
            cleanup_terms,
        );
        if cleaned.is_empty() {
            return String::new();
        }

        return trim_preview_line(&cleaned, LIVE_PREVIEW_MAX_WORDS);
    }

    if lines.len() > 3 {
        lines = lines.split_off(lines.len().saturating_sub(3));
    }

    let mut total_words = lines
        .iter()
        .map(|line| line.split_whitespace().count())
        .sum::<usize>();

    while total_words > LIVE_PREVIEW_MAX_WORDS && !lines.is_empty() {
        let first_word_count = lines[0].split_whitespace().count();
        if lines.len() == 1 {
            lines[0] = trim_preview_line(&lines[0], LIVE_PREVIEW_MAX_WORDS);
            break;
        }
        total_words = total_words.saturating_sub(first_word_count);
        lines.remove(0);
    }

    if lines.is_empty() {
        return String::new();
    }

    if let Some(last_line) = lines.last_mut() {
        *last_line = trim_preview_line(last_line, LIVE_PREVIEW_MAX_WORDS);
    }

    lines.join("\n")
}

fn trim_preview_line(text: &str, max_words: usize) -> String {
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.len() <= max_words {
        return text.to_string();
    }

    words[words.len().saturating_sub(max_words)..].join(" ")
}

pub(crate) fn note_preview_diagnostic(
    app: &AppHandle,
    shared: &SharedState,
    backend: &str,
    status: &str,
    detail: impl Into<String>,
) {
    let detail = detail.into();
    let event_line = if detail.is_empty() {
        status.to_string()
    } else {
        format!("{status}: {detail}")
    };
    let timestamp = Utc::now().format("%H:%M:%S").to_string();

    {
        let mut core = shared.lock();
        core.preview_diagnostics.backend = backend.to_string();
        core.preview_diagnostics.status = status.to_string();
        core.preview_diagnostics.detail = detail.clone();
        core.preview_diagnostics
            .recent_events
            .push(format!("{timestamp}  {event_line}"));
        if core.preview_diagnostics.recent_events.len() > 6 {
            let overflow = core.preview_diagnostics.recent_events.len() - 6;
            core.preview_diagnostics.recent_events.drain(..overflow);
        }
    }

    append_live_preview_log(
        app,
        &format!("{} [{}] {}", Utc::now().to_rfc3339(), backend, event_line),
    );
    emit_snapshot(app, shared);
}

pub(crate) fn transcription_cancelled(
    shared: Option<&SharedState>,
    preview_control: Option<(&PreviewControl, u64)>,
) -> bool {
    if let Some((preview_control, generation)) = preview_control {
        if preview_control.current_generation() != generation {
            return true;
        }
    }

    if let Some(shared) = shared {
        let core = shared.lock();
        matches!(core.phase, AppPhase::Idle | AppPhase::Error)
    } else {
        false
    }
}

fn preview_words(text: &str) -> Vec<String> {
    text.split_whitespace().map(ToString::to_string).collect()
}

fn normalize_preview_word(word: &str) -> String {
    word.trim_matches(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                ',' | '.'
                    | ';'
                    | ':'
                    | '!'
                    | '?'
                    | '"'
                    | '\''
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
            )
    })
    .to_lowercase()
}

pub(crate) fn common_prefix_len(left: &[String], right: &[String]) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(lhs, rhs)| {
            let lhs = normalize_preview_word(lhs);
            let rhs = normalize_preview_word(rhs);
            !lhs.is_empty() && lhs == rhs
        })
        .count()
}

fn render_preview_words(words: &[String]) -> String {
    if words.is_empty() {
        return String::new();
    }

    let start = words.len().saturating_sub(LIVE_PREVIEW_MAX_WORDS);
    words[start..].join(" ")
}

impl PreviewStabilizer {
    pub(crate) fn observe(&mut self, partial: &str) -> Option<String> {
        let current_words = preview_words(partial);
        if current_words.is_empty() {
            return None;
        }

        let previous_words = self.last_partial.clone();
        if let Some(previous_words) = previous_words.as_ref() {
            let stable_prefix_len = common_prefix_len(&self.stable_words, &current_words);
            if !self.stable_words.is_empty() && stable_prefix_len == 0 {
                self.divergence_count += 1;
                if self.divergence_count >= LIVE_PREVIEW_RESET_AFTER_DIVERGENCE {
                    self.stable_words.clear();
                    self.last_partial = Some(current_words);
                    self.divergence_count = 0;
                    return None;
                }
            } else {
                self.divergence_count = 0;
            }

            let shared_len = common_prefix_len(previous_words, &current_words);
            if shared_len > self.stable_words.len() {
                self.stable_words = current_words[..shared_len].to_vec();
            }
        }

        self.last_partial = Some(current_words);
        if self.stable_words.is_empty() {
            None
        } else {
            Some(render_preview_words(&self.stable_words))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cleanup_transcript_text, common_prefix_len, default_cleanup_terms, live_preview_text,
        normalize_cleanup_term, normalize_cleanup_terms,
    };

    #[test]
    fn cleanup_transcript_removes_common_fillers() {
        let cleaned = cleanup_transcript_text(
            "Um, I think uh this should work.",
            true,
            &default_cleanup_terms(),
        );

        assert_eq!(cleaned, "I think this should work.");
    }

    #[test]
    fn cleanup_transcript_removes_multi_word_fillers() {
        let cleaned = cleanup_transcript_text(
            "You know I think this is fine.",
            true,
            &[String::from("you know")],
        );

        assert_eq!(cleaned, "I think this is fine.");
    }

    #[test]
    fn normalize_cleanup_terms_deduplicates_and_trims() {
        let normalized = normalize_cleanup_terms(&[
            String::from(" um "),
            String::from("UM"),
            String::from("you   know"),
        ]);

        assert_eq!(
            normalized,
            vec![String::from("um"), String::from("you know")]
        );
        assert_eq!(normalize_cleanup_term("   "), None);
    }

    #[test]
    fn live_preview_text_keeps_recent_lines() {
        let preview = live_preview_text(
            "first line of text\nsecond line with more words\nthird line stays visible\nfourth line is newest",
            true,
            &default_cleanup_terms(),
        );

        assert_eq!(
            preview,
            "second line with more words\nthird line stays visible\nfourth line is newest"
        );
    }

    #[test]
    fn live_preview_text_normalizes_sentencepiece_style_underscores() {
        let preview = live_preview_text(
            "_this_is_a_test_and_we're_just_making_sure_streaming_works",
            true,
            &default_cleanup_terms(),
        );

        assert_eq!(
            preview,
            "this is a test and we're just making sure streaming works"
        );
    }

    #[test]
    fn live_preview_text_normalizes_sentencepiece_markers() {
        let preview = live_preview_text(
            "▁this▁is▁a▁live▁transcript▁and▁it▁wraps",
            true,
            &default_cleanup_terms(),
        );

        assert_eq!(preview, "this is a live transcript and it wraps");
    }

    #[test]
    fn common_prefix_len_ignores_case_and_terminal_punctuation() {
        let left = vec![
            String::from("Hello,"),
            String::from("world!"),
            String::from("Again"),
        ];
        let right = vec![
            String::from("hello"),
            String::from("world"),
            String::from("different"),
        ];

        assert_eq!(common_prefix_len(&left, &right), 2);
    }
}
