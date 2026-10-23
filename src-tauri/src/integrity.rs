//! Integrity (Phase 4): internal similarity, writing signals, rubric scoring.
//!
//! Two honesty constraints shape this module.
//!
//! Similarity uses word shingles rather than embeddings, so it works on any
//! machine — no model, no daemon, no network — and it detects the thing authors
//! actually hit: passages they wrote twice.
//!
//! The writing signals are **not** an AI detector. They are measurable
//! properties of prose (sentence-length variance, vocabulary richness, abstract
//! noun density). A real detector needs Binoculars or DeBERTa weights, which
//! this build does not ship. Everything here is named and reported as a signal
//! so the UI cannot accidentally present a heuristic as a verdict.

use crate::model::{RubricScore, SimilarityMatch, WritingSignals};
use serde::Serialize;
use std::collections::HashSet;

/// Words, lowercased, punctuation stripped. Shared by every measure so they all
/// agree on what a word is.
fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'').to_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Overlapping n-word sequences. `n` of 5 is long enough that shared phrasing
/// is meaningful and short enough to survive light editing.
pub fn shingles(text: &str, n: usize) -> HashSet<String> {
    let ws = words(text);
    if n == 0 || ws.len() < n {
        // Too short to shingle: treat the whole thing as one unit rather than
        // returning nothing, so short passages can still match each other.
        return if ws.is_empty() { HashSet::new() } else { HashSet::from([ws.join(" ")]) };
    }
    ws.windows(n).map(|w| w.join(" ")).collect()
}

/// Jaccard similarity of two shingle sets: 1.0 identical, 0.0 disjoint.
pub fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 { 0.0 } else { inter / union }
}

/// How much of `needle` appears in `haystack`. Asymmetric on purpose: a short
/// paragraph fully contained in a long chapter is 100% reused, even though
/// Jaccard would score it low.
pub fn containment(needle: &HashSet<String>, haystack: &HashSet<String>) -> f32 {
    if needle.is_empty() {
        return 0.0;
    }
    needle.intersection(haystack).count() as f32 / needle.len() as f32
}

