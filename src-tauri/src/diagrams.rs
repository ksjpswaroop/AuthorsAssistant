//! Generated diagrams: mind maps, flowcharts, infographics (Phase 9).
//!
//! A model is asked for a graph, not for prose. That is the whole design here,
//! and it changes what "failure" means: a rambling paragraph where a node list
//! should be is not a diagram with problems, it is not a diagram. So this module
//! **validates the shape and refuses**, rather than rendering something
//! half-parsed and letting the author work out that the picture is wrong.
//!
//! The three rules everything below follows:
//!
//! 1. **Strict JSON in, or nothing.** Models wrap JSON in prose and in code
//!    fences constantly, so the fence is stripped and the first balanced object
//!    is taken — but if what comes out is not a graph, it is an error with the
//!    model's actual reply attached, not an empty canvas.
//! 2. **Every node must trace to the author's own project.** A node naming a
//!    document that does not exist is dropped and counted, because the point of
//!    a mind map over a manuscript is that it is about *that* manuscript.
//! 3. **Never silently partial.** What was dropped is reported.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{err, AppResult};
use crate::model::DocumentRow;
use crate::{ollama, project};

/// The shapes a diagram can take. Each has a different prompt and a different
/// idea of what an edge means, so they are not one function with a flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramKind {
    /// Themes and threads radiating from a centre.
    MindMap,
    /// Ordered steps with branches, in the order the book tells them.
    Flowchart,
    /// The same events in the order they happened.
    ///
    /// A separate kind rather than a flag: a story told out of order has two
    /// genuinely different shapes, and the model is given the story dates the
    /// author already wrote rather than being asked to infer a chronology.
    ChronoFlowchart,
    /// A reference sheet: a family tree, a magic system, location legends.
    Infographic,
}

