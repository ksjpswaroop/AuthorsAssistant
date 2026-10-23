//! A real agent loop (Phase 5).
//!
//! For most of this project's life the command bar matched strings: it looked
//! for "folder" and a number, and called two tools. That is a parser wearing an
//! agent's clothes — it cannot recover from a phrasing it has not seen, and it
//! cannot decide anything.
//!
//! Ollama's OpenAI-compatible endpoint (`/v1/chat/completions`) supports native
//! tool calling, and the fourteen-tool gateway in [`crate::agent`] is already
//! built, guarded and tested. So the loop is the real thing: send the catalogue,
//! run the `tool_calls` that come back through `agent::dispatch`, feed the
//! results in as `role: "tool"` messages, and go round again until the model
//! stops calling tools or the hop cap is reached.
//!
//! # The guard does not move
//!
//! [`crate::agent::GUARDED`] still refuses any call without `confirmed: true`,
//! and this loop **never sets that flag itself**. When the model asks for a
//! guarded tool the run *pauses*: it returns the whole conversation so far plus
//! the pending call, and a person answers. [`resume`] carries on from exactly
//! that point with the answer, so approving a delete approves *that* delete —
//! not a re-derived one the model might phrase differently on a second attempt.
//!
//! # Traces
//!
//! Every run is appended to `traces/` in the project as JSON: the prompt, each
//! hop with its arguments and result, and the outcome. The inspector reads
//! these. A local agent whose reasoning is invisible is not meaningfully more
//! trustworthy than a remote one.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::agent;
use crate::error::{err, AppResult};

/// Enough hops for a real multi-step task, few enough that a model stuck in a
/// loop costs seconds rather than minutes.
pub const MAX_HOPS: usize = 12;

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

/// One co-author role: a system prompt and the subset of tools it may reach.
///
/// The tool subset is the real constraint. A World-Builder that cannot call
/// `delete_document` cannot delete a document however it is prompted, which is
/// a stronger guarantee than telling a model not to.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: &'static str,
    pub name: &'static str,
    pub blurb: &'static str,
    pub system: &'static str,
    pub tools: Vec<&'static str>,
}

const SHARED: &str = "You are working inside AegisDraft, an author's own book project on their own \
machine. Use the tools to do the work rather than describing what the author should click. \
Take one step at a time and read each result before the next call. If a tool fails, say what \
failed and stop — do not invent a result or try the same call again unchanged. When the work is \
done, reply with a short plain-English account of what you changed. Never claim to have done \
something you did not do with a tool.";

