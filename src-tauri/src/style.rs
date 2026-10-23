//! A writing style profile — what the author's prose actually measures, so
//! "rewrite this in my voice" means something.
//!
//! Before this, `assist(mode = "rewrite")` told the model to write "in the
//! author's own voice" and gave it nothing to go on but the passage in front of
//! it. That is a label, not an instruction: the model fell back on its own
//! defaults, which is precisely the flattening authors complain about.
//!
//! The profile is built from prose the author has marked **Done**, because that
//! is the only signal in the binder that says *this is how I want it to read*.
//! With nothing marked Done it falls back to every scene with prose in it and
//! records that it did, rather than quietly claiming a draft is a standard.
//!
//! Two halves, kept apart on purpose:
//!
//! - **Measured.** Sentence length and its variance, vocabulary range, dialogue
//!   density, abstraction. Arithmetic over the author's own files; no model is
//!   involved and none is needed.
//! - **Described.** One short paragraph a local model writes about the voice.
//!   Optional, clearly labelled as the model's reading, and the profile is
//!   useful without it.
//!
//! Stored as `style-profile.json` at the project root, so it lands in LibGit2
//! snapshots with the prose and an author can read it in a text editor.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{err, AppResult};
use crate::model::{DocKind, WritingSignals};
use crate::{integrity, ollama, project};

pub const FILE: &str = "style-profile.json";

