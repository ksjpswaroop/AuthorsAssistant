//! Deep Continuity: the tools that catch a book contradicting itself.
//!
//! Four capabilities named in [AGENTS.md](../../docs/AGENTS.md) that had no
//! implementation — the Continuity profile was approximating them with
//! `read_document` and `search_binder`, which finds *similar* passages but
//! cannot find *contradictory facts*.
//!
//! # The division of labour
//!
//! Everything cheap and certain is arithmetic; only the genuinely ambiguous
//! reaches a model.
//!
//! | Check | How |
//! | :--- | :--- |
//! | Entities | Matched against the author's own character and location sheets first, then a model for what is left |
//! | Canon | Every canon entry is a falsifiable statement with a source; a scene is checked against them |
//! | Foreshadowing | Plants without payoffs. Pure arithmetic over fields the inspector already has |
//! | Propagation debt | Recorded, never guessed at |
//!
//! # Why debt is recorded rather than fixed
//!
//! When a scene reveals a hole in the world — a fact the book needs but has
//! never established — the tempting thing is to invent one. That is how a model
//! quietly becomes the author of the world's rules. `log_propagation_debt`
//! writes the gap down instead, with the scene that exposed it, and leaves the
//! decision to the person whose book it is.

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{err, AppResult};
use crate::model::{DocKind, DocumentRow};

/// Shingle overlap above which a canon fact and a passage are worth comparing.
/// A recall filter only — the model, or the author, decides if they conflict.
pub const CANON_CANDIDATE: f32 = 0.10;

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub name: String,
    /// `character`, `location`, `date`, or `unknown`.
    pub kind: String,
    /// The binder document this is, when it is one the author has already made.
    pub document_id: Option<String>,
    /// How many times it appears in the passage.
    pub mentions: i64,
}

/// Entities in a passage, matched against what the project already knows.
///
/// Sheets first, deliberately. A character the author has written a sheet for is
/// a fact about the project; a name a model picked out of a sentence is a guess.
/// Matching the certain ones first means the guesses are clearly the leftovers.
pub fn extract_entities(passage: &str, docs: &[DocumentRow]) -> Vec<Entity> {
    let mut out: Vec<Entity> = Vec::new();
    let lower = passage.to_lowercase();

    for doc in docs {
        let kind = match doc.kind {
            DocKind::Sheet => "character",
            DocKind::Location => "location",
            _ => continue,
        };
        // Match on the sheet's title and on its first word, so "Tamsin Reyes"
        // is found by "Tamsin". Short names are skipped: a two-letter title
        // matches half the prose in the book.
        let title = doc.title.trim();
        let first = title.split_whitespace().next().unwrap_or(title);
        let mentions = [title, first]
            .iter()
            .filter(|n| n.len() >= 3)
            .map(|n| count_word(&lower, &n.to_lowercase()))
            .max()
            .unwrap_or(0);
        if mentions > 0 {
            out.push(Entity {
                name: title.to_string(),
                kind: kind.to_string(),
                document_id: Some(doc.id.clone()),
                mentions,
            });
        }
    }

    // Story dates the author writes in notes and prose: "Day 12", "Year 3".
    for word in ["day", "year", "month", "week", "winter", "summer", "spring", "autumn"] {
        let n = count_word(&lower, word);
        if n > 0 {
            out.push(Entity {
                name: word.to_string(),
                kind: "date".into(),
                document_id: None,
                mentions: n,
            });
        }
    }

    out.sort_by(|a, b| b.mentions.cmp(&a.mentions).then(a.name.cmp(&b.name)));
    out
}

/// Whole-word occurrences, so "Sal" does not match "salt".
fn count_word(haystack_lower: &str, needle_lower: &str) -> i64 {
    if needle_lower.is_empty() {
        return 0;
    }
    let mut count = 0;
    let bytes = haystack_lower.as_bytes();
    let mut from = 0usize;
    while let Some(offset) = haystack_lower[from..].find(needle_lower) {
        let at = from + offset;
        let end = at + needle_lower.len();
        let before_ok = at == 0 || !(bytes[at - 1] as char).is_alphanumeric();
        let after_ok = end >= bytes.len() || !(bytes[end] as char).is_alphanumeric();
        if before_ok && after_ok {
            count += 1;
        }
        from = end;
    }
    count
}