pub fn profiles() -> Vec<Profile> {
    vec![
        Profile {
            id: "controller",
            name: "App Controller",
            blurb: "Restructures the binder from a sentence — folders, chapters, sheets, metadata.",
            system: "You restructure the author's binder on request. Prefer the fewest calls that \
                     do the job. Never delete anything unless the author asked for a deletion in \
                     so many words.",
            tools: vec![
                "list_documents",
                "search_binder",
                "create_folder",
                "create_chapter",
                "create_character_sheet",
                "create_location_sheet",
                "save_to_binder",
                "set_metadata",
                "bulk_set_metadata",
                "snapshot_project",
                "canvas_plan",
                "canvas_to_binder",
                "delete_document",
            ],
        },
        Profile {
            id: "continuity",
            name: "Deep Continuity",
            blurb: "Reads across the whole book for contradictions, then cites the page it found them on.",
            system: "You look for continuity errors across the whole book. Read the documents \
                     around the one in question before judging anything. Every issue you report \
                     must cite the document it came from and quote the line — an unsourced \
                     continuity claim is worthless to an author. Start with check_against_canon \
                     and check_foreshadowing_ledger; they are cheap and they use facts the author \
                     has already established. If a scene establishes a new hard fact, append it to \
                     canon with its source. If it reveals something the book has never \
                     established, call log_propagation_debt — never invent the missing fact \
                     yourself. Do not rewrite prose.",
            tools: vec![
                "list_documents",
                "read_document",
                "search_binder",
                "extract_entities",
                "check_against_canon",
                "check_foreshadowing_ledger",
                "log_propagation_debt",
                "search_sources",
                "list_claims",
                "cite_claim",
                "append_canon_entry",
                "set_metadata",
            ],
        },
        Profile {
            id: "integrity",
            name: "Integrity",
            blurb: "Runs the reuse check, reads the measurements, and explains what they mean.",
            system: "You investigate the integrity of a chapter. Start with internal similarity, \
                     which needs no network. Only consider a web check when a passage looks \
                     genuinely borrowed, and remember it sends the author's words off their \
                     machine and needs their explicit yes. Report measurements as measurements. \
                     You must never state or imply a conclusion about whether a human or a machine \
                     wrote something — that is not knowable from these numbers.",
            tools: vec![
                "list_documents",
                "read_document",
                "check_internal_similarity",
                "list_claims",
                "knowledge_gaps",
                "search_web",
            ],
        },
        Profile {
            id: "worldbuilder",
            name: "World-Builder",
            blurb: "Writes lore into the binder where it belongs, not into a chat box.",
            system: "You build out the world of the book and file it in the binder, not in your \
                     reply. Lore goes to Worldbuilding, characters to Characters. Facts you \
                     establish belong in canon with the document that establishes them as the \
                     source. Snapshot before writing anything substantial.",
            tools: vec![
                "list_documents",
                "read_document",
                "search_binder",
                "save_to_binder",
                "create_folder",
                "create_character_sheet",
                "create_location_sheet",
                "append_canon_entry",
                "search_sources",
                "list_claims",
                "knowledge_gaps",
                "generate_diagram",
                "snapshot_project",
            ],
        },
    ]
}

pub fn profile(id: &str) -> AppResult<Profile> {
    profiles()
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| crate::error::AppError(format!("there is no agent profile called `{id}`")))
}

/// The tool catalogue as OpenAI-style function definitions, narrowed to one
/// profile.
fn tool_definitions(profile: &Profile) -> Vec<Value> {
    agent::catalogue()
        .into_iter()
        .filter(|(name, _)| profile.tools.contains(name))
        .map(|(name, description)| {
            json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": description,
                    "parameters": agent::schema(name),
                }
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    error: Option<Value>,
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    message: Option<AssistantMessage>,
}

#[derive(Deserialize, Default)]
struct AssistantMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
}

#[derive(Deserialize, Clone)]
struct ToolCall {
    #[serde(default)]
    id: Option<String>,
    function: FunctionCall,
}

#[derive(Deserialize, Clone)]
struct FunctionCall {
    name: String,
    /// OpenAI sends this as a JSON *string*; Ollama sometimes sends an object.
    /// Accept both rather than failing on a difference that means nothing.
    #[serde(default)]
    arguments: Value,
}

fn parse_arguments(raw: &Value) -> Value {
    match raw {
        Value::String(text) => serde_json::from_str(text).unwrap_or_else(|_| json!({})),
        Value::Object(_) => raw.clone(),
        _ => json!({}),
    }
}