/// Words per window for the vocabulary measure. Type/token ratio falls as text
/// gets longer, so comparing a 400-word scene against a 40,000-word profile
/// would measure length, not vocabulary. Averaging the ratio over fixed windows
/// makes the two numbers comparable, which is the only way the comparison means
/// anything — and it follows that text shorter than one window cannot be
/// measured on this scale at all, so it is not measured, rather than measured
/// differently and compared anyway.
const VOCAB_WINDOW: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleProfile {
    pub built_at: i64,
    /// How many documents went into it, and how many words.
    pub sample_documents: i64,
    pub sample_words: i64,
    /// True when the sample was prose marked Done. False means the author had
    /// marked nothing, so this is built from drafts — worth saying out loud.
    pub from_finished_work: bool,
    pub signals: WritingSignals,
    /// Length-independent vocabulary range; see `VOCAB_WINDOW`.
    pub vocabulary_range: f32,
    /// Share of paragraphs that are dialogue.
    pub dialogue_density: f32,
    /// The model's reading of the voice, when a model was used. Never required.
    pub voice: Option<String>,
    /// The model that wrote `voice`, so the note can be attributed.
    pub voice_model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleNote {
    pub dimension: String,
    /// The open document's measurement.
    pub yours: f32,
    /// The profile's.
    pub profile: f32,
    /// 0–100, where 100 is indistinguishable from the profile.
    pub closeness: i64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleDistance {
    /// The mean of the dimensions below. A summary, not a verdict.
    pub overall: i64,
    pub notes: Vec<StyleNote>,
    /// True when the text was too short to measure honestly.
    pub too_short: bool,
}

// ---------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------

fn word_list(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Type/token ratio averaged over fixed-length windows, so the number does not
/// simply fall as the text gets longer.
/// `None` below one window. A whole-text ratio on 90 words would be a different
/// measurement wearing the same name, and comparing it to a windowed one would
/// report a style difference where there is only a length difference.
pub fn vocabulary_range(text: &str) -> Option<f32> {
    let words = word_list(text);
    if words.len() < VOCAB_WINDOW {
        return None;
    }
    let mut total = 0.0;
    let mut windows = 0;
    for chunk in words.chunks(VOCAB_WINDOW) {
        if chunk.len() < VOCAB_WINDOW {
            break; // A short tail would score high and skew the average.
        }
        let unique: HashSet<&String> = chunk.iter().collect();
        total += unique.len() as f32 / chunk.len() as f32;
        windows += 1;
    }
    if windows == 0 {
        None
    } else {
        Some(total / windows as f32)
    }
}

/// The share of paragraphs that open as dialogue. Paragraph-level rather than
/// character-level: a scene that is half dialogue reads differently from one
/// with a single quoted phrase, and counting quote marks cannot tell them apart.
pub fn dialogue_density(text: &str) -> f32 {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if paragraphs.is_empty() {
        return 0.0;
    }
    let spoken = paragraphs
        .iter()
        .filter(|p| p.starts_with('"') || p.starts_with('\u{201c}') || p.starts_with('\u{2014}'))
        .count();
    spoken as f32 / paragraphs.len() as f32
}

// ---------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------

/// The prose a profile should be built from, and whether it is finished work.
pub fn sample(root: &Path) -> AppResult<(String, i64, bool)> {
    let docs = project::documents(root)?;
    let prose: Vec<_> = docs
        .iter()
        .filter(|d| d.kind != DocKind::Folder && d.path.is_some() && d.word_count > 0)
        .collect();

    let finished: Vec<_> = prose
        .iter()
        .filter(|d| d.status.as_deref() == Some("Done"))
        .collect();
    let from_finished = !finished.is_empty();
    let chosen: Vec<_> = if from_finished {
        finished.iter().map(|d| **d).collect()
    } else {
        prose.clone()
    };

    if chosen.is_empty() {
        return err("there is no prose in this project to build a style profile from");
    }

    let mut text = String::new();
    for doc in &chosen {
        if let Ok(body) = project::read_body(root, &doc.id) {
            text.push_str(body.trim());
            text.push_str("\n\n");
        }
    }
    Ok((text, chosen.len() as i64, from_finished))
}

/// Build a profile from the project's own prose. `model` is optional: without
/// one the measured half is built and `voice` stays `None`.
pub fn build(
    root: &Path,
    model: Option<&str>,
    allow_cloud: bool,
    now_ms: i64,
) -> AppResult<StyleProfile> {
    let (text, documents, from_finished_work) = sample(root)?;
    let signals = integrity::writing_signals(&text);
    if signals.word_count < 200 {
        return err(format!(
            "{} words is not enough prose to measure a style from — write more, or mark more of it Done",
            signals.word_count
        ));
    }

    let mut profile = StyleProfile {
        built_at: now_ms,
        sample_documents: documents,
        sample_words: signals.word_count,
        from_finished_work,
        signals,
        // `build` refuses under 200 words above, so at least one window fits.
        vocabulary_range: vocabulary_range(&text).unwrap_or_default(),
        dialogue_density: dialogue_density(&text),
        voice: None,
        voice_model: None,
    };

    if let Some(model) = model {
        // A failure here loses the description, not the profile: the measured
        // half is the part that has to be right, and it needs no model.
        let excerpt: String = text.chars().take(6000).collect();
        match ollama::generate_guarded(
            model,
            &format!(
                "Describe this author's prose voice in two or three sentences. \
                 Write about rhythm, diction and distance — what a careful editor \
                 would notice. Do not praise it, do not summarise the story, and \
                 do not quote it.\n\n{excerpt}"
            ),
            Some("You are a prose editor describing a voice precisely and without flattery. Return only the description."),
            240,
            0.4,
            allow_cloud,
        ) {
            Ok(text) if !text.trim().is_empty() => {
                profile.voice = Some(text.trim().to_string());
                profile.voice_model = Some(model.to_string());
            }
            _ => {}
        }
    }

    Ok(profile)
}

pub fn save(root: &Path, profile: &StyleProfile) -> AppResult<()> {
    fs::write(root.join(FILE), serde_json::to_string_pretty(profile)?)?;
    Ok(())
}

pub fn load(root: &Path) -> Option<StyleProfile> {
    let raw = fs::read_to_string(root.join(FILE)).ok()?;
    serde_json::from_str(&raw).ok()
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// How close two numbers are on a 0–100 scale, where `scale` is the difference
/// at which they stop resembling each other at all.
fn closeness(a: f32, b: f32, scale: f32) -> i64 {
    if scale <= 0.0 {
        return 100;
    }
    let gap = (a - b).abs() / scale;
    ((1.0 - gap).clamp(0.0, 1.0) * 100.0).round() as i64
}

pub fn distance(profile: &StyleProfile, text: &str) -> StyleDistance {
    let signals = integrity::writing_signals(text);
    if signals.word_count < 60 {
        return StyleDistance {
            overall: 0,
            notes: Vec::new(),
            too_short: true,
        };
    }

    let mine_dialogue = dialogue_density(text);

    let mut notes = vec![
        StyleNote {
            dimension: "Sentence length".into(),
            yours: signals.mean_sentence_length,
            profile: profile.signals.mean_sentence_length,
            closeness: closeness(
                signals.mean_sentence_length,
                profile.signals.mean_sentence_length,
                10.0,
            ),
            note: compare(
                signals.mean_sentence_length,
                profile.signals.mean_sentence_length,
                "Sentences here run longer than you usually write.",
                "Sentences here run shorter than you usually write.",
                "Sentence length matches your usual.",
                1.5,
            ),
        },
        StyleNote {
            dimension: "Rhythm".into(),
            yours: signals.sentence_length_variance,
            profile: profile.signals.sentence_length_variance,
            closeness: closeness(
                signals.sentence_length_variance,
                profile.signals.sentence_length_variance,
                60.0,
            ),
            note: compare(
                signals.sentence_length_variance,
                profile.signals.sentence_length_variance,
                "Sentence lengths vary more than usual — choppier.",
                "Sentence lengths are more even than usual — flatter.",
                "The rhythm matches your usual.",
                8.0,
            ),
        },
        StyleNote {
            dimension: "Dialogue".into(),
            yours: mine_dialogue,
            profile: profile.dialogue_density,
            closeness: closeness(mine_dialogue, profile.dialogue_density, 0.5),
            note: compare(
                mine_dialogue,
                profile.dialogue_density,
                "More dialogue than your usual balance.",
                "Less dialogue than your usual balance.",
                "The dialogue balance matches your usual.",
                0.1,
            ),
        },
        StyleNote {
            dimension: "Abstraction".into(),
            yours: signals.abstract_density,
            profile: profile.signals.abstract_density,
            closeness: closeness(
                signals.abstract_density,
                profile.signals.abstract_density,
                0.05,
            ),
            note: compare(
                signals.abstract_density,
                profile.signals.abstract_density,
                "More abstract words than you usually use.",
                "More concrete than usual.",
                "Abstraction matches your usual.",
                0.01,
            ),
        },
    ];
    // Vocabulary range only when the passage is long enough to measure it on
    // the same scale the profile used. A missing dimension is better than one
    // that reports length as if it were style.
    if let Some(mine) = vocabulary_range(text) {
        notes.push(StyleNote {
            dimension: "Vocabulary range".into(),
            yours: mine,
            profile: profile.vocabulary_range,
            closeness: closeness(mine, profile.vocabulary_range, 0.15),
            note: compare(
                mine,
                profile.vocabulary_range,
                "Wider vocabulary than you usually reach for.",
                "Narrower vocabulary — more repetition than usual.",
                "Vocabulary range matches your usual.",
                0.03,
            ),
        });
    }

    notes.sort_by_key(|n| n.closeness);

    let overall = if notes.is_empty() {
        0
    } else {
        notes.iter().map(|n| n.closeness).sum::<i64>() / notes.len() as i64
    };
    StyleDistance {
        overall,
        notes,
        too_short: false,
    }
}

fn compare(mine: f32, theirs: f32, higher: &str, lower: &str, same: &str, tol: f32) -> String {
    if (mine - theirs).abs() <= tol {
        same.to_string()
    } else if mine > theirs {
        higher.to_string()
    } else {
        lower.to_string()
    }
}

// ---------------------------------------------------------------------------
// Injection
// ---------------------------------------------------------------------------

/// The profile as instructions a model can act on.
///
/// Plain language rather than raw numbers: "averages about 14 words" steers a
/// model; `meanSentenceLength: 13.8` does not, and inviting a model to hit a
/// decimal makes the prose worse.
pub fn prompt_fragment(profile: &StyleProfile) -> String {
    let mean = profile.signals.mean_sentence_length;
    let rhythm = if profile.signals.sentence_length_variance > 60.0 {
        "lengths vary sharply — long sentences broken by very short ones"
    } else if profile.signals.sentence_length_variance > 20.0 {
        "lengths vary moderately"
    } else {
        "lengths stay fairly even"
    };
    let vocab = if profile.vocabulary_range > 0.62 {
        "wide, with little repetition"
    } else if profile.vocabulary_range > 0.48 {
        "moderate"
    } else {
        "deliberately narrow, with words repeated on purpose"
    };
    let dialogue = (profile.dialogue_density * 100.0).round() as i64;

    let mut out = String::from("The author's own measured voice:\n");
    out.push_str(&format!("- Sentences average about {mean:.0} words, and {rhythm}.\n"));
    out.push_str(&format!("- Vocabulary range is {vocab}.\n"));
    out.push_str(&format!(
        "- About {dialogue}% of paragraphs are dialogue.\n"
    ));
    if let Some(voice) = &profile.voice {
        out.push_str(&format!("- Described voice: {voice}\n"));
    }
    out.push_str(
        "Match this. Do not smooth it out, do not raise the register, and do not \
         even out the sentence lengths.",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TERSE: &str = "The well was dry. She knew it before the bucket came up. \
        Salt on the rim. Salt in the rope. She went back up the road without a word, \
        and the dust took her tracks before she reached the gate. Nothing moved. \
        The pumps had stopped in the night and no one had said so.";

    const ORNATE: &str = "It had become apparent to her, in the manner of realisations \
        which arrive not as revelations but as slow accumulations of circumstance, that \
        the well upon which the entire settlement had for three generations depended was \
        no longer capable of yielding water in any quantity sufficient to sustain the \
        population through what promised to be an exceptionally protracted summer season.";

    #[test]
    fn vocabulary_range_does_not_simply_fall_with_length() {
        // The same prose repeated is not a narrower vocabulary per window, only
        // a longer text — a raw type/token ratio would say otherwise.
        let once = vocabulary_range(&TERSE.repeat(4)).unwrap();
        let many = vocabulary_range(&TERSE.repeat(40)).unwrap();
        assert!(
            (once - many).abs() < 0.08,
            "windowed vocabulary must be length-independent: {once} vs {many}"
        );
        // Below one window it is not measured at all, rather than measured on a
        // different scale and compared as if it were the same one.
        assert_eq!(vocabulary_range("Only a handful of words here."), None);
    }

    #[test]
    fn dialogue_is_counted_by_paragraph_not_by_quote_mark() {
        let mostly_spoken = "\"Where is it.\"\n\n\"Gone.\"\n\nShe did not answer.";
        let one_quoted_phrase =
            "She said the word \"gone\" the way other people said a date.\n\nNothing followed.";
        assert!(dialogue_density(mostly_spoken) > 0.6);
        assert_eq!(
            dialogue_density(one_quoted_phrase),
            0.0,
            "a quoted phrase inside prose is not dialogue"
        );
    }

    fn profile_from(text: &str) -> StyleProfile {
        let signals = integrity::writing_signals(text);
        StyleProfile {
            built_at: 0,
            sample_documents: 1,
            sample_words: signals.word_count,
            from_finished_work: true,
            signals,
            vocabulary_range: vocabulary_range(text).unwrap_or_default(),
            dialogue_density: dialogue_density(text),
            voice: None,
            voice_model: None,
        }
    }

    #[test]
    fn a_passage_in_the_authors_voice_scores_above_one_that_is_not() {
        let profile = profile_from(&TERSE.repeat(6));
        let near = distance(&profile, &TERSE.repeat(2));
        let far = distance(&profile, &ORNATE.repeat(3));

        assert!(!near.too_short && !far.too_short);
        assert!(
            near.overall > far.overall + 20,
            "the author's own prose must score clearly closer: {} vs {}",
            near.overall,
            far.overall
        );
        assert!(
            near.overall > 80,
            "own prose scored only {} — {:?}",
            near.overall,
            near.notes.iter().map(|n| (&n.dimension, n.closeness, n.yours, n.profile)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_weakest_dimension_is_named_first() {
        let profile = profile_from(&TERSE.repeat(6));
        let far = distance(&profile, &ORNATE.repeat(3));
        assert_eq!(far.notes[0].dimension, "Sentence length");
        assert!(
            far.notes[0].note.contains("longer"),
            "the note must say which way it is off: {}",
            far.notes[0].note
        );
    }

    #[test]
    fn a_short_paragraph_is_refused_rather_than_scored() {
        let profile = profile_from(&TERSE.repeat(6));
        let d = distance(&profile, "Three words here.");
        assert!(d.too_short);
        assert!(d.notes.is_empty(), "a refusal must not carry fake numbers");
    }

    #[test]
    fn the_prompt_fragment_reads_as_instructions_not_as_a_data_dump() {
        let fragment = prompt_fragment(&profile_from(&TERSE.repeat(6)));
        assert!(fragment.contains("Sentences average about"));
        assert!(fragment.contains("Do not smooth it out"));
        assert!(
            !fragment.contains("meanSentenceLength") && !fragment.contains('{'),
            "no raw field names belong in a prompt: {fragment}"
        );
    }

    #[test]
    fn a_profile_round_trips_through_the_file_it_is_stored_in() {
        let dir = std::env::temp_dir().join(format!("aegis-style-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let profile = profile_from(&TERSE.repeat(6));
        save(&dir, &profile).unwrap();

        let back = load(&dir).expect("the profile must read back");
        assert_eq!(back.sample_words, profile.sample_words);
        assert!((back.vocabulary_range - profile.vocabulary_range).abs() < 1e-5);
        assert!(dir.join(FILE).exists(), "it is a real file the author can read");

        assert!(load(&dir.join("nowhere")).is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_project_with_nothing_marked_done_says_the_profile_came_from_drafts() {
        let base = std::env::temp_dir().join(format!("aegis-style-sample-{}", std::process::id()));
        fs::remove_dir_all(&base).ok();
        fs::create_dir_all(&base).unwrap();
        let summary = project::create(&base, "Voice", "novel").unwrap();
        let root = Path::new(&summary.path);

        let one = project::create_document(root, None, "One", DocKind::Scene).unwrap();
        project::write_body(root, &one.id, &TERSE.repeat(3)).unwrap();
        let two = project::create_document(root, None, "Two", DocKind::Scene).unwrap();
        project::write_body(root, &two.id, &ORNATE.repeat(2)).unwrap();

        let (_, documents, finished) = sample(root).unwrap();
        assert!(!finished, "nothing is marked Done");
        assert_eq!(documents, 2);

        // Marking one Done narrows the sample to it.
        project::update_meta(root, &one.id, None, None, Some("Done")).unwrap();
        let (text, documents, finished) = sample(root).unwrap();
        assert!(finished);
        assert_eq!(documents, 1);
        assert!(text.contains("Salt on the rim"));
        assert!(
            !text.contains("slow accumulations"),
            "a draft must not leak into a profile built from finished work"
        );

        fs::remove_dir_all(&base).ok();
    }
}
