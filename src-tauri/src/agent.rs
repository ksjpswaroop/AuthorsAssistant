//! Agent tooling (Phase 5).
//!
//! Two halves. `dispatch` is the tool gateway: it maps a tool name and a JSON
//! argument object onto the same Rust commands the UI uses, so an agent can
//! never reach a capability the app does not already have. `status` reports
//! whether a hermes-agent runtime is listening.
//!
//! Guardrails live here rather than in the runtime, because the runtime is a
//! separate process that could be replaced. Anything destructive or networked
//! requires `confirmed: true` in its arguments; a model cannot talk its way
//! past this, since the flag is set by the UI after asking a human.

use crate::error::{err, AppResult};
use crate::model::{AgentStatus, DocKind, ToolResult};
use serde_json::{json, Value};
use std::path::Path;

pub const DEFAULT_HOST: &str = "http://127.0.0.1:18765";

/// Tools that mutate or reach the network. Each needs explicit confirmation.
pub const GUARDED: &[&str] =
    &["delete_document", "compile_manuscript", "search_web", "restore_snapshot", "canvas_to_binder"];

/// Every tool the gateway exposes, with a one-line description. This is the
/// catalogue an agent is given; nothing outside it is reachable.
pub fn catalogue() -> Vec<(&'static str, &'static str)> {
    vec![
        ("list_documents", "List every binder document with its metadata."),
        ("read_document", "Read one document's Markdown body."),
        ("create_folder", "Create a binder folder."),
        ("create_chapter", "Create one or more empty scenes inside a folder."),
        ("create_character_sheet", "Create a character sheet from the template."),
        ("create_location_sheet", "Create a location sheet from the template."),
        ("save_to_binder", "Write content into a new document under a category."),
        ("search_binder", "Find documents whose prose overlaps a query."),
        ("set_metadata", "Set synopsis, label or status on a document."),
        ("bulk_set_metadata", "Set fields across many documents at once."),
        ("append_canon_entry", "Append a falsifiable fact plus its source to a canon document."),
        ("check_internal_similarity", "Report passages reused from elsewhere in the project."),
        ("snapshot_project", "Commit a snapshot of the whole project."),
        ("delete_document", "Move a document to the project trash. Requires confirmation."),
        (
            "search_web",
            "Check whether one phrase already exists on the web, through the author's own wigolo daemon. \
             Sends that phrase off the machine. Requires confirmation.",
        ),
        (
            "compile_manuscript",
            "Compile the binder to a file in a chosen format. Overwrites the target. Requires confirmation.",
        ),
        (
            "extract_entities",
            "List the characters, places and dates a document mentions, matched against the \
             project's own sheets.",
        ),
        (
            "check_against_canon",
            "Find which established facts a document touches, so a contradiction can be read. \
             Retrieval, not a verdict.",
        ),
        (
            "check_foreshadowing_ledger",
            "Report plants with no payoff, and payoffs with nothing planted.",
        ),
        (
            "log_propagation_debt",
            "Write down a gap in the world the book has not established. Use this instead of \
             inventing a fact to fill it.",
        ),
        (
            "search_sources",
            "Search the author's registered research sources for passages about a query. Local, \
             no network.",
        ),
        (
            "list_claims",
            "List the checkable claims extracted from the research corpus, with the source each \
             came from.",
        ),
        (
            "cite_claim",
            "Record that a manuscript paragraph rests on a claim, so a retracted source can name \
             what depends on it.",
        ),
        (
            "knowledge_gaps",
            "Report where the research corpus is thin: concepts with no support, sources never \
             mined, claims nothing cites.",
        ),
        (
            "generate_diagram",
            "Generate a mind map, flowchart or reference sheet about a subject, from this \
             project's own documents, and save it into the binder as Markdown.",
        ),
        (
            "canvas_plan",
            "Show what reordering a folder to match the freeform canvas layout would change. \
             Changes nothing.",
        ),
        (
            "canvas_to_binder",
            "Apply the canvas layout as the binder order for a folder. Use canvas_plan first. \
             Requires confirmation.",
        ),
    ]
}

fn arg<'a>(args: &'a Value, key: &str) -> AppResult<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| crate::error::AppError(format!("the tool needs a `{key}` argument")))
}

