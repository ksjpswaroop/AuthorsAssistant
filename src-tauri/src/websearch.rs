//! Web similarity through **wigolo** (Phase 4).
//!
//! Turnitin-class integrity needs one thing this app cannot do alone: ask
//! whether a phrase already exists somewhere on the web. AegisDraft does not
//! query search engines itself. It posts to a wigolo daemon on
//! `127.0.0.1:3333`, which [`crate::sidecar`] can start and stop for it.
//!
//! # Be precise about what leaves the machine
//!
//! Wigolo listens on loopback, and it is tempting to describe this as a local
//! check. It is not. Wigolo queries the internet on the author's behalf, so an
//! approved phrase **does leave this machine** — through a component the author
//! controls and can inspect, which is a meaningful difference, but not the same
//! thing as staying put. The confirm in the UI says exactly that.
//!
//! What is sent is bounded by construction: only the phrases the author ticks,
//! one query each. There is no code path that sends a document, a chapter, or
//! the manuscript, and the word count about to leave is shown before anything
//! goes.
//!
//! # Two scores, kept apart
//!
//! `evidence_score` comes from wigolo and describes how good wigolo thinks the
//! *result* is. `overlap` is measured here, in this file, by word-shingle
//! containment between the phrase we sent and the excerpt that came back — it
//! describes how much of the phrase actually appears in the source. They answer
//! different questions and are never averaged into one number.
//!
//! # On the contract
//!
//! This client is written against wigolo's published REST contract, and its
//! parsing is deliberately forgiving: the response shape differs between
//! wigolo's own README and its REST reference (`excerpt` versus `snippet`,
//! `citation_id` and `source_span` present in one and not the other). Every
//! field but `title` and `url` is optional here, and a missing one is reported
//! as absent rather than defaulted to a number that would look measured.
//!
//! That forgiveness earned itself. Run live against wigolo 0.2.1, the daemon
//! sent `snippet` with an `evidence_score` and **no** `citation_id` or
//! `source_span` — the REST-reference shape, not the README one. A client that
//! had trusted the README would have returned nothing.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{err, AppResult};

pub const DEFAULT_BASE: &str = "http://127.0.0.1:3333";

pub fn base() -> String {
    std::env::var("AEGISDRAFT_WIGOLO").unwrap_or_else(|_| DEFAULT_BASE.to_string())
}