// ---------------------------------------------------------------------------
// Run records
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hop {
    pub tool: String,
    pub args: Value,
    pub ok: bool,
    /// The tool's JSON on success, or the failure in words.
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub profile: String,
    pub model: String,
    pub prompt: String,
    pub started_at: i64,
    /// The whole conversation, including the assistant turn that asked for this.
    pub messages: Vec<Value>,
    pub hops: Vec<Hop>,
    pub call_id: String,
    pub tool: String,
    pub args: Value,
    /// Why a person has to answer, in the words the UI shows.
    pub why: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Outcome {
    /// The model stopped calling tools and wrote an answer.
    Finished { answer: String },
    /// A guarded tool came up. Nothing was run; a person has to answer.
    NeedsApproval { pending: Box<Pending> },
    /// The hop cap was reached with the model still calling tools.
    Exhausted { hops: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub profile: String,
    pub model: String,
    pub prompt: String,
    pub started_at: i64,
    pub finished_at: i64,
    pub hops: Vec<Hop>,
    pub outcome: Outcome,
}

// ---------------------------------------------------------------------------
// The loop
// ---------------------------------------------------------------------------

fn http(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .into()
}

fn now_ms() -> i64 {
    crate::project::now_ms()
}

/// Start a run.
pub fn run(
    root: &Path,
    host: &str,
    model: &str,
    profile_id: &str,
    prompt: &str,
    allow_cloud: bool,
) -> AppResult<Run> {
    let profile = profile(profile_id)?;
    if crate::ollama::is_cloud_model(model) && !allow_cloud {
        return err(format!(
            "`{model}` is a remote model. An agent run sends your prompt, your tool results and \
             pieces of your manuscript to it. Turn remote models on for this session first."
        ));
    }
    let started_at = now_ms();
    let messages = vec![
        json!({ "role": "system", "content": format!("{}\n\n{}", SHARED, profile.system) }),
        json!({ "role": "user", "content": prompt }),
    ];
    drive(
        root,
        host,
        model,
        &profile,
        prompt,
        messages,
        Vec::new(),
        started_at,
    )
}

/// Carry on a paused run with a person's answer.
///
/// `approved = false` is not a cancellation: the refusal goes back to the model
/// as the tool's result, so it can say what it will do instead. That is how a
/// person declining a delete stays a conversation rather than a dead end.
pub fn resume(
    root: &Path,
    host: &str,
    pending: &Pending,
    approved: bool,
) -> AppResult<Run> {
    let profile = profile(&pending.profile)?;
    let mut messages = pending.messages.clone();
    let mut hops = pending.hops.clone();

    let hop = if approved {
        // The one place `confirmed` is ever set, and it is set because a person
        // said so — not because the model asked nicely.
        let mut args = pending.args.clone();
        if let Value::Object(map) = &mut args {
            map.insert("confirmed".into(), Value::Bool(true));
        }
        execute(root, &pending.tool, &args, &pending.model)
    } else {
        Hop {
            tool: pending.tool.clone(),
            args: pending.args.clone(),
            ok: false,
            result: json!({ "refused": "The author declined this. Do not ask again; continue without it or stop." }),
        }
    };
    messages.push(tool_message(&pending.call_id, &hop));
    hops.push(hop);

    drive(
        root,
        host,
        &pending.model,
        &profile,
        &pending.prompt,
        messages,
        hops,
        pending.started_at,
    )
}

fn tool_message(call_id: &str, hop: &Hop) -> Value {
    json!({
        "role": "tool",
        "tool_call_id": call_id,
        "name": hop.tool,
        "content": serde_json::to_string(&hop.result).unwrap_or_else(|_| "{}".into()),
    })
}

fn execute(root: &Path, tool: &str, args: &Value, model: &str) -> Hop {
    // The model is the author of anything this call writes. Threading it here
    // is what keeps the ledger honest about agent-written prose.
    let actor = crate::provenance::Actor::model(model, json!({ "via": "agent_loop" }));
    match agent::dispatch_as(root, tool, args, actor) {
        Ok(result) => Hop {
            tool: tool.to_string(),
            args: args.clone(),
            ok: true,
            result: result.value,
        },
        // A tool failure is data the model has to see, not an exception that
        // kills the run: "that id does not exist" is exactly what it needs to
        // choose a different one.
        Err(e) => Hop {
            tool: tool.to_string(),
            args: args.clone(),
            ok: false,
            result: json!({ "error": e.to_string() }),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn drive(
    root: &Path,
    host: &str,
    model: &str,
    profile: &Profile,
    prompt: &str,
    mut messages: Vec<Value>,
    mut hops: Vec<Hop>,
    started_at: i64,
) -> AppResult<Run> {
    let tools = tool_definitions(profile);

    let finish = |outcome: Outcome, hops: Vec<Hop>| Run {
        profile: profile.id.to_string(),
        model: model.to_string(),
        prompt: prompt.to_string(),
        started_at,
        finished_at: now_ms(),
        hops,
        outcome,
    };

    while hops.len() < MAX_HOPS {
        let reply = ask(host, model, &messages, &tools)?;

        if reply.tool_calls.is_empty() {
            let answer = reply.content.unwrap_or_default();
            let answer = if answer.trim().is_empty() {
                // An empty answer with no tool call is a real failure mode —
                // usually a reasoning model putting its output somewhere else.
                // Say that, rather than showing a blank panel.
                format!(
                    "`{model}` returned nothing and called no tool. If it is a reasoning model, its \
                     output may be arriving in a field this build does not read; try another model."
                )
            } else {
                answer.trim().to_string()
            };
            return Ok(finish(Outcome::Finished { answer }, hops));
        }

        // Record the assistant turn verbatim: resuming later depends on the
        // model seeing its own tool call in the history.
        messages.push(json!({
            "role": "assistant",
            "content": reply.content.clone().unwrap_or_default(),
            "tool_calls": reply.tool_calls.iter().enumerate().map(|(i, c)| json!({
                "id": c.id.clone().unwrap_or_else(|| format!("call_{i}")),
                "type": "function",
                "function": { "name": c.function.name, "arguments": c.function.arguments.clone() }
            })).collect::<Vec<_>>()
        }));

        for (i, call) in reply.tool_calls.iter().enumerate() {
            let call_id = call.id.clone().unwrap_or_else(|| format!("call_{i}"));
            let name = call.function.name.clone();
            let args = parse_arguments(&call.function.arguments);

            // A tool outside the profile is refused as data, so the model can
            // pick another rather than the run dying on it.
            if !profile.tools.contains(&name.as_str()) {
                let hop = Hop {
                    tool: name.clone(),
                    args: args.clone(),
                    ok: false,
                    result: json!({
                        "error": format!("`{name}` is not available to the {} profile.", profile.name)
                    }),
                };
                messages.push(tool_message(&call_id, &hop));
                hops.push(hop);
                continue;
            }

            if agent::GUARDED.contains(&name.as_str()) {
                let why = format!(
                    "The agent wants to run `{name}`, which changes your work or sends it off this \
                     machine. Nothing has run yet."
                );
                return Ok(finish(
                    Outcome::NeedsApproval {
                        pending: Box::new(Pending {
                            profile: profile.id.to_string(),
                            model: model.to_string(),
                            prompt: prompt.to_string(),
                            started_at,
                            messages: messages.clone(),
                            hops: hops.clone(),
                            call_id,
                            tool: name,
                            args,
                            why,
                        }),
                    },
                    hops,
                ));
            }

            let hop = execute(root, &name, &args, model);
            messages.push(tool_message(&call_id, &hop));
            hops.push(hop);

            if hops.len() >= MAX_HOPS {
                break;
            }
        }
    }

    let n = hops.len();
    Ok(finish(Outcome::Exhausted { hops: n }, hops))
}

fn ask(
    host: &str,
    model: &str,
    messages: &[Value],
    tools: &[Value],
) -> AppResult<AssistantMessage> {
    let body = json!({
        "model": model,
        "messages": messages,
        "tools": tools,
        "stream": false,
        // Tool selection should be repeatable; creativity belongs in the prose
        // tools, not in deciding which tool to call.
        "temperature": 0.2,
    });

    let mut res = http(Duration::from_secs(240))
        .post(&format!("{host}/v1/chat/completions"))
        .send_json(&body)
        .map_err(|e| crate::error::AppError(format!("could not reach the model at {host}: {e}")))?;

    let code = res.status().as_u16();
    let text = res.body_mut().read_to_string().unwrap_or_default();
    if code >= 300 {
        return err(format!("the model host answered {code}: {}", text.trim()));
    }
    let parsed: ChatResponse = serde_json::from_str(&text).map_err(|e| {
        crate::error::AppError(format!("could not read the model's reply: {e} — {}", text.trim()))
    })?;
    if let Some(error) = parsed.error {
        return err(format!("the model host reported an error: {error}"));
    }
    parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message)
        .ok_or_else(|| crate::error::AppError("the model returned no reply at all".into()))
}

// ---------------------------------------------------------------------------
// Traces
// ---------------------------------------------------------------------------

pub const TRACE_DIR: &str = "traces";

/// Append a run to the project's trace folder, one JSON file per run.
///
/// Named by start time so they sort, and written into the project rather than
/// an app-support folder: the record of what an agent did to a book belongs
/// with the book.
pub fn record(root: &Path, run: &Run) -> AppResult<String> {
    let dir = root.join(TRACE_DIR);
    std::fs::create_dir_all(&dir)?;
    let name = format!("{}-{}.json", run.started_at, run.profile);
    std::fs::write(dir.join(&name), serde_json::to_string_pretty(run)?)?;
    Ok(name)
}

/// Every recorded run, newest first.
pub fn traces(root: &Path, limit: usize) -> Vec<Run> {
    let dir = root.join(TRACE_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    files.reverse();
    files
        .into_iter()
        .take(limit)
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter_map(|raw| serde_json::from_str(&raw).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc::Receiver;

    /// A stub chat endpoint that answers each request with the next canned
    /// reply. Drives the loop end to end without a model.
    fn stub(replies: Vec<String>) -> (String, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for reply in replies {
                let Ok((mut stream, _)) = listener.accept() else { return };
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                // Read headers, then exactly the declared body.
                loop {
                    let n = match stream.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&buf).to_string();
                    if let Some(header_end) = text.find("\r\n\r\n") {
                        let len = text
                            .lines()
                            .find_map(|l| {
                                let l = l.to_lowercase();
                                l.strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                            })
                            .unwrap_or(0);
                        if buf.len() >= header_end + 4 + len {
                            break;
                        }
                    }
                }
                let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        (format!("http://127.0.0.1:{port}"), rx)
    }

    fn tool_reply(name: &str, args: &str) -> String {
        format!(
            r#"{{"choices":[{{"message":{{"content":"","tool_calls":[{{"id":"call_a","type":"function","function":{{"name":"{name}","arguments":{}}}}}]}}}}]}}"#,
            serde_json::to_string(args).unwrap()
        )
    }

    /// ureq serialises the request body pretty-printed, so every assertion
    /// about what was sent compares against a whitespace-stripped copy.
    fn compact(text: &str) -> String {
        text.chars().filter(|c| !c.is_whitespace()).collect()
    }

    fn text_reply(text: &str) -> String {
        format!(
            r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
            serde_json::to_string(text).unwrap()
        )
    }

    fn scratch(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "aegis-loop-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = crate::project::create(&base, "Loop", "novel").unwrap();
        (base, std::path::PathBuf::from(summary.path))
    }

    #[test]
    fn every_tool_in_the_catalogue_has_a_schema_that_matches_what_it_reads() {
        for (name, _) in agent::catalogue() {
            let schema = agent::schema(name);
            assert_eq!(
                schema["type"], "object",
                "`{name}` has no schema, so no model can call it"
            );
            let props = schema["properties"].as_object().unwrap();
            for required in schema["required"].as_array().unwrap() {
                let key = required.as_str().unwrap();
                assert!(
                    props.contains_key(key),
                    "`{name}` requires `{key}` but does not describe it"
                );
            }
            if agent::GUARDED.contains(&name) {
                assert!(
                    props.contains_key("confirmed"),
                    "`{name}` is guarded, and its schema must say so"
                );
            }
        }
    }

    #[test]
    fn every_profile_only_names_tools_that_exist() {
        let known: Vec<&str> = agent::catalogue().into_iter().map(|(n, _)| n).collect();
        for p in profiles() {
            assert!(!p.tools.is_empty(), "{} reaches nothing", p.name);
            for tool in &p.tools {
                assert!(
                    known.contains(tool),
                    "{} lists `{tool}`, which is not in the gateway",
                    p.name
                );
            }
        }
        // The narrow profiles must not be able to destroy anything, whatever
        // they are told. This is the constraint, not the prompt.
        for id in ["continuity", "integrity", "worldbuilder"] {
            assert!(
                !profile(id).unwrap().tools.contains(&"delete_document"),
                "{id} must not reach delete_document"
            );
        }
    }

    #[test]
    fn a_tool_call_runs_against_the_real_project_and_the_result_goes_back() {
        let (base, root) = scratch("basic");
        let (host, rx) = stub(vec![
            tool_reply("create_folder", r#"{"name":"Act Two"}"#),
            text_reply("Created a folder called Act Two."),
        ]);

        let out = run(&root, &host, "test-model", "controller", "make an act two folder", false).unwrap();

        assert_eq!(out.hops.len(), 1);
        assert!(out.hops[0].ok, "{:?}", out.hops[0].result);
        assert_eq!(out.hops[0].tool, "create_folder");
        match &out.outcome {
            Outcome::Finished { answer } => assert!(answer.contains("Act Two")),
            other => panic!("expected a finish, got {other:?}"),
        }

        // The folder is really in the binder, not just in the transcript.
        let docs = crate::project::documents(&root).unwrap();
        assert!(docs.iter().any(|d| d.title == "Act Two"));

        // The second request carried the tool result back to the model.
        let _first = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let second = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            compact(&second).contains("\"role\":\"tool\""),
            "the result must be fed back: {second}"
        );
        assert!(second.contains("Act Two"));

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn the_catalogue_it_sends_is_narrowed_to_the_profile() {
        let (base, root) = scratch("catalogue");
        let (host, rx) = stub(vec![text_reply("Nothing to do.")]);
        run(&root, &host, "test-model", "integrity", "check this", false).unwrap();

        let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(request.contains("check_internal_similarity"));
        assert!(
            !request.contains("create_folder"),
            "the Integrity profile must not even be shown binder mutations"
        );
        assert!(request.contains("\"parameters\""), "tools need schemas: {request}");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_guarded_tool_pauses_the_run_and_nothing_happens_until_a_person_answers() {
        let (base, root) = scratch("guard");
        let doomed =
            crate::project::create_document(&root, None, "Doomed", crate::model::DocKind::Scene).unwrap();
        let file = root.join(doomed.path.clone().unwrap());
        assert!(file.exists());

        let (host, _rx) = stub(vec![tool_reply(
            "delete_document",
            &format!(r#"{{"id":"{}","confirmed":true}}"#, doomed.id),
        )]);
        let paused = run(&root, &host, "test-model", "controller", "delete it", false).unwrap();

        let pending = match &paused.outcome {
            Outcome::NeedsApproval { pending } => pending.clone(),
            other => panic!("a delete must pause, got {other:?}"),
        };
        assert_eq!(pending.tool, "delete_document");
        assert!(
            file.exists(),
            "the model asserted `confirmed: true` itself — the file must still be there"
        );
        assert!(paused.hops.is_empty(), "nothing ran");

        // Declining feeds the refusal back rather than ending the run.
        let (host2, _rx2) = stub(vec![text_reply("Understood, I left it alone.")]);
        let declined = resume(&root, &host2, &pending, false).unwrap();
        assert!(file.exists(), "declining must not delete anything");
        assert_eq!(declined.hops.len(), 1);
        assert!(!declined.hops[0].ok);

        // Approving actually runs it.
        let (host3, _rx3) = stub(vec![text_reply("Deleted.")]);
        let approved = resume(&root, &host3, &pending, true).unwrap();
        assert!(approved.hops[0].ok, "{:?}", approved.hops[0].result);
        assert!(!file.exists(), "an approved delete must really delete");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_tool_outside_the_profile_is_refused_as_data_not_as_a_crash() {
        let (base, root) = scratch("outside");
        let (host, _rx) = stub(vec![
            tool_reply("delete_document", r#"{"id":"x"}"#),
            text_reply("I cannot delete from here."),
        ]);
        let out = run(&root, &host, "test-model", "worldbuilder", "delete everything", false).unwrap();

        assert_eq!(out.hops.len(), 1);
        assert!(!out.hops[0].ok);
        assert!(
            out.hops[0].result["error"].as_str().unwrap().contains("not available"),
            "{:?}",
            out.hops[0].result
        );
        matches!(out.outcome, Outcome::Finished { .. });

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_failing_tool_is_reported_to_the_model_rather_than_ending_the_run() {
        let (base, root) = scratch("failing");
        let (host, rx) = stub(vec![
            tool_reply("read_document", r#"{"id":"does-not-exist"}"#),
            text_reply("That document is not there."),
        ]);
        let out = run(&root, &host, "test-model", "continuity", "read it", false).unwrap();

        assert_eq!(out.hops.len(), 1);
        assert!(!out.hops[0].ok);
        let _ = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let second = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(second.contains("error"), "the failure must reach the model: {second}");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn arguments_parse_whether_they_arrive_as_a_string_or_an_object() {
        assert_eq!(parse_arguments(&json!("{\"a\":1}"))["a"], 1);
        assert_eq!(parse_arguments(&json!({ "a": 1 }))["a"], 1);
        // Malformed arguments become an empty object, so the tool's own
        // "needs an `id` argument" is what the model hears — not a parse error
        // it can do nothing with.
        assert_eq!(parse_arguments(&json!("not json")), json!({}));
        assert_eq!(parse_arguments(&Value::Null), json!({}));
    }

    #[test]
    fn a_run_that_never_stops_calling_tools_is_cut_off() {
        let (base, root) = scratch("runaway");
        let replies: Vec<String> = (0..MAX_HOPS + 2)
            .map(|_| tool_reply("list_documents", "{}"))
            .collect();
        let (host, _rx) = stub(replies);
        let out = run(&root, &host, "test-model", "controller", "loop forever", false).unwrap();
        match out.outcome {
            Outcome::Exhausted { hops } => assert_eq!(hops, MAX_HOPS),
            other => panic!("expected exhaustion, got {other:?}"),
        }
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn an_empty_reply_says_what_probably_went_wrong() {
        let (base, root) = scratch("empty");
        let (host, _rx) = stub(vec![text_reply("")]);
        let out = run(&root, &host, "reasoner:8b", "controller", "hello", false).unwrap();
        match out.outcome {
            Outcome::Finished { answer } => {
                assert!(answer.contains("reasoning model"), "{answer}");
                assert!(answer.contains("reasoner:8b"));
            }
            other => panic!("{other:?}"),
        }
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_remote_model_is_refused_unless_it_was_turned_on_for_the_session() {
        let (base, root) = scratch("cloud");
        let message = run(&root, "http://127.0.0.1:1", "qwen3.5:397b-cloud", "controller", "hi", false)
            .unwrap_err()
            .to_string();
        assert!(message.contains("remote model"), "{message}");
        assert!(message.contains("sends your prompt"), "it must say what leaves: {message}");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_run_is_written_into_the_project_where_the_book_is() {
        let (base, root) = scratch("trace");
        let (host, _rx) = stub(vec![
            tool_reply("create_folder", r#"{"name":"Traced"}"#),
            text_reply("Made it."),
        ]);
        let out = run(&root, &host, "test-model", "controller", "make Traced", false).unwrap();
        let name = record(&root, &out).unwrap();

        assert!(root.join(TRACE_DIR).join(&name).exists());
        let back = traces(&root, 10);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].prompt, "make Traced");
        assert_eq!(back[0].hops[0].tool, "create_folder");
        // A person can read it without the app.
        let raw = std::fs::read_to_string(root.join(TRACE_DIR).join(&name)).unwrap();
        assert!(raw.contains("create_folder") && raw.contains("Traced"));

        std::fs::remove_dir_all(&base).ok();
    }

    /// A remote model, only when explicitly asked for.
    ///
    /// Gated behind an environment variable rather than the usual reachability
    /// check, because "is it reachable" is the wrong question here: this test
    /// sends a prompt and tool results to someone else's computer, and that must
    /// never happen as a side effect of running `cargo test`.
    #[test]
    fn live_cloud_the_loop_drives_a_remote_model_when_asked() {
        if std::env::var("AEGISDRAFT_LIVE_CLOUD").as_deref() != Ok("1") {
            eprintln!(
                "SKIPPED live_cloud: set AEGISDRAFT_LIVE_CLOUD=1 to send a prompt off this machine"
            );
            return;
        }
        let status = crate::ollama::status();
        let model = std::env::var("AEGISDRAFT_CLOUD_MODEL")
            .unwrap_or_else(|_| "qwen3.5:397b-cloud".to_string());
        if !status.reachable || !status.models.iter().any(|m| m.name == model) {
            eprintln!("SKIPPED live_cloud: {model} is not available through Ollama");
            return;
        }

        let (base, root) = scratch("livecloud");
        match run(
            &root,
            crate::ollama::DEFAULT_HOST,
            &model,
            "controller",
            "Create a binder folder named exactly: Act Two. Then create two empty chapters inside it. Then stop.",
            true,
        ) {
            Ok(run) => {
                eprintln!("EVIDENCE live_cloud [{model}]: {} hops", run.hops.len());
                for hop in &run.hops {
                    eprintln!("EVIDENCE   {} {} -> ok={}", hop.tool, hop.args, hop.ok);
                }
                eprintln!("EVIDENCE outcome: {:?}", run.outcome);
                let docs = crate::project::documents(&root).unwrap();
                eprintln!(
                    "EVIDENCE binder now: {:?}",
                    docs.iter().map(|d| d.title.as_str()).collect::<Vec<_>>()
                );
            }
            Err(e) => eprintln!("EVIDENCE live_cloud failed against {model}: {e}"),
        }
        std::fs::remove_dir_all(&base).ok();
    }

    /// Live, against whatever model is actually installed. Skips loudly.
    #[test]
    fn live_the_loop_drives_a_real_model_through_a_real_tool_call() {
        let status = crate::ollama::status();
        if !status.reachable {
            eprintln!("SKIPPED live_the_loop: Ollama is not running");
            return;
        }
        let models = crate::ollama::generation_models(&status);
        let Some(model) = models.first() else {
            eprintln!("SKIPPED live_the_loop: no local generation model installed");
            return;
        };

        let (base, root) = scratch("live");
        let out = run(
            &root,
            crate::ollama::DEFAULT_HOST,
            &model.name,
            "controller",
            "Create a binder folder named exactly: Act Two. Then stop.",
            false,
        );
        match out {
            Ok(run) => {
                eprintln!(
                    "EVIDENCE live_the_loop [{}]: {} hops, outcome {:?}",
                    model.name,
                    run.hops.len(),
                    run.outcome
                );
                for hop in &run.hops {
                    eprintln!("EVIDENCE   {} {} -> ok={}", hop.tool, hop.args, hop.ok);
                }
                let docs = crate::project::documents(&root).unwrap();
                let made: Vec<&str> = docs.iter().map(|d| d.title.as_str()).collect();
                eprintln!("EVIDENCE binder now: {made:?}");
                // Deliberately not asserted: whether a small model gets the name
                // exactly right is a fact about that model, not about this loop.
                // The loop is proven by the stub tests; this one records reality.
            }
            Err(e) => eprintln!("EVIDENCE live_the_loop failed against {}: {e}", model.name),
        }
        std::fs::remove_dir_all(&base).ok();
    }
}