/// An optional string argument, with an empty string treated as absent.
///
/// Not a nicety. `llama3.2:3b` sends `"parent_id": ""` rather than omitting the
/// field, and passing that straight through made SQLite answer `FOREIGN KEY
/// constraint failed` — an error about the author's database for what was
/// really a model filling in a blank. Models do this; the gateway absorbs it.
fn opt(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn confirmed(args: &Value) -> bool {
    args.get("confirmed").and_then(Value::as_bool).unwrap_or(false)
}

/// The JSON Schema for one tool's arguments.
///
/// A catalogue line tells a model a tool exists; a schema is what lets it
/// actually call one. These are hand-written against what `dispatch` reads —
/// a test walks the catalogue and fails if the two ever drift apart, because a
/// schema that lies is worse than no schema at all.
pub fn schema(tool: &str) -> Value {
    // Every guarded tool carries `confirmed` in its schema so the model can see
    // the gate exists. Sending it does not open the gate: the value that
    // actually reaches `dispatch` is decided by a person, never by the model.
    let obj = |props: Value, required: &[&str]| {
        json!({ "type": "object", "properties": props, "required": required })
    };
    let guarded = |props: Value, required: &[&str]| {
        let mut all = props.as_object().cloned().unwrap_or_default();
        all.insert(
            "confirmed".into(),
            json!({
                "type": "boolean",
                "description": "Only a person can set this. Asking for the call is enough; do not assert it."
            }),
        );
        json!({ "type": "object", "properties": all, "required": required })
    };

    match tool {
        "list_documents" => obj(json!({}), &[]),
        "read_document" => obj(
            json!({ "id": { "type": "string", "description": "Binder document id." } }),
            &["id"],
        ),
        "create_folder" => obj(
            json!({
                "name": { "type": "string" },
                "parent_id": { "type": "string", "description": "Folder to create it inside. Omit for the top level." }
            }),
            &["name"],
        ),
        "create_chapter" => obj(
            json!({
                "parent_id": { "type": "string" },
                "count": { "type": "integer", "description": "How many empty scenes, 1 to 50." },
                "title_prefix": { "type": "string", "description": "Each scene is named \"<prefix> N\". Default \"Chapter\"." }
            }),
            &[],
        ),
        "create_character_sheet" | "create_location_sheet" => obj(
            json!({
                "name": { "type": "string" },
                "scene_id": { "type": "string", "description": "Optional scene to link the sheet to." }
            }),
            &["name"],
        ),
        "save_to_binder" => obj(
            json!({
                "title": { "type": "string" },
                "content": { "type": "string", "description": "Markdown body." },
                "category": {
                    "type": "string",
                    "enum": ["Research", "Worldbuilding", "Canon", "Characters"],
                    "description": "Which folder it belongs in. Default Research."
                }
            }),
            &["title", "content"],
        ),
        "search_binder" => obj(
            json!({
                "query": { "type": "string" },
                "limit": { "type": "integer", "description": "1 to 50. Default 5." }
            }),
            &["query"],
        ),
        "set_metadata" => obj(
            json!({
                "id": { "type": "string" },
                "synopsis": { "type": "string" },
                "label": { "type": "string" },
                "status": { "type": "string", "enum": ["Empty", "Draft", "Done"] }
            }),
            &["id"],
        ),
        "bulk_set_metadata" => obj(
            json!({
                "document_ids": { "type": "array", "items": { "type": "string" } },
                "status": { "type": "string" },
                "label": { "type": "string" },
                "section_type": { "type": "string" },
                "pov": { "type": "string" }
            }),
            &["document_ids"],
        ),
        "append_canon_entry" => obj(
            json!({
                "id": { "type": "string", "description": "The canon document to append to." },
                "fact": { "type": "string", "description": "One falsifiable statement." },
                "source": { "type": "string", "description": "The document that establishes it. An entry without a source is refused." }
            }),
            &["id", "fact", "source"],
        ),
        "check_internal_similarity" => obj(
            json!({
                "id": { "type": "string" },
                "threshold": { "type": "number", "description": "0 to 1. Default 0.5." }
            }),
            &["id"],
        ),
        "snapshot_project" => obj(json!({ "message": { "type": "string" } }), &[]),
        "delete_document" => guarded(json!({ "id": { "type": "string" } }), &["id"]),
        "search_web" => guarded(
            json!({
                "phrase": { "type": "string", "description": "One phrase, 60 words at most. It leaves the machine." },
                "max_results": { "type": "integer" }
            }),
            &["phrase"],
        ),
        "extract_entities" => obj(
            json!({ "id": { "type": "string", "description": "The document to read." } }),
            &["id"],
        ),
        "check_against_canon" => obj(
            json!({ "id": { "type": "string" } }),
            &["id"],
        ),
        "check_foreshadowing_ledger" => obj(json!({}), &[]),
        "log_propagation_debt" => obj(
            json!({
                "gap": { "type": "string", "description": "What the book needs and has never established." },
                "found_in": { "type": "string", "description": "The document that exposed it." }
            }),
            &["gap"],
        ),
        "search_sources" => obj(
            json!({
                "query": { "type": "string" },
                "limit": { "type": "integer", "description": "1 to 20. Default 5." }
            }),
            &["query"],
        ),
        "list_claims" => obj(
            json!({ "source_id": { "type": "string", "description": "Omit for every source." } }),
            &[],
        ),
        "cite_claim" => obj(
            json!({
                "block_id": { "type": "string", "description": "The paragraph, from search_sources or the binder." },
                "claim_id": { "type": "string" }
            }),
            &["block_id", "claim_id"],
        ),
        "knowledge_gaps" => obj(json!({}), &[]),
        "generate_diagram" => obj(
            json!({
                "kind": { "type": "string", "enum": ["mind_map", "flowchart", "infographic"] },
                "query": { "type": "string", "description": "What the diagram should be about." },
                "model": { "type": "string", "description": "Which installed model to ask. Omit for the first available." }
            }),
            &["kind", "query"],
        ),
        "canvas_plan" => obj(
            json!({
                "parent_id": { "type": "string", "description": "The folder to inspect. Omit for the top level." }
            }),
            &[],
        ),
        "canvas_to_binder" => guarded(
            json!({
                "parent_id": { "type": "string", "description": "The folder to reorder. Omit for the top level." }
            }),
            &[],
        ),
        "compile_manuscript" => guarded(
            json!({
                "path": { "type": "string", "description": "Absolute path to write. Overwrites what is there." },
                "preset": { "type": "string" },
                "format": { "type": "string", "enum": ["markdown", "html", "pdf", "epub", "docx", "fountain"] }
            }),
            &["path"],
        ),
        _ => obj(json!({}), &[]),
    }
}

/// Runs one tool call against a project. Returns JSON the agent can read back.
///
/// Attributes any prose written to the author. Use [`dispatch_as`] from the
/// agent loop, where the writer is a model.
pub fn dispatch(root: &Path, tool: &str, args: &Value) -> AppResult<ToolResult> {
    dispatch_as(root, tool, args, crate::provenance::Actor::human())
}

/// Runs one tool call, recording `actor` as the author of anything it writes.
///
/// This parameter is the difference between a provenance ledger and a lie. A
/// model's `save_to_binder` attributed to `Actor::human()` would record AI
/// prose as the author's own work, and a ledger people trust that is quietly
/// wrong is worse than no ledger at all. `NEG-14` and `UF-06` guard it.
pub fn dispatch_as(
    root: &Path,
    tool: &str,
    args: &Value,
    actor: crate::provenance::Actor,
) -> AppResult<ToolResult> {
    if GUARDED.contains(&tool) && !confirmed(args) {
        return err(format!(
            "`{tool}` changes the author's work or leaves the machine. It needs confirmation, \
             which only a person can give."
        ));
    }

    let value = match tool {
        "list_documents" => {
            let docs = crate::project::documents(root)?;
            json!({ "count": docs.len(), "documents": docs })
        }
        "read_document" => {
            let id = arg(args, "id")?;
            json!({ "id": id, "body": crate::project::read_body(root, id)? })
        }
        "create_folder" => {
            let name = arg(args, "name")?;
            let parent = opt(args, "parent_id");
            let doc = crate::project::create_document(root, parent.as_deref(), name, DocKind::Folder)?;
            json!({ "created": doc })
        }
        "create_chapter" => {
            let parent = opt(args, "parent_id");
            let count = args.get("count").and_then(Value::as_i64).unwrap_or(1).clamp(1, 50);
            let prefix = opt(args, "title_prefix").unwrap_or_else(|| "Chapter".to_string());
            let mut made = Vec::new();
            for i in 1..=count {
                made.push(crate::project::create_document(root, parent.as_deref(), &format!("{prefix} {i}"), DocKind::Scene)?);
            }
            json!({ "created": made })
        }
        "create_character_sheet" | "create_location_sheet" => {
            let name = arg(args, "name")?;
            let kind = if tool.starts_with("create_character") { DocKind::Sheet } else { DocKind::Location };
            let link = opt(args, "scene_id");
            json!({ "created": crate::project::create_sheet(root, name, kind, None, link.as_deref())? })
        }
        "save_to_binder" => {
            let title = arg(args, "title")?;
            let content = arg(args, "content")?;
            let category = opt(args, "category").unwrap_or_else(|| "Research".to_string());
            let category = category.as_str();
            let docs = crate::project::documents(root)?;
            let parent = docs.iter().find(|d| d.kind == DocKind::Folder && d.title == category).map(|d| d.id.clone());
            let kind = match category {
                "Worldbuilding" => DocKind::Lore,
                "Canon" => DocKind::Canon,
                "Characters" => DocKind::Sheet,
                _ => DocKind::Research,
            };
            let doc = crate::project::create_document(root, parent.as_deref(), title, kind)?;
            crate::project::write_body_as(root, &doc.id, content, actor.clone())?;
            json!({ "created": doc })
        }
        "search_binder" => {
            let query = arg(args, "query")?;
            let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(5).clamp(1, 50) as usize;
            let docs = crate::project::documents(root)?;
            let needle = crate::integrity::shingles(query, 3);
            let mut scored: Vec<(f32, Value)> = Vec::new();
            for d in &docs {
                let Some(_) = d.path.as_deref() else { continue };
                let body = crate::project::read_body(root, &d.id)?;
                let score = crate::integrity::containment(&needle, &crate::integrity::shingles(&body, 3));
                if score > 0.0 {
                    scored.push((score, json!({ "id": d.id, "title": d.title, "score": score })));
                }
            }
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            json!({ "hits": scored.into_iter().take(limit).map(|(_, v)| v).collect::<Vec<_>>() })
        }
        "set_metadata" => {
            let id = arg(args, "id")?;
            let doc = crate::project::update_meta(
                root,
                id,
                opt(args, "synopsis").as_deref(),
                opt(args, "label").as_deref(),
                opt(args, "status").as_deref(),
            )?;
            json!({ "updated": doc })
        }
        "bulk_set_metadata" => {
            let ids: Vec<String> = args
                .get("document_ids")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let n = crate::project::bulk_set(
                root,
                &ids,
                opt(args, "status").as_deref(),
                opt(args, "label").as_deref(),
                opt(args, "section_type").as_deref(),
                opt(args, "pov").as_deref(),
            )?;
            json!({ "changed": n })
        }
        "append_canon_entry" => {
            let id = arg(args, "id")?;
            let fact = arg(args, "fact")?;
            let source = arg(args, "source")?;
            json!({ "saved": crate::project::append_canon_entry(root, id, fact, source)? })
        }
        "check_internal_similarity" => {
            let id = arg(args, "id")?;
            let subject = crate::project::read_body(root, id)?;
            let docs = crate::project::documents(root)?;
            let mut sources = Vec::new();
            for d in &docs {
                if d.id == id || d.path.is_none() {
                    continue;
                }
                sources.push((d.id.clone(), d.title.clone(), crate::project::read_body(root, &d.id)?));
            }
            let threshold = args.get("threshold").and_then(Value::as_f64).unwrap_or(0.5) as f32;
            json!({ "matches": crate::integrity::internal_similarity(id, &subject, &sources, threshold, 5) })
        }
        "snapshot_project" => {
            let message = opt(args, "message").unwrap_or_else(|| "Snapshot before agent changes".to_string());
            let message = message.as_str();
            json!({ "snapshot": crate::snapshots::create(root, message)? })
        }
        "delete_document" => {
            let id = arg(args, "id")?;
            crate::project::delete_document(root, id)?;
            json!({ "deleted": id })
        }
        "search_web" => {
            let phrase = arg(args, "phrase").or_else(|_| arg(args, "query"))?;
            let max = args.get("max_results").and_then(Value::as_i64).unwrap_or(5) as usize;
            let found = crate::websearch::search(phrase, max)?;
            // The trace is written whether or not anything came back: what was
            // sent is the part that has to be recorded.
            crate::websearch::record(
                root,
                &crate::websearch::TraceEntry {
                    at: crate::project::now_ms(),
                    query: found.query.clone(),
                    word_count: found.query.split_whitespace().count() as i64,
                    base: crate::websearch::base(),
                    result_count: found.matches.len() as i64,
                    urls: found.matches.iter().map(|m| m.url.clone()).collect(),
                },
            )?;
            json!({ "search": found })
        }
        "extract_entities" => {
            let id = arg(args, "id")?;
            let docs = crate::project::documents(root)?;
            let passage = crate::project::read_body(root, id)?;
            json!({ "entities": crate::continuity::extract_entities(&passage, &docs) })
        }
        "check_against_canon" => {
            let id = arg(args, "id")?;
            let docs = crate::project::documents(root)?;
            let passage = crate::project::read_body(root, id)?;
            let canon = crate::continuity::read_canon(root, &docs)?;
            json!({ "canon": crate::continuity::check_against_canon(&passage, &canon) })
        }
        "check_foreshadowing_ledger" => {
            let docs = crate::project::documents(root)?;
            json!({ "ledger": crate::continuity::check_foreshadowing_ledger(&docs) })
        }
        "log_propagation_debt" => {
            let gap = arg(args, "gap")?;
            let found_in = opt(args, "found_in").unwrap_or_else(|| "unspecified".into());
            // The actor is threaded so the debt records who noticed it — an
            // author reading their own book and a model reading it are
            // different kinds of claim.
            let by = match actor.kind {
                crate::provenance::ActorType::AiModel => actor.id.clone(),
                _ => "the author".to_string(),
            };
            json!({
                "debt": crate::continuity::log_propagation_debt(
                    root, gap, &found_in, &by, crate::project::now_ms(),
                )?
            })
        }
        "search_sources" => {
            let query = arg(args, "query")?;
            let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(5).clamp(1, 20) as usize;
            json!({ "passages": crate::research::local_passages(root, query, limit)? })
        }
        "list_claims" => {
            let conn = crate::store::open(root)?;
            let source = opt(args, "source_id");
            json!({ "claims": crate::knowledge::claims(&conn, source.as_deref())? })
        }
        "cite_claim" => {
            let block = arg(args, "block_id")?;
            let claim = arg(args, "claim_id")?;
            let conn = crate::store::open(root)?;
            crate::knowledge::cite(&conn, block, claim, crate::project::now_ms())?;
            json!({ "cited": { "block": block, "claim": claim } })
        }
        "knowledge_gaps" => {
            let conn = crate::store::open(root)?;
            json!({ "gaps": crate::knowledge::gaps(&conn)? })
        }
        "generate_diagram" => {
            let kind = crate::diagrams::DiagramKind::parse(arg(args, "kind")?)?;
            let query = arg(args, "query")?;
            let status = crate::ollama::status();
            let model = match opt(args, "model") {
                Some(name) => name,
                None => match crate::ollama::generation_models(&status).first() {
                    Some(m) => m.name.clone(),
                    None => return err("no local model can generate a diagram"),
                },
            };
            let diagram = crate::diagrams::generate(root, &model, kind, query, false)?;
            let saved = crate::diagrams::save_to_binder(root, &diagram)?;
            json!({ "diagram": diagram, "saved": saved })
        }
        "canvas_plan" => {
            let parent = opt(args, "parent_id");
            json!({ "plan": crate::canvas::plan(root, parent.as_deref())? })
        }
        "canvas_to_binder" => {
            let parent = opt(args, "parent_id");
            json!({ "committed": crate::canvas::commit(root, parent.as_deref())? })
        }
        "compile_manuscript" => {
            let preset = opt(args, "preset").unwrap_or_else(|| "Standard Manuscript".to_string());
            let format = opt(args, "format").unwrap_or_else(|| "markdown".to_string());
            let out = arg(args, "path")?;
            let filters = crate::model::CompileFilters { skip_empty: true, ..Default::default() };

            let mut items = Vec::new();
            for doc in crate::project::documents(root)? {
                if !crate::compile::included(&doc, &filters) {
                    continue;
                }
                let body = crate::project::read_body(root, &doc.id)?;
                items.push(crate::compile::Item { doc, body });
            }
            let bytes = crate::compile::render(
                &items,
                &crate::model::CompilePreset::named(&preset),
                crate::model::ExportFormat::parse(&format),
            )?;
            std::fs::write(out, &bytes)?;
            json!({ "path": out, "bytes": bytes.len(), "documents": items.len() })
        }
        other => return err(format!("there is no tool called `{other}`")),
    };

    Ok(ToolResult { tool: tool.to_string(), ok: true, value })
}

/// Is a hermes-agent runtime listening? Not running is a normal state.
pub fn status() -> AgentStatus {
    let host = std::env::var("AEGISDRAFT_HERMES").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    status_at(&host)
}

pub fn status_at(host: &str) -> AgentStatus {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(2)))
        .build()
        .into();
    match agent.get(&format!("{host}/health")).call() {
        Ok(_) => AgentStatus {
            reachable: true,
            host: host.to_string(),
            tools: catalogue().into_iter().map(|(n, _)| n.to_string()).collect(),
            message: None,
        },
        Err(_) => AgentStatus {
            reachable: false,
            host: host.to_string(),
            tools: catalogue().into_iter().map(|(n, _)| n.to_string()).collect(),
            message: Some(
                "No agent runtime is listening. The tools below are ready; hermes-agent is what calls them.".into(),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn temp_project(tag: &str) -> PathBuf {
        let parent = std::env::temp_dir().join(format!(
            "aegis-agent-{tag}-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&parent).unwrap();
        PathBuf::from(&crate::project::create(&parent, "Agent Book", "novel").unwrap().path)
    }

    fn cleanup(root: &Path) {
        std::fs::remove_dir_all(root.parent().unwrap()).ok();
    }

    // --- guardrails --------------------------------------------------------

    #[test]
    fn every_guarded_tool_refuses_without_confirmation() {
        let root = temp_project("guard");
        for tool in GUARDED {
            let r = dispatch(&root, tool, &json!({ "id": "anything" }));
            assert!(r.is_err(), "`{tool}` ran without confirmation");
            let msg = format!("{}", r.unwrap_err());
            assert!(msg.contains("needs confirmation"), "`{tool}` gave an unhelpful refusal: {msg}");
        }
        cleanup(&root);
    }

    #[test]
    fn a_confirmed_delete_is_allowed_and_the_file_is_only_moved() {
        let root = temp_project("delete");
        let doc = crate::project::create_document(&root, None, "Doomed", DocKind::Scene).unwrap();
        crate::project::write_body(&root, &doc.id, "words worth keeping").unwrap();
        let file = root.join(doc.path.as_ref().unwrap());
        assert!(file.exists());

        dispatch(&root, "delete_document", &json!({ "id": doc.id, "confirmed": true })).unwrap();
        assert!(!file.exists(), "the document left its place");
        let trashed: Vec<_> = std::fs::read_dir(root.join("trash")).unwrap().filter_map(Result::ok).collect();
        assert_eq!(trashed.len(), 1, "and went to trash rather than nowhere");
        cleanup(&root);
    }

    #[test]
    fn an_unknown_tool_is_refused_by_name() {
        let root = temp_project("unknown");
        let r = dispatch(&root, "rm_rf", &json!({}));
        assert!(r.is_err());
        assert!(format!("{}", r.unwrap_err()).contains("no tool called `rm_rf`"));
        cleanup(&root);
    }

    #[test]
    fn a_missing_argument_names_the_argument() {
        let root = temp_project("args");
        let r = dispatch(&root, "read_document", &json!({}));
        assert!(format!("{}", r.unwrap_err()).contains("`id`"), "the error must name what is missing");
        cleanup(&root);
    }

    // --- binder tools ------------------------------------------------------

    #[test]
    fn the_app_controller_sequence_from_the_docs_works_end_to_end() {
        // "Create a folder for Act III, add three chapters, and start a sheet
        // for the antagonist" — the example the product docs are built around.
        let root = temp_project("controller");

        let folder = dispatch(&root, "create_folder", &json!({ "name": "Act III" })).unwrap();
        let folder_id = folder.value["created"]["id"].as_str().unwrap().to_string();

        let chapters = dispatch(
            &root,
            "create_chapter",
            &json!({ "parent_id": folder_id, "count": 3, "title_prefix": "Chapter" }),
        )
        .unwrap();
        assert_eq!(chapters.value["created"].as_array().unwrap().len(), 3);

        dispatch(&root, "create_character_sheet", &json!({ "name": "The Antagonist" })).unwrap();

        let docs = crate::project::documents(&root).unwrap();
        assert!(docs.iter().any(|d| d.title == "Act III" && d.kind == DocKind::Folder));
        assert_eq!(docs.iter().filter(|d| d.parent_id.as_deref() == Some(&folder_id)).count(), 3);

        let sheet = docs.iter().find(|d| d.title == "The Antagonist").unwrap();
        assert_eq!(sheet.kind, DocKind::Sheet);
        // The sheet is filed under Characters, not loose at the root.
        let characters = docs.iter().find(|d| d.title == "Characters").unwrap();
        assert_eq!(sheet.parent_id.as_deref(), Some(characters.id.as_str()));
        // And it has the template, not an empty file.
        assert!(crate::project::read_body(&root, &sheet.id).unwrap().contains("Continuity facts"));
        cleanup(&root);
    }

    #[test]
    fn create_chapter_clamps_an_absurd_count() {
        let root = temp_project("clamp");
        let r = dispatch(&root, "create_chapter", &json!({ "count": 10_000 })).unwrap();
        assert_eq!(r.value["created"].as_array().unwrap().len(), 50, "a runaway loop is capped");
        cleanup(&root);
    }

    #[test]
    fn save_to_binder_files_content_under_the_right_category() {
        let root = temp_project("save");
        dispatch(
            &root,
            "save_to_binder",
            &json!({ "category": "Worldbuilding", "title": "The Kharsa Basin", "content": "A salt pan nine days across." }),
        )
        .unwrap();

        let docs = crate::project::documents(&root).unwrap();
        let lore = docs.iter().find(|d| d.title == "The Kharsa Basin").unwrap();
        assert_eq!(lore.kind, DocKind::Lore);
        let world = docs.iter().find(|d| d.title == "Worldbuilding").unwrap();
        assert_eq!(lore.parent_id.as_deref(), Some(world.id.as_str()));
        assert!(crate::project::read_body(&root, &lore.id).unwrap().contains("salt pan"));
        cleanup(&root);
    }

    #[test]
    fn search_binder_finds_the_document_that_contains_the_phrase() {
        let root = temp_project("search");
        let a = crate::project::create_document(&root, None, "A", DocKind::Scene).unwrap();
        let b = crate::project::create_document(&root, None, "B", DocKind::Scene).unwrap();
        crate::project::write_body(&root, &a.id, "The cartographer broke his leg on the descent.").unwrap();
        crate::project::write_body(&root, &b.id, "Tariffs are counted in days, not coins.").unwrap();

        let r = dispatch(&root, "search_binder", &json!({ "query": "broke his leg on the descent" })).unwrap();
        let hits = r.value["hits"].as_array().unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0]["title"], "A", "the document containing the phrase ranks first");
        cleanup(&root);
    }

    #[test]
    fn canon_entries_must_carry_a_source() {
        let root = temp_project("canon");
        let docs = crate::project::documents(&root).unwrap();
        let canon_folder = docs.iter().find(|d| d.title == "Canon").unwrap().id.clone();
        let entry = crate::project::create_document(&root, Some(&canon_folder), "Cast facts", DocKind::Canon).unwrap();

        assert!(dispatch(&root, "append_canon_entry", &json!({ "id": entry.id, "fact": "Tamsin is left-handed" })).is_err(),
                "a fact with no source is not canon");

        dispatch(
            &root,
            "append_canon_entry",
            &json!({ "id": entry.id, "fact": "Tamsin is left-handed", "source": "1. The Well at Kharsa" }),
        )
        .unwrap();
        let body = crate::project::read_body(&root, &entry.id).unwrap();
        assert!(body.contains("left-handed") && body.contains("1. The Well at Kharsa"));
        cleanup(&root);
    }

    #[test]
    fn similarity_through_the_gateway_finds_reuse() {
        let root = temp_project("sim");
        let shared = "The rim of the basin held the last of the heat the way a kiln holds it, all at once.";
        let a = crate::project::create_document(&root, None, "A", DocKind::Scene).unwrap();
        let b = crate::project::create_document(&root, None, "B", DocKind::Scene).unwrap();
        crate::project::write_body(&root, &a.id, shared).unwrap();
        crate::project::write_body(&root, &b.id, &format!("Earlier prose. {shared}")).unwrap();

        let r = dispatch(&root, "check_internal_similarity", &json!({ "id": a.id, "threshold": 0.6 })).unwrap();
        let matches = r.value["matches"].as_array().unwrap();
        assert!(!matches.is_empty(), "the reused paragraph should be found");
        assert_eq!(matches[0]["sourceTitle"], "B");
        cleanup(&root);
    }

    #[test]
    fn bulk_metadata_reports_how_many_rows_it_changed() {
        let root = temp_project("bulk");
        let ids: Vec<String> = (0..3)
            .map(|i| crate::project::create_document(&root, None, &format!("S{i}"), DocKind::Scene).unwrap().id)
            .collect();

        let r = dispatch(&root, "bulk_set_metadata", &json!({ "document_ids": ids, "status": "Draft" })).unwrap();
        assert_eq!(r.value["changed"], 3);

        let docs = crate::project::documents(&root).unwrap();
        assert_eq!(docs.iter().filter(|d| d.status.as_deref() == Some("Draft")).count(), 3);
        cleanup(&root);
    }

    // --- catalogue and runtime --------------------------------------------

    #[test]
    fn every_advertised_tool_actually_dispatches() {
        // The catalogue is what an agent is told it can do. A name in the list
        // that does not resolve would make the agent fail unpredictably.
        let root = temp_project("catalogue");
        for (name, description) in catalogue() {
            assert!(!description.is_empty(), "{name} has no description");
            let r = dispatch(&root, name, &json!({}));
            if let Err(e) = &r {
                let msg = format!("{e}");
                assert!(
                    !msg.contains("no tool called"),
                    "`{name}` is advertised but not implemented"
                );
            }
        }
        cleanup(&root);
    }

    #[test]
    fn agent_status_reports_absence_without_failing() {
        let s = status_at("http://127.0.0.1:1");
        assert!(!s.reachable);
        assert!(s.message.is_some());
        assert!(!s.tools.is_empty(), "the tools exist whether or not a runtime is listening");
        assert!(s.tools.contains(&"create_folder".to_string()));
    }

    #[test]
    fn a_blank_optional_argument_is_treated_as_absent() {
        // Verbatim from a live `llama3.2:3b` run: it filled the optional field
        // in with an empty string instead of leaving it out, and SQLite then
        // reported a foreign-key failure as if the author's project were broken.
        let args = json!({ "name": "Act Two", "parent_id": "" });
        assert_eq!(opt(&args, "parent_id"), None);
        assert_eq!(opt(&args, "name").as_deref(), Some("Act Two"));
        assert_eq!(opt(&json!({ "a": "  " }), "a"), None, "whitespace is blank too");
        assert_eq!(opt(&json!({}), "missing"), None);
        assert_eq!(opt(&json!({ "a": 7 }), "a"), None, "a number is not a string id");

        let base = std::env::temp_dir().join(format!("aegis-blank-{}", std::process::id()));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = crate::project::create(&base, "Blank", "novel").unwrap();
        let root = std::path::PathBuf::from(summary.path);

        let out = dispatch(&root, "create_folder", &args).expect("a blank parent must mean top level");
        assert!(out.ok);
        assert_eq!(out.value["created"]["parentId"], Value::Null);

        std::fs::remove_dir_all(&base).ok();
    }
}