/// A bearer token, needed only for a wigolo bound to something other than
/// loopback. Absent is the normal case.
pub fn token() -> Option<String> {
    std::env::var("WIGOLO_API_TOKEN").ok().filter(|t| !t.is_empty())
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        // wigolo puts an actionable reason in the body (`error_reason`, `hint`);
        // a bare status number would throw that away.
        .http_status_as_error(false)
        .build()
        .into()
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WigoloStatus {
    pub running: bool,
    pub base: String,
    /// wigolo's own health verdict, when it answered.
    pub health: Option<String>,
    /// Why it could not be reached, in words the author can act on.
    pub detail: Option<String>,
    /// True when a bearer token is configured here.
    pub authenticated: bool,
}

#[derive(Deserialize)]
struct HealthBody {
    #[serde(default)]
    status: Option<String>,
}

/// Never an error: "not running" is an ordinary state the UI has to show, the
/// same way it shows Ollama being absent.
pub fn status() -> WigoloStatus {
    status_at(&base())
}

pub fn status_at(base: &str) -> WigoloStatus {
    let mut out = WigoloStatus {
        running: false,
        base: base.to_string(),
        health: None,
        detail: None,
        authenticated: token().is_some(),
    };
    // `/health` is open even when a token is configured, so it is the honest
    // probe: a 401 here would mean something else is wrong.
    match agent(Duration::from_secs(4)).get(&format!("{base}/health")).call() {
        Ok(mut res) => {
            let code = res.status().as_u16();
            let body = res.body_mut().read_to_string().unwrap_or_default();
            if code >= 300 {
                out.detail = Some(format!("wigolo answered {code}: {}", body.trim()));
                return out;
            }
            out.running = true;
            out.health = serde_json::from_str::<HealthBody>(&body)
                .ok()
                .and_then(|h| h.status)
                .or(Some("healthy".to_string()));
        }
        Err(e) => {
            out.detail = Some(format!(
                "no wigolo at {base} ({e}). Start one with `wigolo serve`, or point AEGISDRAFT_WIGOLO somewhere else."
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SearchRequest<'a> {
    query: &'a str,
    max_results: usize,
    /// wigolo's own knob; "fast" keeps a per-phrase check to a few seconds.
    search_depth: &'a str,
}

#[derive(Deserialize, Default)]
struct Evidence {
    #[serde(rename = "final", default)]
    final_score: Option<f32>,
}

#[derive(Deserialize, Default)]
struct Span {
    #[serde(default)]
    start: Option<i64>,
    #[serde(default)]
    end: Option<i64>,
}

#[derive(Deserialize)]
struct RawResult {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    url: Option<String>,
    // wigolo's README calls it `excerpt`; its REST reference shows `snippet`.
    // Accept either, and say so rather than silently showing an empty match.
    #[serde(default)]
    excerpt: Option<String>,
    #[serde(default)]
    snippet: Option<String>,
    #[serde(default)]
    citation_id: Option<String>,
    #[serde(default)]
    source_span: Option<Span>,
    #[serde(default)]
    evidence_score: Option<Evidence>,
}

#[derive(Deserialize, Default)]
struct Freshness {
    #[serde(default)]
    published: Option<String>,
}

#[derive(Deserialize)]
struct SearchBody {
    #[serde(default)]
    results: Vec<RawResult>,
    #[serde(default)]
    engines_used: Vec<String>,
    #[serde(default)]
    freshness_signal: Option<Freshness>,
    /// wigolo keeps degraded-but-successful calls at 2xx and reports the
    /// problem in the body, so this has to be read rather than the status code.
    #[serde(default)]
    warning: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebMatch {
    pub title: String,
    pub url: String,
    pub excerpt: String,
    /// wigolo's citation id, so a report can cite a source rather than assert a
    /// number. Absent on versions that do not send one.
    pub citation_id: Option<String>,
    /// Byte offsets into the source document, from wigolo.
    pub source_span: Option<(i64, i64)>,
    /// wigolo's own confidence in the result. **Not** a similarity score.
    pub evidence_score: Option<f32>,
    /// Word-shingle containment of the sent phrase in the returned excerpt,
    /// measured here. This is the number that answers "does my sentence appear
    /// in that page".
    pub overlap: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchResult {
    pub query: String,
    pub matches: Vec<WebMatch>,
    pub engines: Vec<String>,
    pub published: Option<String>,
    /// Anything wigolo flagged about this call — a failed engine, stale cache.
    pub warning: Option<String>,
}

/// Word-shingle containment: what share of the phrase's 3-word runs appear in
/// the excerpt. Containment rather than Jaccard, because a long page excerpt
/// would drown a short phrase in a symmetric measure.
pub fn containment(phrase: &str, excerpt: &str) -> f32 {
    let shingles = |text: &str| -> Vec<String> {
        let words: Vec<String> = text
            .split_whitespace()
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                    .to_lowercase()
            })
            .filter(|w| !w.is_empty())
            .collect();
        if words.len() < 3 {
            return words;
        }
        words.windows(3).map(|w| w.join(" ")).collect()
    };
    let mine = shingles(phrase);
    if mine.is_empty() {
        return 0.0;
    }
    let theirs: std::collections::HashSet<String> = shingles(excerpt).into_iter().collect();
    let hits = mine.iter().filter(|s| theirs.contains(*s)).count();
    hits as f32 / mine.len() as f32
}

pub fn search(phrase: &str, max_results: usize) -> AppResult<WebSearchResult> {
    search_at(&base(), token().as_deref(), phrase, max_results)
}

pub fn search_at(
    base: &str,
    token: Option<&str>,
    phrase: &str,
    max_results: usize,
) -> AppResult<WebSearchResult> {
    let trimmed = phrase.trim();
    if trimmed.is_empty() {
        return err("there is no phrase to check");
    }
    // A guard on the caller, not a formality: this is the one place in the app
    // where the author's words leave the machine, and it must be a phrase.
    let word_count = trimmed.split_whitespace().count();
    if word_count > 60 {
        return err(format!(
            "{word_count} words is a passage, not a phrase — web checks send phrases only. Select less."
        ));
    }

    let mut request = agent(Duration::from_secs(60)).post(&format!("{base}/v1/search"));
    if let Some(token) = token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut res = request
        .send_json(SearchRequest {
            query: trimmed,
            max_results: max_results.clamp(1, 10),
            search_depth: "fast",
        })
        .map_err(|e| {
            crate::error::AppError(format!(
                "could not reach wigolo at {base} ({e}). Is `wigolo serve` running?"
            ))
        })?;

    let code = res.status().as_u16();
    let body = res.body_mut().read_to_string().unwrap_or_default();
    if code >= 300 {
        return err(format!("wigolo answered {code}: {}", body.trim()));
    }
    let parsed: SearchBody = serde_json::from_str(&body).map_err(|e| {
        crate::error::AppError(format!("wigolo sent something this build cannot read: {e}"))
    })?;

    let mut matches = Vec::new();
    for raw in parsed.results {
        let Some(url) = raw.url else { continue };
        let excerpt = raw.excerpt.or(raw.snippet).unwrap_or_default();
        matches.push(WebMatch {
            title: raw.title.unwrap_or_else(|| url.clone()),
            overlap: containment(trimmed, &excerpt),
            citation_id: raw.citation_id,
            source_span: raw
                .source_span
                .and_then(|s| Some((s.start?, s.end?))),
            evidence_score: raw.evidence_score.and_then(|e| e.final_score),
            excerpt,
            url,
        });
    }
    // Strongest textual overlap first: that is the one an author has to explain.
    matches.sort_by(|a, b| b.overlap.total_cmp(&a.overlap));

    Ok(WebSearchResult {
        query: trimmed.to_string(),
        matches,
        engines: parsed.engines_used,
        published: parsed.freshness_signal.and_then(|f| f.published),
        warning: parsed.warning.or(parsed.error),
    })
}

// ---------------------------------------------------------------------------
// Research trace
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceEntry {
    pub at: i64,
    /// Exactly what left the machine.
    pub query: String,
    pub word_count: i64,
    pub base: String,
    pub result_count: i64,
    /// The URLs that came back, so the record is auditable later.
    pub urls: Vec<String>,
}

pub const TRACE_FILE: &str = "research-trace.json";

/// Append one entry to the project's research trace.
///
/// The trace is the receipt: if the author is ever asked what they looked up
/// while writing, the answer is a file in their own project, not a promise.
pub fn record(root: &Path, entry: &TraceEntry) -> AppResult<()> {
    let path = root.join(TRACE_FILE);
    let mut all: Vec<TraceEntry> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    all.push(entry.clone());
    std::fs::write(&path, serde_json::to_string_pretty(&all)?)?;
    Ok(())
}

pub fn trace(root: &Path) -> Vec<TraceEntry> {
    std::fs::read_to_string(root.join(TRACE_FILE))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A one-shot HTTP server answering with `body`, handing back the request it
    /// received. Enough to prove this client speaks wigolo's contract without
    /// wigolo being installed — and, more usefully, to prove it keeps working
    /// when wigolo's fields differ from one version to the next.
    fn serve_once(body: &'static str, code: u16) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).unwrap_or(0);
                let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
                let reason = if code == 200 { "OK" } else { "Error" };
                let response = format!(
                    "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        (format!("http://127.0.0.1:{port}"), rx)
    }

    const README_SHAPE: &str = r#"{
      "results": [{
        "title": "Logical replication - PostgreSQL docs",
        "url": "https://www.postgresql.org/docs/current/logical-replication.html",
        "excerpt": "the well at kharsa was the only honest thing in the town",
        "citation_id": "src-1",
        "source_span": { "start": 1042, "end": 1305 },
        "evidence_score": { "final": 0.86, "semantic": 0.91 }
      }],
      "citations": [{ "id": "src-1", "url": "x" }],
      "freshness_signal": { "published": "2026-05-12", "confidence": "high" }
    }"#;

    /// The shape wigolo's REST reference actually records: `snippet`, no
    /// citation id, no span. Both must work.
    const REST_SHAPE: &str = r#"{
      "results": [{
        "title": "Something else",
        "url": "https://example.org/a",
        "snippet": "a completely unrelated sentence about turbines",
        "relevance_score": 0.968,
        "evidence_score": {"final": 0.224, "components": {"base_rrf": 0.023}}
      }],
      "engines_used": ["bing", "duckduckgo"]
    }"#;

    const PHRASE: &str = "The well at Kharsa was the only honest thing in the town";

    #[test]
    fn it_sends_a_phrase_the_way_wigolos_contract_says_to() {
        let (base, rx) = serve_once(README_SHAPE, 200);
        search_at(&base, None, PHRASE, 3).unwrap();

        let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(request.starts_with("POST /v1/search "), "{request}");
        assert!(request.contains("content-type: application/json") || request.contains("Content-Type: application/json"));
        let compact: String = request.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(compact.contains("\"max_results\":3"), "{request}");
        assert!(request.contains("The well at Kharsa"), "the phrase itself must be the query");
        assert!(
            !request.to_lowercase().contains("authorization"),
            "no token is configured, so none must be sent"
        );
    }

    #[test]
    fn a_token_is_sent_as_a_bearer_header_when_one_exists() {
        let (base, rx) = serve_once(README_SHAPE, 200);
        search_at(&base, Some("s3cret"), PHRASE, 1).unwrap();
        let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            request.to_lowercase().contains("authorization: bearer s3cret"),
            "{request}"
        );
    }

    #[test]
    fn provenance_survives_the_round_trip() {
        let (base, _rx) = serve_once(README_SHAPE, 200);
        let out = search_at(&base, None, PHRASE, 3).unwrap();
        let m = &out.matches[0];
        assert_eq!(m.citation_id.as_deref(), Some("src-1"));
        assert_eq!(m.source_span, Some((1042, 1305)));
        assert_eq!(m.evidence_score, Some(0.86));
        assert_eq!(out.published.as_deref(), Some("2026-05-12"));
    }

    #[test]
    fn the_other_documented_shape_parses_too_and_says_what_is_missing() {
        let (base, _rx) = serve_once(REST_SHAPE, 200);
        let out = search_at(&base, None, PHRASE, 3).unwrap();
        assert_eq!(out.matches.len(), 1);
        let m = &out.matches[0];
        assert!(m.excerpt.contains("turbines"), "`snippet` must be read as the excerpt");
        assert_eq!(m.citation_id, None, "absent, not invented");
        assert_eq!(m.source_span, None);
        assert_eq!(m.evidence_score, Some(0.224));
        assert_eq!(out.engines, vec!["bing", "duckduckgo"]);
    }

    #[test]
    fn overlap_is_measured_here_and_is_not_wigolos_score() {
        let (base, _rx) = serve_once(README_SHAPE, 200);
        let hit = search_at(&base, None, PHRASE, 3).unwrap();
        assert!(
            hit.matches[0].overlap > 0.8,
            "the excerpt repeats the phrase: {}",
            hit.matches[0].overlap
        );
        assert_ne!(
            hit.matches[0].overlap,
            hit.matches[0].evidence_score.unwrap(),
            "the two numbers answer different questions and must stay apart"
        );

        let (base, _rx) = serve_once(REST_SHAPE, 200);
        let miss = search_at(&base, None, PHRASE, 3).unwrap();
        assert_eq!(miss.matches[0].overlap, 0.0, "unrelated prose overlaps nothing");
    }

    #[test]
    fn a_passage_is_refused_before_anything_leaves_the_machine() {
        // No server at all: if this reached the network the test would hang or
        // fail on connection, so passing proves the guard runs first.
        let long = "word ".repeat(200);
        let message = search_at("http://127.0.0.1:1", None, &long, 3)
            .unwrap_err()
            .to_string();
        assert!(message.contains("is a passage, not a phrase"), "{message}");
        assert!(search_at("http://127.0.0.1:1", None, "   ", 3).is_err());
    }

    #[test]
    fn an_error_body_is_reported_instead_of_a_bare_status_code() {
        let (base, _rx) = serve_once(
            r#"{"error":"missing token","error_reason":"unauthorized","hint":"set WIGOLO_API_TOKEN"}"#,
            401,
        );
        let message = search_at(&base, None, PHRASE, 3).unwrap_err().to_string();
        assert!(message.contains("401"), "{message}");
        assert!(message.contains("WIGOLO_API_TOKEN"), "the hint is the actionable half: {message}");
    }

    #[test]
    fn an_absent_wigolo_is_a_state_not_an_error() {
        let s = status_at("http://127.0.0.1:1");
        assert!(!s.running);
        assert!(s.detail.unwrap().contains("wigolo serve"), "the fix must be named");
    }

    #[test]
    fn a_healthy_wigolo_reports_its_own_verdict() {
        let (base, _rx) = serve_once(r#"{"status":"healthy","cache":"active"}"#, 200);
        let s = status_at(&base);
        assert!(s.running);
        assert_eq!(s.health.as_deref(), Some("healthy"));
    }

    #[test]
    fn the_trace_records_exactly_what_left_the_machine() {
        let dir = std::env::temp_dir().join(format!("aegis-trace-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::remove_file(dir.join(TRACE_FILE)).ok();

        record(
            &dir,
            &TraceEntry {
                at: 1,
                query: PHRASE.to_string(),
                word_count: 11,
                base: "http://127.0.0.1:3333".into(),
                result_count: 2,
                urls: vec!["https://example.org/a".into()],
            },
        )
        .unwrap();
        record(
            &dir,
            &TraceEntry {
                at: 2,
                query: "another phrase".to_string(),
                word_count: 2,
                base: "http://127.0.0.1:3333".into(),
                result_count: 0,
                urls: vec![],
            },
        )
        .unwrap();

        let all = trace(&dir);
        assert_eq!(all.len(), 2, "the trace appends, it does not replace");
        assert_eq!(all[0].query, PHRASE);
        assert_eq!(all[1].query, "another phrase");

        let raw = std::fs::read_to_string(dir.join(TRACE_FILE)).unwrap();
        assert!(raw.contains("Kharsa"), "a person must be able to read it");

        std::fs::remove_dir_all(&dir).ok();
    }
}