/// Compares one document against others and reports passages reused above the
/// threshold. `sources` is (id, title, text).
pub fn internal_similarity(
    subject_id: &str,
    subject_text: &str,
    sources: &[(String, String, String)],
    threshold: f32,
    shingle_size: usize,
) -> Vec<SimilarityMatch> {
    let mut out = Vec::new();
    for paragraph in subject_text.split("\n\n") {
        let p = paragraph.trim();
        if words(p).len() < shingle_size {
            continue; // too short to say anything about
        }
        let p_sh = shingles(p, shingle_size);
        for (id, title, text) in sources {
            if id == subject_id {
                continue; // a document is never its own plagiarism
            }
            let score = containment(&p_sh, &shingles(text, shingle_size));
            if score >= threshold {
                out.push(SimilarityMatch {
                    source_id: id.clone(),
                    source_title: title.clone(),
                    excerpt: p.chars().take(240).collect(),
                    score,
                });
            }
        }
    }
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn sentences(text: &str) -> Vec<&str> {
    text.split(['.', '!', '?'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Abstractions that tend to replace concrete detail. A signal, never a verdict.
const ABSTRACT: &[&str] = &[
    "testament", "relentless", "profound", "myriad", "tapestry", "journey", "essence",
    "realm", "vast", "endless", "timeless", "unwavering", "beacon", "landscape",
];

/// Measurable properties of a passage. Nothing here decides anything.
pub fn writing_signals(text: &str) -> WritingSignals {
    let ws = words(text);
    let sents = sentences(text);
    let word_count = ws.len();
    if word_count == 0 {
        return WritingSignals::default();
    }

    let unique: HashSet<&String> = ws.iter().collect();
    let type_token = unique.len() as f32 / word_count as f32;

    let lengths: Vec<f32> = sents.iter().map(|s| words(s).len() as f32).collect();
    let mean = if lengths.is_empty() { 0.0 } else { lengths.iter().sum::<f32>() / lengths.len() as f32 };
    let variance = if lengths.len() < 2 {
        0.0
    } else {
        lengths.iter().map(|l| (l - mean).powi(2)).sum::<f32>() / lengths.len() as f32
    };

    let abstract_hits = ws.iter().filter(|w| ABSTRACT.contains(&w.as_str())).count();

    WritingSignals {
        word_count: word_count as i64,
        sentence_count: sents.len() as i64,
        mean_sentence_length: mean,
        sentence_length_variance: variance,
        type_token_ratio: type_token,
        abstract_density: abstract_hits as f32 / word_count as f32,
    }
}

/// Rubric criteria, each a number the author can check against the text.
pub fn rubric(text: &str) -> Vec<RubricScore> {
    let s = writing_signals(text);
    let mut out = Vec::new();

    let rhythm = if s.sentence_count < 2 {
        50.0
    } else {
        (s.sentence_length_variance / 40.0 * 100.0).min(100.0)
    };
    out.push(RubricScore {
        criterion: "Sentence rhythm".into(),
        score: rhythm.round() as i64,
        note: if rhythm < 35.0 {
            "Sentences are close to the same length throughout, which reads flat aloud.".into()
        } else {
            "Sentence lengths vary.".into()
        },
    });

    let richness = (s.type_token_ratio * 160.0).min(100.0);
    out.push(RubricScore {
        criterion: "Vocabulary range".into(),
        score: richness.round() as i64,
        note: format!("{:.0}% of words are distinct.", s.type_token_ratio * 100.0),
    });

    let concrete = (100.0 - s.abstract_density * 3000.0).clamp(0.0, 100.0);
    out.push(RubricScore {
        criterion: "Concrete detail".into(),
        score: concrete.round() as i64,
        note: if concrete < 60.0 {
            "Several abstractions stand in for things a reader could picture.".into()
        } else {
            "Mostly concrete.".into()
        },
    });

    let dialogue = text.matches('"').count() + text.matches('\u{201C}').count();
    out.push(RubricScore {
        criterion: "Dialogue presence".into(),
        score: if dialogue > 0 { 100 } else { 0 },
        note: if dialogue > 0 { "Contains dialogue.".into() } else { "No dialogue in this passage.".into() },
    });

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const REUSED: &str = "The rim of the basin held the last of the heat the way a kiln holds it, grudgingly and all at once.";

    #[test]
    fn identical_text_is_fully_similar_and_unrelated_text_is_not() {
        let a = shingles(REUSED, 5);
        assert!((jaccard(&a, &a) - 1.0).abs() < 1e-6);
        let b = shingles("Quarterly amortisation of intangible assets across the reporting period.", 5);
        assert_eq!(jaccard(&a, &b), 0.0);
    }

    #[test]
    fn punctuation_and_case_do_not_change_a_match() {
        let a = shingles("The well at Kharsa was going dry.", 4);
        let b = shingles("the WELL at kharsa was going dry", 4);
        assert!((jaccard(&a, &b) - 1.0).abs() < 1e-6, "matching must survive case and punctuation");
    }

    #[test]
    fn containment_is_asymmetric_so_a_quote_inside_a_chapter_scores_high() {
        let quote = shingles(REUSED, 5);
        let chapter = shingles(&format!("Some earlier prose. {REUSED} And a great deal of text afterwards that goes on."), 5);
        assert!(containment(&quote, &chapter) > 0.95, "the quote is almost entirely inside the chapter");
        assert!(containment(&chapter, &quote) < 0.6, "the chapter is not inside the quote");
        assert!(jaccard(&quote, &chapter) < 0.6, "jaccard alone would understate the reuse");
    }

    #[test]
    fn short_passages_still_compare_instead_of_vanishing() {
        let a = shingles("too short", 5);
        let b = shingles("too short", 5);
        assert_eq!(a.len(), 1, "a passage shorter than n becomes one unit");
        assert!((jaccard(&a, &b) - 1.0).abs() < 1e-6);
        assert!(shingles("", 5).is_empty());
    }

    #[test]
    fn internal_similarity_finds_a_reused_paragraph_and_names_its_source() {
        let subject = format!("An original opening paragraph with its own words.\n\n{REUSED}");
        let sources = vec![
            ("ch1".to_string(), "1. The Well".to_string(), format!("Earlier text. {REUSED}")),
            ("ch2".to_string(), "2. Salt".to_string(), "Entirely different prose about tariffs.".to_string()),
        ];
        let hits = internal_similarity("ch9", &subject, &sources, 0.6, 5);
        assert_eq!(hits.len(), 1, "only the reused paragraph should match");
        assert_eq!(hits[0].source_id, "ch1");
        assert_eq!(hits[0].source_title, "1. The Well");
        assert!(hits[0].score > 0.9);
        assert!(hits[0].excerpt.contains("kiln"));
    }

    #[test]
    fn a_document_is_never_its_own_plagiarism() {
        let sources = vec![("ch1".to_string(), "One".to_string(), REUSED.to_string())];
        assert!(internal_similarity("ch1", REUSED, &sources, 0.5, 5).is_empty());
    }

    #[test]
    fn the_threshold_actually_filters() {
        let sources = vec![("ch1".to_string(), "One".to_string(), format!("prefix {REUSED}"))];
        assert!(!internal_similarity("x", REUSED, &sources, 0.5, 5).is_empty());
        assert!(internal_similarity("x", REUSED, &sources, 1.01, 5).is_empty(), "an impossible threshold matches nothing");
    }

    #[test]
    fn signals_separate_flat_prose_from_varied_prose() {
        let flat = "The man walked. The man stopped. The man turned. The man left.";
        let varied = "He ran. In the long slow hours after the crossing closed, when the salt had gone grey and \
                      the wind had finally dropped, he sat down and counted what he had left. Twice.";

        let f = writing_signals(flat);
        let v = writing_signals(varied);
        assert!(v.sentence_length_variance > f.sentence_length_variance, "varied prose must show more variance");
        assert!(v.type_token_ratio > f.type_token_ratio, "repeated words lower the type-token ratio");
        assert_eq!(f.sentence_count, 4);
    }

    #[test]
    fn signals_are_safe_on_empty_and_single_sentence_input() {
        let empty = writing_signals("");
        assert_eq!(empty.word_count, 0);
        assert_eq!(empty.sentence_length_variance, 0.0, "no NaN from an empty passage");

        let one = writing_signals("Only one sentence here.");
        assert_eq!(one.sentence_count, 1);
        assert_eq!(one.sentence_length_variance, 0.0, "variance of one sample is zero, not NaN");
        assert!(one.type_token_ratio > 0.0);
    }

    #[test]
    fn abstract_prose_scores_lower_on_concrete_detail() {
        let abstractish = "The vast endless landscape was a testament to the relentless timeless journey of the realm.";
        let concrete = "The well held four feet of water. He lowered a knotted cord and wrote the number down.";

        let score = |t: &str| rubric(t).into_iter().find(|r| r.criterion == "Concrete detail").unwrap().score;
        assert!(score(abstractish) < score(concrete), "abstraction must cost concrete-detail points");
    }

    #[test]
    fn the_rubric_always_returns_every_criterion_even_for_empty_text() {
        let r = rubric("");
        assert_eq!(r.len(), 4);
        assert!(r.iter().all(|c| (0..=100).contains(&c.score)), "scores stay in range");
        assert!(r.iter().all(|c| !c.note.is_empty()), "every score explains itself");
    }

    #[test]
    fn dialogue_is_detected_with_straight_and_curly_quotes() {
        let straight = rubric("He said \"we cross at moonrise\" and meant it.");
        let curly = rubric("He said \u{201C}we cross at moonrise\u{201D} and meant it.");
        let get = |v: &[RubricScore]| v.iter().find(|r| r.criterion == "Dialogue presence").unwrap().score;
        assert_eq!(get(&straight), 100);
        assert_eq!(get(&curly), 100);
        assert_eq!(get(&rubric("No one spoke for a long while.")), 0);
    }
}

// ---------------------------------------------------------------------------
// The integrity packet (Phase 4)
// ---------------------------------------------------------------------------

/// Everything an integrity check found about one document, in a form an author
/// can hand to an editor, a supervisor or a publisher.
///
/// The point of a packet is that the numbers travel with the method that made
/// them. A bare "12% similar" is unfalsifiable; a packet says what was compared,
/// how, against what, and what the app explicitly did *not* determine.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityPacket {
    pub generated_at: i64,
    pub project: String,
    pub document_id: String,
    pub document_title: String,
    pub word_count: i64,
    /// Passages that also appear elsewhere in this project.
    pub internal_matches: Vec<SimilarityMatch>,
    pub threshold: f32,
    pub signals: WritingSignals,
    pub rubric: Vec<RubricScore>,
    /// Every web check made in this project, with what was sent. Empty when
    /// none were made — which is itself worth stating in a packet.
    pub web_checks: Vec<crate::websearch::TraceEntry>,
    /// What the numbers above are, and what they are not.
    pub method: Vec<String>,
}

pub fn method_notes() -> Vec<String> {
    vec![
        "Internal similarity is word-shingle containment: the share of this passage's three-word runs that also appear in another document in the same project. No model is involved.".into(),
        "The threshold is the author's, not a standard. Passages below it are not listed.".into(),
        "Writing signals are measurements of the prose — sentence length and its variance, vocabulary range, abstraction. They describe the text.".into(),
        "This packet does not determine who or what wrote the text. AegisDraft ships no AI-writing detector and makes no claim about authorship. Any tool that offers one from these numbers alone is guessing.".into(),
        "Web checks list every phrase that was sent off this machine, through the author's own wigolo daemon, with the sources that came back. A phrase not listed here was never sent.".into(),
    ]
}

/// The packet as an HTML page, for the reader who is not going to open JSON.
///
/// Deliberately plain: no stylesheet a publisher's mail client will strip, no
/// chart that would imply a precision these numbers do not have.
pub fn packet_html(packet: &IntegrityPacket) -> String {
    fn esc(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    let mut out = String::new();
    out.push_str("<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\n");
    out.push_str(&format!(
        "<title>Integrity report — {}</title>\n",
        esc(&packet.document_title)
    ));
    out.push_str(
        "<style>body{font:15px/1.6 Georgia,serif;max-width:46em;margin:3em auto;padding:0 1em;color:#1c2b33}\
         h1{font-size:1.5em;margin:0 0 .2em}h2{font-size:1.05em;margin:2em 0 .5em;letter-spacing:.02em;text-transform:uppercase;color:#5b6c76}\
         table{border-collapse:collapse;width:100%}td,th{text-align:left;padding:.4em .6em;border-bottom:1px solid #e4e9ec;vertical-align:top}\
         .n{font-variant-numeric:tabular-nums;white-space:nowrap}blockquote{margin:.4em 0;color:#41525e}\
         ol{padding-left:1.2em}li{margin:.4em 0}.q{color:#5b6c76;font-size:.92em}</style></head><body>\n",
    );
    out.push_str(&format!("<h1>{}</h1>\n", esc(&packet.document_title)));
    out.push_str(&format!(
        "<p class=\"q\">{} · {} words · report generated from the project at {}</p>\n",
        esc(&packet.project),
        packet.word_count,
        esc(&packet.project)
    ));

    out.push_str("<h2>Reused from elsewhere in this project</h2>\n");
    if packet.internal_matches.is_empty() {
        out.push_str(&format!(
            "<p>Nothing above {:.0}% overlap.</p>\n",
            packet.threshold * 100.0
        ));
    } else {
        out.push_str("<table><tr><th>Source</th><th class=\"n\">Overlap</th></tr>\n");
        for m in &packet.internal_matches {
            out.push_str(&format!(
                "<tr><td>{}<blockquote>{}</blockquote></td><td class=\"n\">{:.0}%</td></tr>\n",
                esc(&m.source_title),
                esc(&m.excerpt),
                m.score * 100.0
            ));
        }
        out.push_str("</table>\n");
    }

    out.push_str("<h2>Measurements</h2>\n<table>\n");
    let s = &packet.signals;
    for (name, value) in [
        ("Sentences", format!("{}", s.sentence_count)),
        ("Mean sentence length", format!("{:.1} words", s.mean_sentence_length)),
        ("Sentence length variance", format!("{:.1}", s.sentence_length_variance)),
        ("Type/token ratio", format!("{:.3}", s.type_token_ratio)),
        ("Abstract word density", format!("{:.1}%", s.abstract_density * 100.0)),
    ] {
        out.push_str(&format!("<tr><td>{name}</td><td class=\"n\">{value}</td></tr>\n"));
    }
    out.push_str("</table>\n");

    out.push_str("<h2>Rubric</h2>\n<table>\n");
    for r in &packet.rubric {
        out.push_str(&format!(
            "<tr><td>{}<br><span class=\"q\">{}</span></td><td class=\"n\">{}</td></tr>\n",
            esc(&r.criterion),
            esc(&r.note),
            r.score
        ));
    }
    out.push_str("</table>\n");

    out.push_str("<h2>Phrases sent off this machine</h2>\n");
    if packet.web_checks.is_empty() {
        out.push_str("<p>None. No web check has been run in this project.</p>\n");
    } else {
        out.push_str("<table><tr><th>Phrase</th><th class=\"n\">Sources</th></tr>\n");
        for entry in &packet.web_checks {
            out.push_str(&format!(
                "<tr><td>{}</td><td class=\"n\">{}</td></tr>\n",
                esc(&entry.query),
                entry.result_count
            ));
        }
        out.push_str("</table>\n");
    }

    out.push_str("<h2>Method</h2>\n<ol>\n");
    for note in &packet.method {
        out.push_str(&format!("<li>{}</li>\n", esc(note)));
    }
    out.push_str("</ol>\n</body></html>\n");
    out
}