impl DiagramKind {
    pub fn parse(s: &str) -> AppResult<Self> {
        match s {
            "mind_map" => Ok(DiagramKind::MindMap),
            "flowchart" => Ok(DiagramKind::Flowchart),
            "chrono_flowchart" => Ok(DiagramKind::ChronoFlowchart),
            "infographic" => Ok(DiagramKind::Infographic),
            other => err(format!("there is no diagram kind called `{other}`")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub label: String,
    /// A short kind the renderer colours by — "theme", "character", "scene".
    #[serde(default)]
    pub group: Option<String>,
    /// The binder document this node is about, when it names one. Resolved
    /// against the real binder, never taken from the model's word for it.
    #[serde(default)]
    pub document_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagram {
    pub kind: DiagramKind,
    pub title: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// The model that produced it, so a reader knows this is a reading of the
    /// book rather than a measurement of it.
    pub model: String,
    pub generated_at: i64,
    /// Nodes and edges thrown away in validation, each with the reason. Empty
    /// is the good case; non-empty is information, never a silent loss.
    pub dropped: Vec<String>,
}

/// What the model is asked to return, before any validation.
#[derive(Deserialize)]
struct RawGraph {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    nodes: Vec<RawNode>,
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct RawNode {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    document_id: Option<String>,
    /// Some models answer with the document title instead of its id. Accept it
    /// and resolve it, rather than dropping a node that is basically right.
    #[serde(default)]
    document: Option<String>,
}

/// Pull the first balanced JSON object out of a model's reply.
///
/// Models wrap JSON in ```json fences and in sentences like "Here is the graph:"
/// no matter how firmly the prompt says not to. Refusing those replies would
/// mean refusing most correct answers, so the object is extracted — but only an
/// object: anything else still fails loudly.
pub fn extract_json(reply: &str) -> Option<&str> {
    let text = reply.trim();
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn slug(text: &str) -> String {
    project::slugify(text)
}

/// Turn a model's reply into a diagram, or fail with the reply attached.
pub fn validate(
    kind: DiagramKind,
    reply: &str,
    docs: &[DocumentRow],
    model: &str,
    now: i64,
) -> AppResult<Diagram> {
    let Some(json) = extract_json(reply) else {
        return err(format!(
            "{model} did not return a diagram. It replied: {}",
            snippet(reply)
        ));
    };
    let raw: RawGraph = serde_json::from_str(json).map_err(|e| {
        crate::error::AppError(format!(
            "{model} returned something that is not a graph ({e}). It replied: {}",
            snippet(reply)
        ))
    })?;

    let by_id: HashSet<&str> = docs.iter().map(|d| d.id.as_str()).collect();
    let by_title: HashMap<String, &str> =
        docs.iter().map(|d| (d.title.to_lowercase(), d.id.as_str())).collect();

    let mut dropped = Vec::new();
    let mut nodes: Vec<Node> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for raw_node in raw.nodes {
        let label = raw_node.label.or_else(|| raw_node.id.clone()).unwrap_or_default();
        let label = label.trim().to_string();
        if label.is_empty() {
            dropped.push("a node with no label".to_string());
            continue;
        }
        let id = raw_node.id.filter(|i| !i.trim().is_empty()).unwrap_or_else(|| slug(&label));
        if !seen.insert(id.clone()) {
            dropped.push(format!("a second node with the id `{id}`"));
            continue;
        }

        // A document reference is resolved against the real binder, by id or by
        // title. A model naming a document that is not there is the failure this
        // check exists for.
        let document_id = match raw_node.document_id.or(raw_node.document) {
            None => None,
            Some(reference) => {
                let reference = reference.trim();
                if by_id.contains(reference) {
                    Some(reference.to_string())
                } else if let Some(found) = by_title.get(&reference.to_lowercase()) {
                    Some(found.to_string())
                } else {
                    dropped.push(format!(
                        "`{label}` pointed at a document that is not in this project ({reference})"
                    ));
                    None
                }
            }
        };

        nodes.push(Node { id, label, group: raw_node.group, document_id });
    }

    if nodes.is_empty() {
        return err(format!(
            "{model} returned a diagram with no usable nodes. It replied: {}",
            snippet(reply)
        ));
    }

    // An edge to a node that does not exist would render as a line into
    // nothing, so it is dropped and named.
    let ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let mut edges = Vec::new();
    for edge in raw.edges {
        if !ids.contains(edge.from.as_str()) || !ids.contains(edge.to.as_str()) {
            dropped.push(format!("an edge between nodes that do not exist ({} → {})", edge.from, edge.to));
            continue;
        }
        if edge.from == edge.to {
            dropped.push(format!("an edge from `{}` to itself", edge.from));
            continue;
        }
        edges.push(edge);
    }

    Ok(Diagram {
        kind,
        title: raw.title.unwrap_or_else(|| default_title(kind)).trim().to_string(),
        nodes,
        edges,
        model: model.to_string(),
        generated_at: now,
        dropped,
    })
}

fn default_title(kind: DiagramKind) -> String {
    match kind {
        DiagramKind::MindMap => "Mind map",
        DiagramKind::Flowchart => "Flowchart",
        DiagramKind::ChronoFlowchart => "Chronology",
        DiagramKind::Infographic => "Reference sheet",
    }
    .to_string()
}

fn snippet(reply: &str) -> String {
    let trimmed = reply.trim();
    if trimmed.is_empty() {
        return "nothing at all".to_string();
    }
    let short: String = trimmed.chars().take(200).collect();
    format!("\"{short}\"")
}

// ---------------------------------------------------------------------------
// Prompting
// ---------------------------------------------------------------------------

const SHAPE: &str = r#"Return ONE JSON object and nothing else. No prose, no code fence.
{"title": "...", "nodes": [{"id": "short-slug", "label": "...", "group": "...", "document_id": "..."}], "edges": [{"from": "id", "to": "id", "label": "..."}]}
Rules: every edge's from and to must be a node id you listed. Use document_id only when the node is about one of the documents listed below, copying its id exactly. Between 4 and 24 nodes."#;

pub fn prompt(kind: DiagramKind, query: &str, docs: &[DocumentRow]) -> String {
    // A chronological flowchart is given the scenes in story-time order, taken
    // from the dates the author wrote in their notes. The model is not asked to
    // work out what happened when; the project already knows, and a model
    // guessing a chronology would be inventing one.
    let ordered: Vec<DocumentRow> = if kind == DiagramKind::ChronoFlowchart {
        let timeline = crate::analysis::timeline(docs, true);
        let by_id: std::collections::HashMap<&str, &DocumentRow> =
            docs.iter().map(|d| (d.id.as_str(), d)).collect();
        timeline
            .iter()
            .filter_map(|t| by_id.get(t.document_id.as_str()).map(|d| (*d).clone()))
            .collect()
    } else {
        docs.to_vec()
    };

    let dated: std::collections::HashMap<String, String> = if kind == DiagramKind::ChronoFlowchart {
        crate::analysis::timeline(docs, true)
            .into_iter()
            .filter_map(|t| t.story_day.map(|d| (t.document_id, format!("day {d}"))))
            .collect()
    } else {
        std::collections::HashMap::new()
    };

    let catalogue: String = ordered
        .iter()
        .filter(|d| d.kind != crate::model::DocKind::Folder)
        .take(80)
        .map(|d| {
            let when = dated
                .get(&d.id)
                .map(|w| format!(" | when: {w}"))
                .unwrap_or_default();
            format!(
                "- {} | {} | {}{}\n",
                d.id,
                d.title,
                d.synopsis.clone().unwrap_or_else(|| "no synopsis".into()),
                when
            )
        })
        .collect();

    let instruction = match kind {
        DiagramKind::MindMap => {
            "Build a mind map. One central node for the subject, branches for the themes and \
             threads around it, and leaves for the scenes where each appears. `group` should be \
             \"theme\", \"character\", \"place\" or \"scene\"."
        }
        DiagramKind::Flowchart => {
            "Build a flowchart of what happens and what follows from it, in the order the book \
             tells it. Edges run in causal order and their labels say why one thing leads to the \
             next. `group` should be \"event\", \"decision\" or \"outcome\"."
        }
        DiagramKind::ChronoFlowchart => {
            "Build a flowchart of what happens in the order it HAPPENED, not the order the book \
             tells it. The documents below are already listed in story-time order and carry their \
             dates — follow that order and do not re-derive it. Where the book tells something out \
             of sequence, the chart still runs forwards in time. `group` should be \"event\", \
             \"decision\" or \"outcome\"."
        }
        DiagramKind::Infographic => {
            "Build a reference sheet — a family tree, a system's rules, a set of places and how \
             they relate. Edges are relationships and their labels name them. `group` should be \
             the sort of thing the node is."
        }
    };

    format!(
        "{instruction}\n\nSubject: {query}\n\nDocuments in this project (id | title | synopsis):\n{catalogue}\n{SHAPE}"
    )
}

/// Ask a model for a diagram about `query`, and validate what comes back.
pub fn generate(
    root: &std::path::Path,
    model: &str,
    kind: DiagramKind,
    query: &str,
    allow_cloud: bool,
) -> AppResult<Diagram> {
    if query.trim().is_empty() {
        return err("a diagram needs a subject — what should it be about?");
    }
    let docs = project::documents(root)?;
    let reply = ollama::generate_guarded(
        model,
        &prompt(kind, query, &docs),
        Some("You return JSON graphs and nothing else. Never explain, never apologise, never use a code fence."),
        1400,
        // Low, not zero: a diagram of a novel is a reading, but the *shape* has
        // to be reliable, and high temperature is where malformed JSON comes from.
        0.3,
        allow_cloud,
    )?;
    validate(kind, &reply, &docs, model, project::now_ms())
}

/// Save a diagram into the binder as a document the author owns.
///
/// Written as Markdown with a Mermaid block, not as a private binary: it opens
/// in any editor, renders on GitHub, and survives this app being uninstalled.
pub fn save_to_binder(root: &std::path::Path, diagram: &Diagram) -> AppResult<DocumentRow> {
    let docs = project::documents(root)?;
    let parent = docs
        .iter()
        .find(|d| d.kind == crate::model::DocKind::Folder && d.title == "Worldbuilding")
        .or_else(|| {
            docs.iter()
                .find(|d| d.kind == crate::model::DocKind::Folder && d.title == "Research")
        })
        .map(|d| d.id.clone());

    let doc = project::create_document(
        root,
        parent.as_deref(),
        &diagram.title,
        crate::model::DocKind::Lore,
    )?;
    project::write_body(root, &doc.id, &to_markdown(diagram))?;
    project::documents(root)?
        .into_iter()
        .find(|d| d.id == doc.id)
        .ok_or_else(|| crate::error::AppError("the diagram vanished after being saved".into()))
}

/// Mermaid, because it is text an author can edit and a dozen tools can render.
pub fn to_markdown(diagram: &Diagram) -> String {
    let safe = |text: &str| text.replace(['"', '\n'], " ");
    let mut out = format!("# {}\n\n", diagram.title);
    out.push_str("```mermaid\n");
    out.push_str(match diagram.kind {
        DiagramKind::Flowchart | DiagramKind::ChronoFlowchart => "flowchart TD\n",
        _ => "graph TD\n",
    });
    for node in &diagram.nodes {
        out.push_str(&format!("  {}[\"{}\"]\n", slug(&node.id), safe(&node.label)));
    }
    for edge in &diagram.edges {
        match &edge.label {
            Some(label) if !label.trim().is_empty() => out.push_str(&format!(
                "  {} -->|{}| {}\n",
                slug(&edge.from),
                safe(label),
                slug(&edge.to)
            )),
            _ => out.push_str(&format!("  {} --> {}\n", slug(&edge.from), slug(&edge.to))),
        }
    }
    out.push_str("```\n\n");
    out.push_str(&format!(
        "Generated by {} on this machine. This is a model's reading of the book, not a measurement of it — check it before you rely on it.\n",
        diagram.model
    ));
    if !diagram.dropped.is_empty() {
        out.push_str("\nLeft out:\n");
        for reason in &diagram.dropped {
            out.push_str(&format!("- {reason}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DocKind;

    fn fixture_docs() -> Vec<DocumentRow> {
        let base = DocumentRow {
            id: String::new(),
            parent_id: None,
            title: String::new(),
            kind: DocKind::Scene,
            path: Some("manuscript/scenes/x.md".into()),
            position: 0,
            synopsis: None,
            label: None,
            status: None,
            word_count: 100,
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
        };
        vec![
            DocumentRow { id: "doc-1".into(), title: "The Well".into(), ..base.clone() },
            DocumentRow { id: "doc-2".into(), title: "The Descent".into(), ..base },
        ]
    }

    #[test]
    fn json_is_found_inside_the_prose_models_insist_on_adding() {
        assert_eq!(extract_json(r#"{"a":1}"#), Some(r#"{"a":1}"#));
        assert_eq!(
            extract_json("Here is the graph:\n```json\n{\"a\":1}\n```\nHope that helps!"),
            Some(r#"{"a":1}"#)
        );
        // Nesting and braces inside strings must not end the object early.
        assert_eq!(
            extract_json(r#"prefix {"a":{"b":2},"c":"} not the end"} suffix"#),
            Some(r#"{"a":{"b":2},"c":"} not the end"}"#)
        );
        assert_eq!(extract_json(r#"{"a":"\""}"#), Some(r#"{"a":"\""}"#));
        assert_eq!(extract_json("no json at all"), None);
        assert_eq!(extract_json("{unclosed"), None);
    }

    #[test]
    fn a_good_graph_validates_and_keeps_its_document_links() {
        let reply = r#"```json
        {"title":"Water and debt","nodes":[
          {"id":"water","label":"Water","group":"theme"},
          {"id":"well","label":"The Well","group":"scene","document_id":"doc-1"},
          {"id":"descent","label":"The Descent","group":"scene","document":"The Descent"}
        ],"edges":[{"from":"water","to":"well","label":"introduced"},{"from":"water","to":"descent"}]}
        ```"#;
        let d = validate(DiagramKind::MindMap, reply, &fixture_docs(), "test-model", 7).unwrap();
        assert_eq!(d.title, "Water and debt");
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.edges.len(), 2);
        assert!(d.dropped.is_empty(), "{:?}", d.dropped);
        assert_eq!(d.nodes[1].document_id.as_deref(), Some("doc-1"));
        // Resolved by title, because models answer that way.
        assert_eq!(d.nodes[2].document_id.as_deref(), Some("doc-2"));
        assert_eq!(d.generated_at, 7);
        assert_eq!(d.model, "test-model");
    }

    #[test]
    fn prose_where_a_graph_should_be_is_refused_with_what_it_actually_said() {
        let message = validate(
            DiagramKind::MindMap,
            "I'd be happy to help you map the themes of your novel! First, let's consider...",
            &fixture_docs(),
            "chatty:7b",
            0,
        )
        .unwrap_err()
        .to_string();
        assert!(message.contains("did not return a diagram"), "{message}");
        assert!(message.contains("chatty:7b"));
        assert!(message.contains("happy to help"), "the reply must be quoted back: {message}");
    }

    #[test]
    fn an_empty_reply_is_named_as_empty_rather_than_rendered() {
        let message = validate(DiagramKind::Flowchart, "   ", &fixture_docs(), "m", 0)
            .unwrap_err()
            .to_string();
        assert!(message.contains("nothing at all"), "{message}");
    }

    #[test]
    fn a_graph_with_no_usable_nodes_is_refused_not_rendered_empty() {
        let message = validate(
            DiagramKind::MindMap,
            r#"{"title":"x","nodes":[],"edges":[]}"#,
            &fixture_docs(),
            "m",
            0,
        )
        .unwrap_err()
        .to_string();
        assert!(message.contains("no usable nodes"), "{message}");
    }

    #[test]
    fn an_invented_document_is_dropped_and_reported() {
        let reply = r#"{"nodes":[
          {"id":"a","label":"Real","document_id":"doc-1"},
          {"id":"b","label":"Invented","document_id":"doc-999"}
        ],"edges":[]}"#;
        let d = validate(DiagramKind::MindMap, reply, &fixture_docs(), "m", 0).unwrap();
        assert_eq!(d.nodes.len(), 2, "the node survives; only its false claim is dropped");
        assert_eq!(d.nodes[1].document_id, None);
        assert!(
            d.dropped.iter().any(|r| r.contains("doc-999")),
            "the author must be told: {:?}",
            d.dropped
        );
    }

    #[test]
    fn edges_into_nothing_are_dropped_and_reported() {
        let reply = r#"{"nodes":[{"id":"a","label":"A"},{"id":"b","label":"B"}],
          "edges":[{"from":"a","to":"b"},{"from":"a","to":"ghost"},{"from":"a","to":"a"}]}"#;
        let d = validate(DiagramKind::Flowchart, reply, &fixture_docs(), "m", 0).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.dropped.len(), 2);
        assert!(d.dropped.iter().any(|r| r.contains("ghost")));
        assert!(d.dropped.iter().any(|r| r.contains("to itself")));
    }

    #[test]
    fn a_node_without_an_id_gets_one_and_duplicates_are_refused() {
        let reply = r#"{"nodes":[
          {"label":"The Rim at Dusk"},
          {"id":"x","label":"One"},
          {"id":"x","label":"Two"},
          {"label":"   "}
        ],"edges":[]}"#;
        let d = validate(DiagramKind::MindMap, reply, &fixture_docs(), "m", 0).unwrap();
        assert_eq!(d.nodes[0].id, "the-rim-at-dusk");
        assert_eq!(d.nodes.len(), 2);
        assert!(d.dropped.iter().any(|r| r.contains("second node")));
        assert!(d.dropped.iter().any(|r| r.contains("no label")));
    }

    #[test]
    fn the_markdown_is_something_another_tool_can_render() {
        let reply = r#"{"title":"Threads","nodes":[
          {"id":"a","label":"Water \"scarcity\""},{"id":"b","label":"Debt"}
        ],"edges":[{"from":"a","to":"b","label":"drives"}]}"#;
        let d = validate(DiagramKind::Flowchart, reply, &fixture_docs(), "llama3.2:3b", 0).unwrap();
        let md = to_markdown(&d);

        assert!(md.starts_with("# Threads"));
        assert!(md.contains("```mermaid"));
        assert!(md.contains("flowchart TD"));
        assert!(md.contains("a -->|drives| b"));
        // A quote in a label would break the Mermaid node, so it is stripped.
        assert!(!md.contains(r#"["Water "scarcity""]"#));
        // And it says whose reading this is.
        assert!(md.contains("llama3.2:3b"));
        assert!(md.contains("not a measurement"));
    }

    #[test]
    fn the_prompt_carries_the_real_binder_and_asks_for_one_object() {
        let p = prompt(DiagramKind::MindMap, "water", &fixture_docs());
        assert!(p.contains("doc-1 | The Well"));
        assert!(p.contains("Subject: water"));
        assert!(p.contains("Return ONE JSON object"));
        assert!(p.contains("mind map"));
        assert!(!p.contains("flowchart of what happens"), "one kind's prompt, not all three");
    }

    #[test]
    fn kinds_parse_and_an_unknown_one_is_refused() {
        assert_eq!(DiagramKind::parse("mind_map").unwrap(), DiagramKind::MindMap);
        assert_eq!(DiagramKind::parse("flowchart").unwrap(), DiagramKind::Flowchart);
        assert_eq!(DiagramKind::parse("infographic").unwrap(), DiagramKind::Infographic);
        assert!(DiagramKind::parse("pie_chart").is_err());
    }

    #[test]
    fn a_saved_diagram_is_a_document_the_author_owns() {
        let base = std::env::temp_dir().join(format!("aegis-diagram-{}", std::process::id()));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = project::create(&base, "Diagrams", "novel").unwrap();
        let root = std::path::Path::new(&summary.path);

        let d = validate(
            DiagramKind::MindMap,
            r#"{"title":"Threads","nodes":[{"id":"a","label":"Water"}],"edges":[]}"#,
            &project::documents(root).unwrap(),
            "m",
            0,
        )
        .unwrap();
        let doc = save_to_binder(root, &d).unwrap();

        assert_eq!(doc.title, "Threads");
        // It lands in Worldbuilding, where lore belongs.
        let docs = project::documents(root).unwrap();
        let parent = docs.iter().find(|p| Some(&p.id) == doc.parent_id.as_ref()).unwrap();
        assert_eq!(parent.title, "Worldbuilding");

        // And it is a real Markdown file on disk.
        let rel = doc.path.clone().unwrap();
        let body = std::fs::read_to_string(root.join(&rel)).unwrap();
        assert!(body.contains("```mermaid"));

        std::fs::remove_dir_all(&base).ok();
    }

    /// Live: does an installed model actually return a usable graph? Skips loudly.
    #[test]
    fn live_a_model_returns_a_graph_this_build_can_read() {
        let status = ollama::status();
        if !status.reachable {
            eprintln!("SKIPPED live_diagram: Ollama is not running");
            return;
        }
        let models = ollama::generation_models(&status);
        let Some(model) = models.first() else {
            eprintln!("SKIPPED live_diagram: no local generation model");
            return;
        };

        let base = std::env::temp_dir().join(format!("aegis-diagram-live-{}", std::process::id()));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = project::create(&base, "Live", "novel").unwrap();
        let root = std::path::Path::new(&summary.path);
        let scene =
            project::create_document(root, None, "The Well at Kharsa", DocKind::Scene).unwrap();
        project::write_body(root, &scene.id, "The well was going dry, and the town knew it.").unwrap();
        project::update_meta(root, &scene.id, Some("Tamsin buys water."), None, None).unwrap();

        match generate(root, &model.name, DiagramKind::MindMap, "water and debt", false) {
            Ok(d) => {
                eprintln!(
                    "EVIDENCE live_diagram [{}]: \"{}\", {} nodes, {} edges, {} dropped",
                    model.name,
                    d.title,
                    d.nodes.len(),
                    d.edges.len(),
                    d.dropped.len()
                );
                for reason in &d.dropped {
                    eprintln!("EVIDENCE   dropped: {reason}");
                }
            }
            // A small model failing to hold a JSON shape is a fact about that
            // model. The point of this test is to record which, not to assert
            // that every model can.
            Err(e) => eprintln!("EVIDENCE live_diagram refused {}: {e}", model.name),
        }
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_chronological_flowchart_is_given_story_order_not_asked_to_infer_it() {
        // Told out of order: the flashback is second in the book, first in time.
        let mut docs = fixture_docs();
        docs[0].position = 0;
        docs[0].notes = Some("Takes place on Day 40.".into());
        docs[1].position = 1;
        docs[1].notes = Some("Takes place on Day 3.".into());

        let told = prompt(DiagramKind::Flowchart, "how it unfolds", &docs);
        let happened = prompt(DiagramKind::ChronoFlowchart, "how it unfolds", &docs);

        // Narrative order follows the binder.
        assert!(
            told.find("The Well").unwrap() < told.find("The Descent").unwrap(),
            "narrative order is binder order"
        );
        // Story order follows the dates the author wrote.
        assert!(
            happened.find("The Descent").unwrap() < happened.find("The Well").unwrap(),
            "the earlier day comes first, whatever the binder says"
        );
        assert!(happened.contains("day 3"), "the dates travel with the scenes: {happened}");
        assert!(
            happened.contains("order it HAPPENED"),
            "the instruction must say which order it means"
        );
        assert!(
            happened.contains("do not re-derive it"),
            "a model asked to infer a chronology would invent one"
        );
        // And the narrative prompt carries no dates, because it does not need them.
        assert!(!told.contains("when:"));
    }

    #[test]
    fn the_chronological_kind_parses_and_renders_as_a_flowchart() {
        assert_eq!(
            DiagramKind::parse("chrono_flowchart").unwrap(),
            DiagramKind::ChronoFlowchart
        );
        let d = validate(
            DiagramKind::ChronoFlowchart,
            r#"{"nodes":[{"id":"a","label":"Day 3"},{"id":"b","label":"Day 40"}],
                "edges":[{"from":"a","to":"b","label":"then"}]}"#,
            &fixture_docs(),
            "m",
            0,
        )
        .unwrap();
        assert_eq!(d.title, "Chronology");
        let md = to_markdown(&d);
        assert!(md.contains("flowchart TD"), "it draws as a flowchart: {md}");
        assert!(md.contains("a -->|then| b"));
    }

}
