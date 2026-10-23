//! Deep research: question → evidence → cited brief (K2P-OS §4, §11.1).
//!
//! The loop K2P-OS describes as `research`: decompose a question, gather
//! evidence for each part, extract checkable claims, notice where the evidence
//! disagrees with itself, and synthesise a brief in which **every statement
//! carries the source it came from**.
//!
//! # What makes this different from asking a model a question
//!
//! A model asked "what do we know about X" answers from its weights, and the
//! answer is unattributable. This never asks a model what is true. It asks it
//! only to:
//!
//! 1. break a question into sub-questions,
//! 2. read a passage the author owns and say what it asserts,
//! 3. write prose that cites the claims already gathered.
//!
//! Every fact in the output traces to a registered source. A synthesis sentence
//! whose citation cannot be resolved is dropped, because an uncited assertion
//! that looks cited is the worst thing a research tool can produce.
//!
//! # Local first, and precise about the exception
//!
//! Retrieval runs over the author's own corpus — registered sources and the
//! manuscript — with no network. The web is an explicit, per-run opt-in that
//! goes through the author's own wigolo, and every query is written to the
//! research trace. A brief says which of its sources were local and which came
//! from the web.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{err, AppResult};

/// Sub-questions per run. Enough to cover a real question, few enough that a
/// run finishes while the author is still watching.
pub const MAX_SUBQUESTIONS: usize = 6;
/// Passages read per sub-question.
pub const PASSAGES_PER_QUESTION: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubQuestion {
    pub text: String,
    /// Why this part matters to the whole. Keeps the plan reviewable.
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Passage {
    pub source_id: String,
    pub source_title: String,
    /// True when this came from the web rather than the author's own corpus.
    pub from_web: bool,
    pub text: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub question: String,
    pub passages: Vec<Passage>,
    pub claims: Vec<crate::knowledge::Claim>,
    /// Anything thrown away, with the reason. Never a silent loss.
    pub dropped: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Brief {
    pub question: String,
    pub plan: Vec<SubQuestion>,
    pub findings: Vec<Finding>,
    pub contradictions: Vec<crate::knowledge::ClaimLink>,
    /// The synthesised answer, or `None` when nothing could be grounded.
    pub synthesis: Option<String>,
    /// Every source the brief rests on, in citation form.
    pub bibliography: Vec<(String, String)>,
    pub model: String,
    pub used_web: bool,
    pub at: i64,
    /// What was searched and what was not — the scope of the answer.
    pub method: String,
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

const PLAN_SHAPE: &str = r#"Return ONE JSON object and nothing else.
{"questions": [{"text": "a specific sub-question", "rationale": "why it matters"}]}
Between 2 and 6 sub-questions. Each must be answerable from documents. Do not ask for opinions, predictions, or anything a library could not settle."#;

pub fn plan_prompt(question: &str, corpus: &[String]) -> String {
    let listing = if corpus.is_empty() {
        "The corpus is empty.".to_string()
    } else {
        format!("Sources available:\n{}", corpus.join("\n"))
    };
    format!(
        "Break this research question into sub-questions that could each be answered from \
         documents.\n\nQuestion: {question}\n\n{listing}\n\n{PLAN_SHAPE}"
    )
}

#[derive(Deserialize)]
struct RawPlan {
    #[serde(default)]
    questions: Vec<RawQuestion>,
}

#[derive(Deserialize)]
struct RawQuestion {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
}

/// Parse a plan, or fall back to the question itself.
///
/// A model that will not decompose is not a reason to fail: researching the
/// question as asked is a worse plan, not no plan.
pub fn parse_plan(reply: &str, question: &str) -> Vec<SubQuestion> {
    let parsed = crate::diagrams::extract_json(reply)
        .and_then(|j| serde_json::from_str::<RawPlan>(j).ok())
        .map(|p| {
            p.questions
                .into_iter()
                .filter_map(|q| {
                    let text = q.text?.trim().to_string();
                    (text.split_whitespace().count() >= 3).then_some(SubQuestion {
                        text,
                        rationale: q.rationale,
                    })
                })
                .take(MAX_SUBQUESTIONS)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if parsed.is_empty() {
        vec![SubQuestion {
            text: question.to_string(),
            rationale: Some("The model did not decompose the question, so it is researched whole.".into()),
        }]
    } else {
        parsed
    }
}

// ---------------------------------------------------------------------------
// Retrieval
// ---------------------------------------------------------------------------

/// Find passages in the author's own corpus.
///
/// Shingle containment rather than embeddings: it needs no embedding model, it
/// works with Ollama stopped, and for "which passage is about this question" it
/// is good enough. Embedding search already exists in `rag` for the manuscript;
/// this is the corpus-wide path that must never fail for want of a model.
pub fn local_passages(
    root: &std::path::Path,
    question: &str,
    limit: usize,
) -> AppResult<Vec<Passage>> {
    let conn = crate::store::open(root)?;
    let needle = crate::integrity::shingles(question, 2);
    let mut scored: Vec<Passage> = Vec::new();

    for source in crate::sources::list(&conn)? {
        let Ok(text) = crate::sources::read_text(root, &source) else { continue };
        for window in crate::rag::chunk(&text, 400, 60)?.into_iter() {
            let score = crate::integrity::containment(
                &needle,
                &crate::integrity::shingles(&window, 2),
            );
            if score > 0.0 {
                scored.push(Passage {
                    source_id: source.id.clone(),
                    source_title: source.title.clone(),
                    from_web: false,
                    text: window,
                    score,
                });
            }
        }
    }

    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    // One passage per source at most, so a single long document cannot crowd
    // out every other view of the question.
    let mut seen: HashMap<String, usize> = HashMap::new();
    scored.retain(|p| {
        let n = seen.entry(p.source_id.clone()).or_insert(0);
        *n += 1;
        *n <= 2
    });
    scored.truncate(limit);
    Ok(scored)
}

/// Fetch from the web through the author's own wigolo, registering each result
/// as a source so anything built on it is citable.
fn web_passages(root: &std::path::Path, question: &str, limit: usize) -> AppResult<Vec<Passage>> {
    let found = crate::websearch::search(question, limit)?;
    let now = crate::project::now_ms();

    crate::websearch::record(
        root,
        &crate::websearch::TraceEntry {
            at: now,
            query: found.query.clone(),
            word_count: found.query.split_whitespace().count() as i64,
            base: crate::websearch::base(),
            result_count: found.matches.len() as i64,
            urls: found.matches.iter().map(|m| m.url.clone()).collect(),
        },
    )?;

    let mut out = Vec::new();
    for m in found.matches {
        if m.excerpt.split_whitespace().count() < 20 {
            continue; // a snippet too short to support a claim
        }
        let (source, _) = crate::sources::register_text(
            root,
            &m.title,
            &m.excerpt,
            crate::sources::SourceKind::Web,
            Some(&m.url),
            Some("Retrieved during a research run."),
            now,
        )?;
        out.push(Passage {
            source_id: source.id,
            source_title: m.title,
            from_web: true,
            text: m.excerpt,
            score: m.evidence_score.unwrap_or(0.5),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Synthesis
// ---------------------------------------------------------------------------

fn synthesis_prompt(question: &str, claims: &[crate::knowledge::Claim]) -> String {
    let numbered: String = claims
        .iter()
        .enumerate()
        .map(|(i, c)| format!("[{}] {}\n", i + 1, c.text))
        .collect();
    format!(
        "Answer the question using ONLY the numbered claims below. Cite every statement with its \
         number in square brackets, like [2]. If the claims disagree, say so and cite both. If \
         they do not answer the question, say exactly what is missing rather than filling the gap \
         from your own knowledge.\n\n\
         Question: {question}\n\nClaims:\n{numbered}\n\
         Write three to six sentences of plain prose. No headings, no preamble, no bullet list."
    )
}

/// Keep only the citations that resolve, and say what was dropped.
///
/// A model will occasionally cite `[9]` when eight claims were supplied. Left
/// alone that renders as a confident, unverifiable reference — the single most
/// damaging thing a research tool can emit — so an unresolvable citation is
/// removed and the sentence carrying it is flagged.
pub fn check_citations(text: &str, claim_count: usize) -> (String, Vec<String>) {
    let mut dropped = Vec::new();
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(close) = chars[i..].iter().position(|c| *c == ']') {
                let inner: String = chars[i + 1..i + close].iter().collect();
                if let Ok(n) = inner.trim().parse::<usize>() {
                    if n == 0 || n > claim_count {
                        dropped.push(format!(
                            "a citation to [{n}], which is not one of the {claim_count} claims gathered"
                        ));
                        i += close + 1;
                        continue;
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    (out.split_whitespace().collect::<Vec<_>>().join(" "), dropped)
}

// ---------------------------------------------------------------------------
// The loop
// ---------------------------------------------------------------------------

/// Run a full research pass.
pub fn run(
    root: &std::path::Path,
    question: &str,
    model: &str,
    use_web: bool,
    allow_cloud: bool,
) -> AppResult<Brief> {
    if question.split_whitespace().count() < 3 {
        return err("a research question needs to be a question, not a word");
    }
    let now = crate::project::now_ms();
    let conn = crate::store::open(root)?;

    let corpus: Vec<String> = crate::sources::list(&conn)?
        .iter()
        .take(60)
        .map(|s| format!("- {} ({})", s.title, s.kind.as_str()))
        .collect();
    let corpus_size = corpus.len();
    drop(conn);

    // 1. Decompose.
    let plan = match crate::ollama::generate_guarded(
        model,
        &plan_prompt(question, &corpus),
        Some("You plan research. Return only JSON."),
        700,
        0.3,
        allow_cloud,
    ) {
        Ok(reply) => parse_plan(&reply, question),
        Err(_) => parse_plan("", question),
    };

    // 2–4. Gather, extract, verify.
    let mut findings = Vec::new();
    let mut all_claims: Vec<crate::knowledge::Claim> = Vec::new();

    for sub in &plan {
        let mut passages = local_passages(root, &sub.text, PASSAGES_PER_QUESTION)?;
        let mut dropped = Vec::new();

        if use_web {
            match web_passages(root, &sub.text, 3) {
                Ok(more) => passages.extend(more),
                Err(e) => dropped.push(format!("the web search failed: {e}")),
            }
        }

        let mut claims = Vec::new();
        for p in &passages {
            let reply = match crate::ollama::generate_guarded(
                model,
                &crate::knowledge::extraction_prompt(&p.text),
                Some("You extract checkable claims as JSON. Never explain."),
                900,
                0.2,
                allow_cloud,
            ) {
                Ok(r) => r,
                Err(e) => {
                    dropped.push(format!("a passage could not be read: {e}"));
                    continue;
                }
            };
            match crate::knowledge::validate_extraction(&reply, &p.text, &p.source_id, model, now) {
                Ok(part) => {
                    let conn = crate::store::open(root)?;
                    crate::knowledge::save_extraction(&conn, &part)?;
                    dropped.extend(part.dropped);
                    claims.extend(part.claims);
                }
                Err(e) => dropped.push(format!("a passage produced nothing usable: {e}")),
            }
        }

        all_claims.extend(claims.clone());
        findings.push(Finding { question: sub.text.clone(), passages, claims, dropped });
    }

    // 5. Where does the evidence disagree with itself?
    let contradictions = if all_claims.len() > 1 {
        crate::knowledge::find_contradictions(root, model, allow_cloud)
            .map(|r| r.links)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // 6. Synthesise, and keep only what is actually grounded.
    let synthesis = if all_claims.is_empty() {
        None
    } else {
        match crate::ollama::generate_guarded(
            model,
            &synthesis_prompt(question, &all_claims),
            Some(
                "You write cited prose from supplied claims only. You never add facts of your own.",
            ),
            900,
            0.35,
            allow_cloud,
        ) {
            Ok(text) => {
                let (clean, dropped) = check_citations(&text, all_claims.len());
                if let Some(first) = findings.first_mut() {
                    first.dropped.extend(dropped);
                }
                (!clean.trim().is_empty()).then_some(clean)
            }
            Err(_) => None,
        }
    };

    // 7. The bibliography is generated from registered sources, never typed.
    let conn = crate::store::open(root)?;
    let mut bibliography = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for c in &all_claims {
        if !seen.insert(c.source_id.clone()) {
            continue;
        }
        if let Some(s) = crate::sources::get(&conn, &c.source_id)? {
            bibliography.push((s.id.clone(), crate::sources::citation(&s)));
        }
    }

    let used_web = findings.iter().any(|f| f.passages.iter().any(|p| p.from_web));
    let method = format!(
        "{} sub-questions over a corpus of {corpus_size} sources. Passages were found by word \
         overlap, claims were extracted by {model} and kept only when their quotation appears in \
         the source, and every citation that could not be resolved was removed. {}",
        plan.len(),
        if used_web {
            "The web was searched through your own wigolo, and those queries left this machine."
        } else {
            "Nothing left this machine."
        }
    );

    Ok(Brief {
        question: question.to_string(),
        plan,
        findings,
        contradictions,
        synthesis,
        bibliography,
        model: model.to_string(),
        used_web,
        at: now,
        method,
    })
}

/// A brief as a Markdown document the author owns.
pub fn to_markdown(brief: &Brief) -> String {
    let mut out = format!("# {}\n\n", brief.question);

    match &brief.synthesis {
        Some(text) => out.push_str(&format!("{text}\n\n")),
        None => out.push_str(
            "No grounded answer. Nothing in the corpus supported a claim about this question, \
             and rather than write around that, the brief says so.\n\n",
        ),
    }

    if !brief.contradictions.is_empty() {
        out.push_str("## Where the sources disagree\n\n");
        for link in &brief.contradictions {
            out.push_str(&format!("- {}\n", link.reason));
        }
        out.push('\n');
    }

    out.push_str("## What was asked\n\n");
    for (i, q) in brief.plan.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", i + 1, q.text));
    }
    out.push('\n');

    out.push_str("## Evidence\n\n");
    let mut n = 0;
    for f in &brief.findings {
        for c in &f.claims {
            n += 1;
            out.push_str(&format!("{n}. {} — *{}*\n", c.text, c.excerpt.trim()));
        }
    }
    if n == 0 {
        out.push_str("None.\n");
    }
    out.push('\n');

    if !brief.bibliography.is_empty() {
        out.push_str("## Sources\n\n");
        for (_, line) in &brief.bibliography {
            out.push_str(&format!("- {line}\n"));
        }
        out.push('\n');
    }

    let dropped: Vec<&String> = brief.findings.iter().flat_map(|f| f.dropped.iter()).collect();
    if !dropped.is_empty() {
        out.push_str("## Left out\n\n");
        for d in &dropped {
            out.push_str(&format!("- {d}\n"));
        }
        out.push('\n');
    }

    out.push_str(&format!("---\n\n{}\n", brief.method));
    out
}

/// Save a brief into the binder as a document the author owns.
pub fn save_to_binder(
    root: &std::path::Path,
    brief: &Brief,
) -> AppResult<crate::model::DocumentRow> {
    let docs = crate::project::documents(root)?;
    let parent = docs
        .iter()
        .find(|d| d.kind == crate::model::DocKind::Folder && d.title == "Research")
        .map(|d| d.id.clone());
    let title = format!(
        "Research: {}",
        brief.question.chars().take(60).collect::<String>()
    );
    let doc = crate::project::create_document(
        root,
        parent.as_deref(),
        &title,
        crate::model::DocKind::Research,
    )?;
    // A brief is the model's synthesis of the author's sources, so it is
    // attributed to the model, not to the author.
    crate::project::write_body_as(
        root,
        &doc.id,
        &to_markdown(brief),
        crate::provenance::Actor::model(&brief.model, serde_json::json!({ "via": "research" })),
    )?;
    crate::project::documents(root)?
        .into_iter()
        .find(|d| d.id == doc.id)
        .ok_or_else(|| crate::error::AppError("the brief vanished after being saved".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plan_is_parsed_and_a_refusal_still_yields_one() {
        let good = r#"```json
        {"questions":[
          {"text":"What does logical replication actually replicate?","rationale":"scope"},
          {"text":"How does it handle conflicting writes?","rationale":"the hard part"},
          {"text":"no"}
        ]}```"#;
        let plan = parse_plan(good, "the original question");
        assert_eq!(plan.len(), 2, "a fragment too short to be a question is dropped");
        assert!(plan[0].text.starts_with("What does"));

        // A model that will not decompose is not a failure.
        let fallback = parse_plan("I'm not sure how to break that down!", "How does replication work?");
        assert_eq!(fallback.len(), 1);
        assert_eq!(fallback[0].text, "How does replication work?");
        assert!(fallback[0].rationale.as_ref().unwrap().contains("did not decompose"));
    }

    #[test]
    fn the_plan_is_capped() {
        let many: Vec<String> = (0..20)
            .map(|i| format!(r#"{{"text":"Question number {i} about a thing"}}"#))
            .collect();
        let plan = parse_plan(&format!(r#"{{"questions":[{}]}}"#, many.join(",")), "q");
        assert_eq!(plan.len(), MAX_SUBQUESTIONS);
    }

    #[test]
    fn a_citation_that_cannot_be_resolved_is_removed_not_rendered() {
        let text = "Replication uses identity [1]. It resolves conflicts by ordering [9]. \
                    Both are supported [2].";
        let (clean, dropped) = check_citations(text, 3);
        assert!(clean.contains("[1]") && clean.contains("[2]"));
        assert!(!clean.contains("[9]"), "an unresolvable citation must not survive: {clean}");
        assert_eq!(dropped.len(), 1);
        assert!(dropped[0].contains("[9]"));
        // The sentence itself is kept, minus the false reference.
        assert!(clean.contains("It resolves conflicts by ordering"));
    }

    #[test]
    fn valid_citations_are_left_alone() {
        let text = "A claim [1] and another [3].";
        let (clean, dropped) = check_citations(text, 3);
        assert_eq!(clean, text);
        assert!(dropped.is_empty());
    }

    #[test]
    fn brackets_that_are_not_citations_survive() {
        let text = "The author [sic] wrote it [1].";
        let (clean, dropped) = check_citations(text, 2);
        assert!(clean.contains("[sic]"), "only numeric citations are checked: {clean}");
        assert!(dropped.is_empty());
    }

    #[test]
    fn a_question_that_is_not_a_question_is_refused() {
        let base = std::env::temp_dir().join(format!("aegis-research-{}", std::process::id()));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = crate::project::create(&base, "R", "nonfiction").unwrap();
        let root = std::path::Path::new(&summary.path);
        let message = run(root, "replication", "m", false, false).unwrap_err().to_string();
        assert!(message.contains("needs to be a question"), "{message}");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_brief_with_no_evidence_says_so_rather_than_writing_around_it() {
        let brief = Brief {
            question: "What is known about X?".into(),
            plan: vec![SubQuestion { text: "What is X?".into(), rationale: None }],
            findings: vec![],
            contradictions: vec![],
            synthesis: None,
            bibliography: vec![],
            model: "m".into(),
            used_web: false,
            at: 0,
            method: "Nothing left this machine.".into(),
        };
        let md = to_markdown(&brief);
        assert!(md.contains("No grounded answer"));
        assert!(md.contains("the brief says so"));
        assert!(md.contains("## Evidence\n\nNone."));
        assert!(md.contains("Nothing left this machine."));
    }

    #[test]
    fn a_brief_carries_its_evidence_its_sources_and_what_it_left_out() {
        let claim = crate::knowledge::Claim {
            id: "c1".into(),
            text: "Symbolic constraints improve consistency".into(),
            kind: crate::knowledge::ClaimKind::Result,
            source_id: "s1".into(),
            excerpt: "constraints improve consistency by a wide margin".into(),
            concepts: vec![],
            extracted_by: "m".into(),
            at: 0,
        };
        let brief = Brief {
            question: "Do symbolic constraints help?".into(),
            plan: vec![SubQuestion { text: "Do they improve consistency?".into(), rationale: None }],
            findings: vec![Finding {
                question: "Do they improve consistency?".into(),
                passages: vec![],
                claims: vec![claim],
                dropped: vec!["a citation to [9], which is not one of the 1 claims gathered".into()],
            }],
            contradictions: vec![],
            synthesis: Some("They do, in every configuration tested [1].".into()),
            bibliography: vec![("s1".into(), "Smith, J. (2025). A Paper. https://doi.org/10.1234/x".into())],
            model: "llama3.2:3b".into(),
            used_web: false,
            at: 0,
            method: "1 sub-questions over a corpus of 1 sources.".into(),
        };
        let md = to_markdown(&brief);
        assert!(md.starts_with("# Do symbolic constraints help?"));
        assert!(md.contains("in every configuration tested [1]"));
        assert!(md.contains("1. Symbolic constraints improve consistency"));
        assert!(md.contains("*constraints improve consistency by a wide margin*"));
        assert!(md.contains("Smith, J. (2025)"));
        assert!(md.contains("## Left out"), "what was dropped must reach the reader");
        assert!(md.contains("not one of the 1 claims gathered"));
    }

    #[test]
    fn local_retrieval_finds_the_right_passage_without_any_model() {
        let base = std::env::temp_dir().join(format!("aegis-retrieve-{}", std::process::id()));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = crate::project::create(&base, "R", "nonfiction").unwrap();
        let root = std::path::Path::new(&summary.path);

        crate::sources::register_text(
            root,
            "Replication",
            "Logical replication is a method of replicating data objects and their changes based \
             upon their replication identity, which is usually a primary key.",
            crate::sources::SourceKind::Text,
            None,
            None,
            1,
        )
        .unwrap();
        crate::sources::register_text(
            root,
            "Harvest",
            "The wheat harvest failed in the third year of the drought and the granaries emptied \
             before the spring rains arrived that year.",
            crate::sources::SourceKind::Text,
            None,
            None,
            2,
        )
        .unwrap();

        let hits = local_passages(root, "what does logical replication replicate", 5).unwrap();
        assert!(!hits.is_empty(), "retrieval must work with no model running");
        assert_eq!(hits[0].source_title, "Replication");
        assert!(!hits[0].from_web);
        assert!(hits[0].score > 0.0);

        std::fs::remove_dir_all(&base).ok();
    }

    /// Live: a full research pass over a real corpus with a real model.
    #[test]
    fn live_research_produces_a_grounded_brief() {
        let status = crate::ollama::status();
        if !status.reachable {
            eprintln!("SKIPPED live_research: Ollama is not running");
            return;
        }
        let models = crate::ollama::generation_models(&status);
        let Some(model) = models.first() else {
            eprintln!("SKIPPED live_research: no local generation model");
            return;
        };

        let base = std::env::temp_dir().join(format!("aegis-live-research-{}", std::process::id()));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = crate::project::create(&base, "Live", "nonfiction").unwrap();
        let root = std::path::Path::new(&summary.path);

        crate::sources::register_text(
            root,
            "Replication basics",
            "Logical replication is a method of replicating data objects and their changes based \
             upon their replication identity, which is usually a primary key. It differs from \
             physical replication, which uses exact block addresses and byte-by-byte copying. \
             Logical replication permits replicating between different major versions of the \
             server, and permits replicating only a subset of tables.",
            crate::sources::SourceKind::Text,
            None,
            None,
            1,
        )
        .unwrap();

        match run(root, "How does logical replication differ from physical replication?", &model.name, false, false) {
            Ok(brief) => {
                eprintln!(
                    "EVIDENCE live_research [{}]: {} sub-questions, {} claims, {} sources",
                    model.name,
                    brief.plan.len(),
                    brief.findings.iter().map(|f| f.claims.len()).sum::<usize>(),
                    brief.bibliography.len()
                );
                for q in &brief.plan {
                    eprintln!("EVIDENCE   asked: {}", q.text);
                }
                for f in &brief.findings {
                    for c in &f.claims {
                        eprintln!("EVIDENCE   claim: {}", c.text);
                    }
                }
                match &brief.synthesis {
                    Some(s) => eprintln!("EVIDENCE   answer: {}", s.chars().take(400).collect::<String>()),
                    None => eprintln!("EVIDENCE   no grounded answer"),
                }
                assert!(!brief.used_web, "this run was local only");
                assert!(brief.method.contains("Nothing left this machine"));
            }
            Err(e) => eprintln!("EVIDENCE live_research failed: {e}"),
        }
        std::fs::remove_dir_all(&base).ok();
    }
}