// ---------------------------------------------------------------------------
// Canon
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonEntry {
    pub fact: String,
    /// The document that establishes it. A canon entry without one is refused
    /// at write time, so this is always present.
    pub source: String,
    /// The canon document it lives in.
    pub document_id: String,
}

/// Parse the canon documents. The format is what `append_canon_entry` writes:
/// `- <fact>` followed by an indented `*established in:* <source>`.
pub fn read_canon(root: &std::path::Path, docs: &[DocumentRow]) -> AppResult<Vec<CanonEntry>> {
    let mut out = Vec::new();
    for doc in docs.iter().filter(|d| d.kind == DocKind::Canon) {
        let Ok(body) = crate::project::read_body(root, &doc.id) else { continue };
        let mut pending: Option<String> = None;
        for line in body.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("- ") {
                // A bullet with no source yet is kept until we see one or the
                // next bullet replaces it.
                pending = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("*established in:*") {
                if let Some(fact) = pending.take() {
                    out.push(CanonEntry {
                        fact,
                        source: rest.trim().to_string(),
                        document_id: doc.id.clone(),
                    });
                }
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonCandidate {
    pub fact: String,
    pub source: String,
    /// The paragraph in the passage that touches the same subject.
    pub passage_excerpt: String,
    pub overlap: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonCheck {
    pub candidates: Vec<CanonCandidate>,
    pub canon_entries: usize,
    /// What this examined and what it could not, so the scope is on the report.
    pub method: String,
}

/// Which established facts this passage touches.
///
/// Retrieval, not judgement. It answers "which of the book's rules is this
/// paragraph near", and hands those to a model or a person to decide whether
/// they conflict. Deciding here with word overlap alone would produce confident
/// nonsense — "his left leg" and "his right leg" overlap almost perfectly.
pub fn check_against_canon(passage: &str, canon: &[CanonEntry]) -> CanonCheck {
    let paragraphs: Vec<&str> = passage
        .split("\n\n")
        .map(str::trim)
        .filter(|p| p.split_whitespace().count() >= 4)
        .collect();

    let mut candidates = Vec::new();
    for entry in canon {
        let fact = crate::integrity::shingles(&entry.fact, 2);
        let mut best: Option<(f32, &str)> = None;
        for p in &paragraphs {
            let overlap = crate::integrity::containment(&fact, &crate::integrity::shingles(p, 2));
            if overlap >= CANON_CANDIDATE && best.map(|(b, _)| overlap > b).unwrap_or(true) {
                best = Some((overlap, p));
            }
        }
        if let Some((overlap, p)) = best {
            candidates.push(CanonCandidate {
                fact: entry.fact.clone(),
                source: entry.source.clone(),
                passage_excerpt: p.chars().take(240).collect(),
                overlap,
            });
        }
    }
    candidates.sort_by(|a, b| b.overlap.total_cmp(&a.overlap));

    CanonCheck {
        canon_entries: canon.len(),
        method: format!(
            "{} established facts were compared against this passage by shared wording. \
             Facts that share no phrasing with it were not examined, and nothing here is a \
             verdict — a contradiction still needs reading.",
            canon.len()
        ),
        candidates,
    }
}

// ---------------------------------------------------------------------------
// Foreshadowing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: String,
    pub planted_in: Vec<(String, String)>,
    pub paid_off_in: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerReport {
    /// Planted and never paid off. The promise the book has not kept.
    pub unpaid: Vec<Thread>,
    /// Paid off with nothing planted. A payoff that lands from nowhere.
    pub unplanted: Vec<Thread>,
    pub complete: Vec<Thread>,
}

fn split_ids(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or_default()
        .split([',', ';', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Plants without payoffs, and payoffs without plants.
///
/// Pure arithmetic over the inspector's own fields — no model, no network, and
/// it works on a project that has never seen either.
pub fn check_foreshadowing_ledger(docs: &[DocumentRow]) -> LedgerReport {
    let mut planted: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut paid: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for doc in docs {
        for id in split_ids(doc.plants.as_deref()) {
            planted.entry(id).or_default().push((doc.id.clone(), doc.title.clone()));
        }
        for id in split_ids(doc.payoffs.as_deref()) {
            paid.entry(id).or_default().push((doc.id.clone(), doc.title.clone()));
        }
    }

    let all: HashSet<&String> = planted.keys().chain(paid.keys()).collect();
    let mut report = LedgerReport { unpaid: Vec::new(), unplanted: Vec::new(), complete: Vec::new() };

    let mut ids: Vec<&String> = all.into_iter().collect();
    ids.sort();
    for id in ids {
        let thread = Thread {
            id: id.clone(),
            planted_in: planted.get(id).cloned().unwrap_or_default(),
            paid_off_in: paid.get(id).cloned().unwrap_or_default(),
        };
        match (thread.planted_in.is_empty(), thread.paid_off_in.is_empty()) {
            (false, true) => report.unpaid.push(thread),
            (true, false) => report.unplanted.push(thread),
            _ => report.complete.push(thread),
        }
    }
    report
}

// ---------------------------------------------------------------------------
// Propagation debt
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Debt {
    pub id: String,
    /// What the book needs and has not established.
    pub gap: String,
    /// The document that exposed it.
    pub found_in: String,
    /// Who noticed — a person, or the model that was reading.
    pub found_by: String,
    pub at: i64,
    pub resolved_at: Option<i64>,
}

pub const DEBT_FILE: &str = "agent/debts.json";

/// Write down a hole in the world instead of inventing something to fill it.
///
/// This is the whole point of the tool. A model that answers "what colour are
/// the guild robes" by choosing a colour has just become the author of the
/// world's rules, and the author will find out three chapters later. Recording
/// the gap keeps the decision where it belongs.
pub fn log_propagation_debt(
    root: &std::path::Path,
    gap: &str,
    found_in: &str,
    found_by: &str,
    now: i64,
) -> AppResult<Debt> {
    let gap = gap.trim();
    if gap.split_whitespace().count() < 3 {
        return err("a propagation debt needs to say what is missing");
    }
    let mut all = debts(root);
    // The same gap noticed twice is one debt, not two.
    if let Some(existing) = all
        .iter()
        .find(|d| d.resolved_at.is_none() && d.gap.eq_ignore_ascii_case(gap))
    {
        return Ok(existing.clone());
    }
    let debt = Debt {
        id: format!("debt_{}", crate::project::new_id()),
        gap: gap.to_string(),
        found_in: found_in.to_string(),
        found_by: found_by.to_string(),
        at: now,
        resolved_at: None,
    };
    all.push(debt.clone());
    write_debts(root, &all)?;
    Ok(debt)
}

pub fn debts(root: &std::path::Path) -> Vec<Debt> {
    std::fs::read_to_string(root.join(DEBT_FILE))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn resolve_debt(root: &std::path::Path, id: &str, now: i64) -> AppResult<()> {
    let mut all = debts(root);
    let Some(debt) = all.iter_mut().find(|d| d.id == id) else {
        return err(format!("no debt {id}"));
    };
    debt.resolved_at = Some(now);
    write_debts(root, &all)
}

fn write_debts(root: &std::path::Path, all: &[Debt]) -> AppResult<()> {
    let path = root.join(DEBT_FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(all)?)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// The whole check
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuityReport {
    pub document_id: String,
    pub entities: Vec<Entity>,
    pub canon: CanonCheck,
    pub ledger: LedgerReport,
    pub open_debts: Vec<Debt>,
}

/// Everything the Continuity agent would gather, with no model involved.
///
/// A model can then reason over this. It could not have gathered it: the canon
/// format, the plants and payoffs, and which sheets exist are facts about the
/// project, not things to infer from prose.
pub fn report(root: &std::path::Path, document_id: &str) -> AppResult<ContinuityReport> {
    let docs = crate::project::documents(root)?;
    let passage = crate::project::read_body(root, document_id)?;
    let canon = read_canon(root, &docs)?;
    Ok(ContinuityReport {
        document_id: document_id.to_string(),
        entities: extract_entities(&passage, &docs),
        canon: check_against_canon(&passage, &canon),
        ledger: check_foreshadowing_ledger(&docs),
        open_debts: debts(root).into_iter().filter(|d| d.resolved_at.is_none()).collect(),
    })
}

#[allow(dead_code)]
fn unused(_: &Connection) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(id: &str, title: &str, kind: DocKind) -> DocumentRow {
        DocumentRow {
            id: id.into(),
            parent_id: None,
            title: title.into(),
            kind,
            path: Some(format!("x/{id}.md")),
            position: 0,
            synopsis: None,
            label: None,
            status: None,
            word_count: 0,
            updated_at: 0,
            notes: None,
            target: None,
            pov: None,
            plants: None,
            payoffs: None,
            keywords: Vec::new(),
            links: Vec::new(),
            section_type: None,
            icon: None,
            x: None,
            y: None,
        }
    }

    #[test]
    fn entities_come_from_the_authors_own_sheets_first() {
        let docs = vec![
            doc("c1", "Tamsin Reyes", DocKind::Sheet),
            doc("c2", "The Cartographer", DocKind::Sheet),
            doc("l1", "Kharsa", DocKind::Location),
            doc("s1", "A scene", DocKind::Scene),
        ];
        let passage = "Tamsin walked back to Kharsa on the twelfth day. Tamsin had counted \
                       the water twice. The cartographer was not with her.";
        let found = extract_entities(passage, &docs);

        let tamsin = found.iter().find(|e| e.name == "Tamsin Reyes").unwrap();
        assert_eq!(tamsin.kind, "character");
        assert_eq!(tamsin.mentions, 2, "matched on the first name");
        assert_eq!(tamsin.document_id.as_deref(), Some("c1"));

        let kharsa = found.iter().find(|e| e.name == "Kharsa").unwrap();
        assert_eq!(kharsa.kind, "location");

        assert!(found.iter().any(|e| e.name == "day" && e.kind == "date"));
        // A scene is not an entity.
        assert!(!found.iter().any(|e| e.name == "A scene"));
    }

    #[test]
    fn a_name_inside_another_word_is_not_a_mention() {
        let docs = vec![doc("c1", "Sal", DocKind::Sheet)];
        let found = extract_entities("The salt flats were salty and Sal was not there.", &docs);
        assert_eq!(
            found.iter().find(|e| e.name == "Sal").unwrap().mentions,
            1,
            "salt and salty are not Sal"
        );
    }

    #[test]
    fn a_short_sheet_title_does_not_match_everything() {
        let docs = vec![doc("c1", "Al", DocKind::Sheet)];
        // Two letters would match "also", "alone", "always"…
        assert!(extract_entities("He was also alone and always late.", &docs).is_empty());
    }

    #[test]
    fn canon_entries_parse_in_the_format_the_app_writes() {
        let body = "- The cartographer's left leg was broken on the descent.  \n  \
                    *established in:* 2. The Descent\n\n\
                    - Water is rationed at four measures a day.  \n  \
                    *established in:* 1. The Well\n";
        let dir = std::env::temp_dir().join(format!("aegis-canon-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let summary = crate::project::create(&dir, "Canon", "novel").unwrap();
        let root = std::path::Path::new(&summary.path);
        let canon_doc = crate::project::documents(root)
            .unwrap()
            .into_iter()
            .find(|d| d.title == "Canon")
            .unwrap();
        let entry = crate::project::create_document(root, Some(&canon_doc.id), "Facts", DocKind::Canon)
            .unwrap();
        crate::project::write_body(root, &entry.id, body).unwrap();

        let found = read_canon(root, &crate::project::documents(root).unwrap()).unwrap();
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].fact.contains("left leg"));
        assert_eq!(found[0].source, "2. The Descent");
        assert!(found.iter().all(|e| !e.source.is_empty()), "every fact has its source");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn canon_retrieval_finds_the_rule_a_scene_is_near_without_judging_it() {
        let canon = vec![
            CanonEntry {
                fact: "The cartographer's left leg was broken on the descent".into(),
                source: "2. The Descent".into(),
                document_id: "c".into(),
            },
            CanonEntry {
                fact: "Water is rationed at four measures a day".into(),
                source: "1. The Well".into(),
                document_id: "c".into(),
            },
        ];
        // The classic continuity error: the injured leg, sprinted on.
        let passage = "He sprinted away on his broken left leg, which the descent had cost him, \
                       and did not look back at the rim.";
        let check = check_against_canon(passage, &canon);

        assert_eq!(check.canon_entries, 2);
        assert!(!check.candidates.is_empty());
        assert!(
            check.candidates[0].fact.contains("left leg"),
            "the nearest rule must surface first: {:?}",
            check.candidates
        );
        assert_eq!(check.candidates[0].source, "2. The Descent");
        assert!(
            check.method.contains("nothing here is a verdict"),
            "retrieval must not present itself as judgement"
        );
        // The unrelated rule about water does not surface.
        assert!(!check.candidates.iter().any(|c| c.fact.contains("rationed")));
    }

    #[test]
    fn the_foreshadowing_ledger_separates_promises_from_payoffs() {
        let mut plant_only = doc("s1", "1. The Well", DocKind::Scene);
        plant_only.plants = Some("stone-marker, missing-map".into());
        let mut both = doc("s2", "2. The Descent", DocKind::Scene);
        both.plants = Some("broken-leg".into());
        both.payoffs = Some("stone-marker".into());
        let mut payoff_only = doc("s3", "3. The Return", DocKind::Scene);
        payoff_only.payoffs = Some("broken-leg, unexplained-debt".into());

        let report = check_foreshadowing_ledger(&[plant_only, both, payoff_only]);

        let unpaid: Vec<&str> = report.unpaid.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(unpaid, vec!["missing-map"], "a promise the book has not kept");

        let unplanted: Vec<&str> = report.unplanted.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(unplanted, vec!["unexplained-debt"], "a payoff from nowhere");

        let complete: Vec<&str> = report.complete.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(complete, vec!["broken-leg", "stone-marker"]);

        // The report names where, not just what.
        let thread = report.unpaid.iter().find(|t| t.id == "missing-map").unwrap();
        assert_eq!(thread.planted_in[0].1, "1. The Well");
    }

    #[test]
    fn an_empty_project_has_an_empty_ledger_rather_than_an_error() {
        let report = check_foreshadowing_ledger(&[doc("s1", "A scene", DocKind::Scene)]);
        assert!(report.unpaid.is_empty() && report.unplanted.is_empty() && report.complete.is_empty());
    }

    #[test]
    fn a_gap_is_written_down_rather_than_filled_in() {
        let dir = std::env::temp_dir().join(format!("aegis-debt-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let summary = crate::project::create(&dir, "Debt", "novel").unwrap();
        let root = std::path::Path::new(&summary.path);

        let d = log_propagation_debt(
            root,
            "The book never establishes what the guild robes look like",
            "4. The Hall",
            "llama3.2:3b",
            100,
        )
        .unwrap();
        assert!(d.resolved_at.is_none());
        assert_eq!(d.found_by, "llama3.2:3b");

        // The same gap noticed again is one debt, not two.
        let again = log_propagation_debt(
            root,
            "the book never establishes what the guild robes look like",
            "7. The Road",
            "someone else",
            200,
        )
        .unwrap();
        assert_eq!(again.id, d.id);
        assert_eq!(debts(root).len(), 1);

        // It is a readable file in the project.
        assert!(root.join(DEBT_FILE).exists());
        let raw = std::fs::read_to_string(root.join(DEBT_FILE)).unwrap();
        assert!(raw.contains("guild robes"));

        resolve_debt(root, &d.id, 300).unwrap();
        assert_eq!(debts(root)[0].resolved_at, Some(300));

        // A gap too vague to act on is refused.
        assert!(log_propagation_debt(root, "unclear", "x", "y", 1).is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
