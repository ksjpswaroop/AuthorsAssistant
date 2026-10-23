//! Ollama client (Phase 3).
//!
//! Everything here talks to loopback only. There is no configuration for a
//! remote host because there is no code path that would use one — the privacy
//! promise is enforced by the absence of the feature, not by a setting.

use crate::error::{err, AppResult};
use crate::model::{ModelInfo, OllamaStatus};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const DEFAULT_HOST: &str = "http://127.0.0.1:11434";

fn host() -> String {
    std::env::var("AEGISDRAFT_OLLAMA").unwrap_or_else(|_| DEFAULT_HOST.to_string())
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        // Keep 4xx/5xx as ordinary responses. ureq's error for a status code
        // carries only the number, and Ollama puts the reason in the body —
        // which is the part a user can act on.
        .http_status_as_error(false)
        .build()
        .into()
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<TagModel>,
}

#[derive(Deserialize)]
struct TagModel {
    name: String,
    #[serde(default)]
    size: u64,
}

/// Is Ollama running, and what does it have? Never an error — "not running" is
/// a normal state the UI must show, not an exception.
pub fn status() -> OllamaStatus {
    status_at(&host())
}

pub fn status_at(host: &str) -> OllamaStatus {
    match agent(Duration::from_secs(3)).get(&format!("{host}/api/tags")).call() {
        Ok(mut res) if res.status().is_success() => match res.body_mut().read_json::<TagsResponse>() {
            Ok(tags) => OllamaStatus {
                reachable: true,
                host: host.to_string(),
                models: tags
                    .models
                    .into_iter()
                    .map(|m| ModelInfo { name: m.name, size: m.size })
                    .collect(),
                message: None,
            },
            Err(e) => OllamaStatus {
                reachable: false,
                host: host.to_string(),
                models: vec![],
                message: Some(format!("Ollama replied with something unexpected: {e}")),
            },
        },
        Ok(res) => OllamaStatus {
            reachable: false,
            host: host.to_string(),
            models: vec![],
            message: Some(format!("Ollama answered with HTTP {}", res.status())),
        },
        Err(_) => OllamaStatus {
            reachable: false,
            host: host.to_string(),
            models: vec![],
            message: Some("Ollama is not running. Start it, then try again.".into()),
        },
    }
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a [String],
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

/// Names that mark a model as built for embeddings. Ollama exposes no flag for
/// this, and asking a chat model to embed fails with a message that blames the
/// server ("This server does not support embeddings") rather than the model —
/// so the distinction has to be made here or the error is unreadable.
pub fn looks_like_embedding_model(name: &str) -> bool {
    let n = name.to_lowercase();
    ["embed", "bge", "gte", "e5-", "minilm"].iter().any(|hint| n.contains(hint))
}

/// A `:cloud` tag is a model Ollama proxies to a remote service. Using one
/// sends the prompt — and therefore the manuscript — off this machine, which is
/// the single thing this product promises not to do. It is never chosen
/// automatically; the caller must opt in, and the UI must say so.
pub fn is_cloud_model(name: &str) -> bool {
    name.contains(":cloud") || name.ends_with("-cloud")
}

/// Models that can generate text: local, real weights, and not an embedding
/// model. An embedding model asked to generate returns an empty string, so it
/// must be excluded here as deliberately as chat models are excluded above.
pub fn generation_models(status: &OllamaStatus) -> Vec<ModelInfo> {
    let mut ms: Vec<ModelInfo> = status
        .models
        .iter()
        .filter(|m| !m.name.contains(":cloud") && m.size >= 100_000_000 && !looks_like_embedding_model(&m.name))
        .cloned()
        .collect();
    ms.sort_by_key(|m| m.size);
    ms
}

/// Embedding models installed here, cheapest first. Name is a hint only; the
/// caller should still handle a failure, because the only certain test is a
/// real request.
pub fn embedding_models(status: &OllamaStatus) -> Vec<ModelInfo> {
    let mut ms: Vec<ModelInfo> = status
        .models
        .iter()
        .filter(|m| !m.name.contains(":cloud") && looks_like_embedding_model(&m.name))
        .cloned()
        .collect();
    ms.sort_by_key(|m| m.size);
    ms
}

/// Embeds a batch of texts. Embedding is the hot path during indexing, so it
/// goes in one request rather than one per chunk.
pub fn embed(model: &str, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
    embed_at(&host(), model, texts)
}

pub fn embed_at(host: &str, model: &str, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let mut res = agent(Duration::from_secs(120))
        .post(&format!("{host}/api/embed"))
        .send_json(EmbedRequest { model, input: texts })
        .map_err(|e| crate::error::AppError(format!("could not reach Ollama to embed: {e}")))?;

    let status = res.status().as_u16();
    if status >= 300 {
        let body = res.body_mut().read_to_string().unwrap_or_default();
        // Ollama answers 501 "This server does not support embeddings" even when
        // the server is fine and the model simply cannot embed. Say what is
        // actually wrong — that message cost a wrong diagnosis once already.
        if !looks_like_embedding_model(model) {
            return err(format!(
                "`{model}` is not an embedding model. Pull one — for example \
                 `ollama pull nomic-embed-text` — and choose it for indexing."
            ));
        }
        return Err(refusal(status, body));
    }

    let parsed: EmbedResponse = res
        .body_mut()
        .read_json()
        .map_err(|e| crate::error::AppError(format!("Ollama returned an unreadable embedding: {e}")))?;

    if parsed.embeddings.len() != texts.len() {
        return err(format!(
            "asked for {} embeddings and got {}",
            texts.len(),
            parsed.embeddings.len()
        ));
    }
    Ok(parsed.embeddings)
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    /// Reasoning models put their chain of thought in a separate `thinking`
    /// field and leave `response` empty until they finish. On a long prompt
    /// that never happens inside any sane token budget, so the answer comes
    /// back blank. We want prose, not deliberation.
    think: bool,
    options: GenerateOptions,
}

#[derive(Serialize)]
struct GenerateOptions {
    temperature: f32,
    num_predict: i32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
    /// Present on reasoning models. Read only so an empty answer can explain
    /// itself instead of surfacing as an empty string.
    #[serde(default)]
    thinking: Option<String>,
    #[serde(default)]
    done_reason: Option<String>,
}

#[derive(Deserialize)]
struct OllamaError {
    error: String,
}

/// Turns a non-2xx response into an error carrying Ollama's own explanation.
fn refusal(status: u16, body: String) -> crate::error::AppError {
    let detail = serde_json::from_str::<OllamaError>(&body)
        .map(|e| e.error)
        .unwrap_or_else(|_| body.chars().take(200).collect());
    crate::error::AppError(format!("Ollama refused this (HTTP {status}): {detail}"))
}

/// One-shot generation. Streaming belongs in the UI layer later; the tools that
/// call this want a whole answer.
pub fn generate(model: &str, prompt: &str, system: Option<&str>, max_tokens: i32, temperature: f32) -> AppResult<String> {
    generate_at(&host(), model, prompt, system, max_tokens, temperature)
}

/// Refuses a cloud model unless the caller has explicitly consented. The gate
/// lives here, below the UI, so no future caller can reach a remote model by
/// accident.
pub fn generate_guarded(
    model: &str,
    prompt: &str,
    system: Option<&str>,
    max_tokens: i32,
    temperature: f32,
    allow_cloud: bool,
) -> AppResult<String> {
    if is_cloud_model(model) && !allow_cloud {
        return err(format!(
            "`{model}` runs on a remote service, so using it would send this text off \
             your machine. Turn on cloud models in Settings if that is what you want."
        ));
    }
    generate_at(&host(), model, prompt, system, max_tokens, temperature)
}

pub fn generate_at(
    host: &str,
    model: &str,
    prompt: &str,
    system: Option<&str>,
    max_tokens: i32,
    temperature: f32,
) -> AppResult<String> {
    if prompt.trim().is_empty() {
        return err("nothing to send");
    }
    let mut res = agent(Duration::from_secs(300))
        .post(&format!("{host}/api/generate"))
        .send_json(GenerateRequest {
            model,
            prompt,
            stream: false,
            system,
            think: false,
            options: GenerateOptions { temperature, num_predict: max_tokens },
        })
        .map_err(|e| crate::error::AppError(format!("could not reach Ollama: {e}")))?;

    let status = res.status().as_u16();
    if status >= 300 {
        let body = res.body_mut().read_to_string().unwrap_or_default();
        if looks_like_embedding_model(model) {
            return err(format!(
                "`{model}` is an embedding model and cannot write prose. Choose a \
                 generation model such as llama3.2:3b."
            ));
        }
        return Err(refusal(status, body));
    }

    let parsed: GenerateResponse = res
        .body_mut()
        .read_json()
        .map_err(|e| crate::error::AppError(format!("Ollama returned an unreadable answer: {e}")))?;

    if parsed.response.trim().is_empty() {
        // An empty string is the worst possible answer: it looks like the
        // feature silently failed. Say which of the two things happened.
        let thought = parsed.thinking.unwrap_or_default();
        if !thought.trim().is_empty() {
            return err(format!(
                "`{model}` spent its whole budget reasoning and never wrote an answer. \
                 Give it more room, or choose a model that does not reason before replying."
            ));
        }
        if parsed.done_reason.as_deref() == Some("length") {
            return err(format!("`{model}` hit the token limit before writing anything."));
        }
        return err(format!("`{model}` returned nothing."));
    }
    Ok(parsed.response)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing listens here, so it stands in for "Ollama is not installed".
    const DEAD: &str = "http://127.0.0.1:1";

    /// Live tests talk to a real Ollama. When it is not running they say so and
    /// pass — a developer without Ollama still gets a green suite, and the live
    /// assertions really run wherever it is present. No test mutates the
    /// environment, because cargo runs them in parallel.
    fn live() -> Option<(String, OllamaStatus)> {
        let h = host();
        let s = status_at(&h);
        if s.reachable {
            Some((h, s))
        } else {
            eprintln!("SKIP: Ollama not reachable at {h} — live assertions skipped");
            None
        }
    }

    /// The smallest model that actually runs on this machine. A `:cloud` tag is
    /// a remote model Ollama proxies — it reports a tiny size, sorts first, and
    /// returns nothing locally, so it must be excluded explicitly.
    /// An installed embedding model, if there is one.
    ///
    /// An earlier version of this helper picked "the smallest local model" and
    /// asked it to embed. Ollama answers that with "This server does not
    /// support embeddings", which reads like a daemon limitation but is really
    /// "this model is not an embedding model" — and the wrong diagnosis got
    /// reported as fact. Ask for an embedding model by name instead.
    fn embedding_model(s: &OllamaStatus) -> Option<String> {
        match embedding_models(s).first() {
            Some(m) => Some(m.name.clone()),
            None => {
                eprintln!(
                    "SKIP: no embedding model installed. `ollama pull nomic-embed-text` \
                     (or any *-embed* model) to run these assertions."
                );
                None
            }
        }
    }

    /// The cheapest model here that can actually generate. Must not fall back to
    /// an embedding model: those are small, so they sort first, and they return
    /// nothing when asked for prose.
    fn smallest_local(s: &OllamaStatus) -> Option<String> {
        match generation_models(s).first() {
            Some(m) => Some(m.name.clone()),
            None => {
                eprintln!("SKIP: no local generation model installed");
                None
            }
        }
    }

    #[test]
    fn status_reports_absence_instead_of_failing() {
        let s = status_at(DEAD);
        assert!(!s.reachable);
        assert!(s.message.is_some(), "an unreachable daemon must explain itself");
        assert!(s.models.is_empty());
    }

    #[test]
    fn empty_input_short_circuits_without_a_request() {
        assert_eq!(embed_at(DEAD, "nonexistent", &[]).unwrap().len(), 0);
        assert!(generate_at(DEAD, "nonexistent", "   ", None, 10, 0.0).is_err());
    }

    #[test]
    fn a_chat_model_asked_to_embed_is_told_what_is_actually_wrong() {
        // Ollama's own message blames the server. If a real daemon is running
        // with only chat models, the error must name the model instead.
        let Some((h, s)) = live() else { return };
        let Some(chat) = s
            .models
            .iter()
            .find(|m| !m.name.contains(":cloud") && m.size >= 100_000_000 && !looks_like_embedding_model(&m.name))
        else {
            return;
        };
        match embed_at(&h, &chat.name, &["probe".to_string()]) {
            Ok(_) => eprintln!("NOTE: {} embeds after all; no misleading error to correct", chat.name),
            Err(e) => {
                let msg = format!("{e}");
                eprintln!("EVIDENCE chat-model-embed: {msg}");
                assert!(
                    msg.contains("is not an embedding model"),
                    "the error must name the model, not blame the server: {msg}"
                );
            }
        }
    }

    #[test]
    fn generation_and_embedding_pools_never_overlap() {
        // Picking a model for the wrong job fails in a way that looks like the
        // feature is broken: an embedding model generates an empty string, and
        // a chat model refuses to embed with a message that blames the server.
        let Some((_, s)) = live() else { return };
        let gen: Vec<String> = generation_models(&s).into_iter().map(|m| m.name).collect();
        let emb: Vec<String> = embedding_models(&s).into_iter().map(|m| m.name).collect();
        eprintln!("EVIDENCE pools: generation={gen:?} embedding={emb:?}");
        for g in &gen {
            assert!(!emb.contains(g), "{g} is in both pools");
        }
        assert!(gen.iter().all(|m| !m.contains(":cloud")));
        assert!(emb.iter().all(|m| !m.contains(":cloud")));
    }

    #[test]
    fn cloud_models_are_recognised_and_gated() {
        assert!(is_cloud_model("qwen3.5:cloud"));
        assert!(is_cloud_model("qwen3.5:397b-cloud"));
        assert!(is_cloud_model("deepseek-v4-flash:cloud"));
        assert!(!is_cloud_model("llama3.2:3b"));

        // Without consent the call is refused before any request is made.
        let r = generate_guarded("qwen3.5:397b-cloud", "hello", None, 32, 0.0, false);
        let msg = format!("{}", r.unwrap_err());
        assert!(msg.contains("off \nyour machine") || msg.contains("off your machine"), "must warn about leaving the machine: {msg}");
    }

    /// A reasoning model with a small budget answers with an empty `response`
    /// and a long `thinking`. Returning "" would look like a broken feature.
    #[test]
    fn live_a_reasoning_model_still_produces_prose() {
        let Some((h, s)) = live() else { return };
        let cloud = s.models.iter().find(|m| is_cloud_model(&m.name)).map(|m| m.name.clone());
        let Some(name) = cloud else {
            eprintln!("SKIP: no cloud model installed");
            return;
        };
        match generate_at(&h, &name, "Continue in one sentence: The well was going dry.", None, 300, 0.7) {
            Ok(out) => {
                eprintln!("EVIDENCE reasoning-model: {name} -> {:?}", out.trim().chars().take(120).collect::<String>());
                assert!(!out.trim().is_empty());
            }
            Err(e) => eprintln!("SKIP: {name} unavailable ({e})"),
        }
    }

    #[test]
    fn embedding_models_are_recognised_by_name() {
        for good in ["nomic-embed-text", "xagi/qwen3-embedding:0.6b-q4_k_m-1024", "bge-m3", "all-minilm"] {
            assert!(looks_like_embedding_model(good), "{good} should be recognised");
        }
        for bad in ["llama3.2:3b", "qwen2.5-coder:7b", "gpt-oss:20b"] {
            assert!(!looks_like_embedding_model(bad), "{bad} is not an embedding model");
        }
    }

    #[test]
    fn an_unreachable_host_is_an_error_not_a_hang() {
        let r = embed_at(DEAD, "any", &["text".to_string()]);
        assert!(r.is_err());
        assert!(format!("{}", r.unwrap_err()).contains("could not reach Ollama"));
    }

    #[test]
    fn live_status_lists_installed_models() {
        let Some((h, s)) = live() else { return };
        assert!(!s.models.is_empty(), "a reachable Ollama should report models");
        eprintln!("EVIDENCE live_status: {} models at {h}", s.models.len());
        for m in s.models.iter().take(6) {
            eprintln!("  - {} ({:.1} GB)", m.name, m.size as f64 / 1e9);
        }
    }

    #[test]
    fn live_embeddings_are_deterministic_and_semantically_ordered() {
        let Some((h, s)) = live() else { return };
        let Some(model) = embedding_model(&s) else { return };
        let texts = vec![
            "The well at Kharsa was going dry.".to_string(),
            "The well in the town had almost no water left.".to_string(),
            "Quarterly amortisation of intangible assets.".to_string(),
        ];
        let vecs = embed_at(&h, &model, &texts).expect("embedding should succeed");
        assert_eq!(vecs.len(), 3);
        assert!(vecs[0].len() > 8, "an embedding should have real dimensions");
        assert!(vecs.iter().all(|v| v.len() == vecs[0].len()), "dimensions must agree");

        let again = embed_at(&h, &model, &texts[..1]).unwrap();
        let repeat = crate::rag::cosine(&vecs[0], &again[0]);
        assert!(repeat > 0.99, "embeddings must be deterministic (got {repeat:.4})");

        let near = crate::rag::cosine(&vecs[0], &vecs[1]);
        let far = crate::rag::cosine(&vecs[0], &vecs[2]);
        eprintln!(
            "EVIDENCE live_embeddings: model={model} dims={} repeat={repeat:.4} near={near:.4} far={far:.4}",
            vecs[0].len()
        );
        assert!(near > far, "related prose must outscore unrelated prose (near {near:.4} vs far {far:.4})");
    }

    #[test]
    fn live_generation_returns_text() {
        let Some((h, s)) = live() else { return };
        let Some(model) = smallest_local(&s) else { return };
        let out = generate_at(&h, &model, "Reply with exactly the word: acknowledged", Some("Answer in one word."), 16, 0.0)
            .expect("generation should succeed");
        eprintln!("EVIDENCE live_generate: model={model} -> {:?}", out.trim());
        assert!(!out.trim().is_empty(), "generation must return something");
    }

    #[test]
    fn live_end_to_end_retrieval_finds_the_right_passage() {
        let Some((h, s)) = live() else { return };
        let Some(model) = embedding_model(&s) else { return };
        // Index three real passages, then ask a question none of them contains
        // verbatim, and check the right one comes back first.
        let passages = vec![
            "The cartographer broke his leg on the descent and the surgeon splinted it badly.".to_string(),
            "Tamsin bought two skins of water with money she did not have.".to_string(),
            "The marker stone had been moved since the last crossing.".to_string(),
        ];
        let vecs = embed_at(&h, &model, &passages).unwrap();

        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE documents (id TEXT PRIMARY KEY);").unwrap();
        conn.execute("INSERT INTO documents (id) VALUES ('d1')", []).unwrap();
        crate::rag::migrate(&conn).unwrap();
        let chunks: Vec<(String, Vec<f32>)> = passages.iter().cloned().zip(vecs).collect();
        crate::rag::index_document(&mut conn, "d1", &chunks, &model, 0).unwrap();

        let q = embed_at(&h, &model, &["Who was injured and how was it treated?".to_string()]).unwrap();
        let hits = crate::rag::search(&conn, &q[0], 3, None).unwrap();
        eprintln!("EVIDENCE live_retrieval: top={:?} score={:.4}", hits[0].text, hits[0].score);
        assert!(
            hits[0].text.contains("broke his leg"),
            "retrieval should surface the injury passage, got: {:?}",
            hits[0].text
        );
    }
}
