//! End-to-end integration across every implemented phase.
//!
//! These run against a real project folder on disk — real SQLite, real Markdown
//! files, a real Git repository — rather than in-memory fixtures. A unit test
//! can pass while a module's migration is never wired up or a command never
//! reaches the filesystem; this suite is what catches that.

use aegisdraft_lib::{
    agent, analysis, collections, compile, import, integrity, project, scrivenings, snapshots,
    blocks, canvas, continuity, knowledge, provenance, research, sources, store, style,
    websearch,
};
use aegisdraft_lib::model::{
    CollectionKind, CompileFilters, CompilePreset, DocKind, ExportFormat, SmartFilter,
};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

struct Fixture {
    parent: PathBuf,
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.parent).ok();
    }
}

/// A project with three chapters, one of which deliberately reuses a paragraph
/// from another, plus a linked character sheet and story dates.
fn fixture(tag: &str) -> Fixture {
    let parent = std::env::temp_dir().join(format!(
        "aegis-e2e-{tag}-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&parent).unwrap();
    let root = PathBuf::from(&project::create(&parent, "E2E Book", "novel").unwrap().path);

    let act = project::create_document(&root, None, "Act I", DocKind::Folder).unwrap();
    let opening = "The well at Kharsa was the only honest thing in the town and it was going dry that season.";

    let c1 = project::create_document(&root, Some(&act.id), "1. The Well", DocKind::Scene).unwrap();
    project::write_body(&root, &c1.id, &format!("{opening}\n\nShe counted what she had left. Twice.")).unwrap();
    project::update_meta(&root, &c1.id, Some("Tamsin buys water."), Some("Plot"), Some("Done")).unwrap();
    project::update_extended(&root, &c1.id, Some("Takes place on Day 1."), Some(2000), Some("Tamsin"), None, None).unwrap();

    let c2 = project::create_document(&root, Some(&act.id), "2. The Descent", DocKind::Scene).unwrap();
    project::write_body(&root, &c2.id, "The descent took the leg in the second hour. The surgeon splinted it badly.").unwrap();
    project::update_meta(&root, &c2.id, Some("His leg breaks."), Some("Character"), Some("Draft")).unwrap();
    project::update_extended(&root, &c2.id, Some("Takes place on Day 12."), None, Some("Tamsin"), None, None).unwrap();

    // Deliberate reuse of chapter one's opening.
    let c3 = project::create_document(&root, Some(&act.id), "3. The Second Well", DocKind::Scene).unwrap();
    project::write_body(&root, &c3.id, &format!("{opening}\n\nA year later, and she wrote it again.")).unwrap();
    project::update_meta(&root, &c3.id, Some("An echo."), Some("Plot"), Some("Draft")).unwrap();
    project::update_extended(&root, &c3.id, Some("Takes place on Day 4."), None, Some("Tamsin"), None, None).unwrap();

    let sheet = project::create_sheet(&root, "Tamsin", DocKind::Sheet, None, Some(&c1.id)).unwrap();
    project::link(&root, &sheet.id, &c2.id, "appears_in").unwrap();
    let carto = project::create_sheet(&root, "The Cartographer", DocKind::Sheet, None, Some(&c2.id)).unwrap();
    let _ = carto;

    Fixture { parent, root }
}

fn scene_ids(root: &std::path::Path) -> Vec<String> {
    let mut d: Vec<_> = project::documents(root)
        .unwrap()
        .into_iter()
        .filter(|x| x.kind == DocKind::Scene && x.word_count > 0)
        .collect();
    d.sort_by_key(|x| x.position);
    d.into_iter().map(|x| x.id).collect()
}

// --- Phase 1 and 2 ----------------------------------------------------------

#[test]
fn e2e_project_survives_without_the_database() {
    let f = fixture("survive");
    let ids = scene_ids(&f.root);
    let doc = project::documents(&f.root).unwrap().into_iter().find(|d| d.id == ids[0]).unwrap();

    // The promise the whole product rests on: prose is readable without us.
    let on_disk = std::fs::read_to_string(f.root.join(doc.path.unwrap())).unwrap();
    assert!(on_disk.contains("The well at Kharsa"));
    assert!(f.root.join("project.sqlite").exists());
}

#[test]
fn e2e_snapshot_restore_round_trip_on_a_real_project() {
    let f = fixture("snap");
    let ids = scene_ids(&f.root);
    let doc = project::documents(&f.root).unwrap().into_iter().find(|d| d.id == ids[0]).unwrap();
    let rel = doc.path.clone().unwrap();

    let first = snapshots::create(&f.root, "Before edits").unwrap().expect("a snapshot");
    project::write_body(&f.root, &ids[0], "Completely rewritten.").unwrap();
    snapshots::create(&f.root, "After edits").unwrap().expect("a second snapshot");

    snapshots::restore_file(&f.root, &first.id, &rel).unwrap();
    assert!(project::read_body(&f.root, &ids[0]).unwrap().contains("The well at Kharsa"));

    // Structure is captured too, so a restore is not prose-only.
    let listed = snapshots::list(&f.root, None, 20).unwrap();
    assert!(listed.iter().any(|s| s.files.iter().any(|x| x == "project.sqlite")));
}

// --- Phase 6 ----------------------------------------------------------------

#[test]
fn e2e_smart_collection_tracks_metadata_on_a_real_project() {
    let f = fixture("coll");
    let filter = serde_json::to_string(&SmartFilter { status: Some("Draft".into()), ..Default::default() }).unwrap();
    let conn = store::open(&f.root).unwrap();
    collections::create(&conn, "c1", "Needs revision", CollectionKind::Smart, Some(&filter), 0).unwrap();
    assert_eq!(collections::members(&conn, "c1").unwrap().len(), 2, "two chapters start as Draft");
    drop(conn);

    // Promote one to Done; it leaves the collection without moving a file.
    let ids = scene_ids(&f.root);
    let doc = project::documents(&f.root).unwrap().into_iter().find(|d| d.id == ids[1]).unwrap();
    let path_before = doc.path.clone().unwrap();
    project::update_meta(&f.root, &ids[1], None, None, Some("Done")).unwrap();

    let conn = store::open(&f.root).unwrap();
    assert_eq!(collections::members(&conn, "c1").unwrap().len(), 1);
    let after = project::documents(&f.root).unwrap().into_iter().find(|d| d.id == ids[1]).unwrap();
    assert_eq!(after.path.unwrap(), path_before, "the file never moved");
}

#[test]
fn e2e_scrivenings_edit_writes_back_to_the_right_files() {
    let f = fixture("scriv");
    let act = project::documents(&f.root).unwrap().into_iter().find(|d| d.title == "Act I").unwrap();
    let mut kids: Vec<_> = project::documents(&f.root)
        .unwrap()
        .into_iter()
        .filter(|d| d.parent_id.as_deref() == Some(act.id.as_str()))
        .collect();
    kids.sort_by_key(|d| d.position);

    let pieces: Vec<scrivenings::Piece> = kids
        .iter()
        .map(|k| scrivenings::Piece { id: k.id.clone(), body: project::read_body(&f.root, &k.id).unwrap() })
        .collect();
    let ids: Vec<String> = pieces.iter().map(|p| p.id.clone()).collect();

    // Edit only the middle document through the combined view.
    let edited = scrivenings::stitch(&pieces).replace("The surgeon splinted it badly.", "The surgeon splinted it well enough.");
    let split = scrivenings::split(&edited, &ids).unwrap();
    for p in &split {
        project::write_body(&f.root, &p.id, &p.body).unwrap();
    }

    assert!(project::read_body(&f.root, &ids[1]).unwrap().contains("well enough"));
    assert!(project::read_body(&f.root, &ids[0]).unwrap().contains("The well at Kharsa"), "the first document was untouched");
    assert!(project::read_body(&f.root, &ids[2]).unwrap().contains("A year later"), "the third document was untouched");

    // A mangled boundary refuses rather than writing half the chapter.
    let mangled = edited.replacen(scrivenings::SEPARATOR, " ", 1);
    assert!(scrivenings::split(&mangled, &ids).is_err());
}

// --- Phase 7 ----------------------------------------------------------------

#[test]
fn e2e_compile_produces_every_format_from_real_files() {
    let f = fixture("compile");
    let filters = CompileFilters { skip_empty: true, ..Default::default() };

    let mut items = Vec::new();
    for doc in project::documents(&f.root).unwrap() {
        if !compile::included(&doc, &filters) {
            continue;
        }
        let body = project::read_body(&f.root, &doc.id).unwrap();
        items.push(compile::Item { doc, body });
    }
    assert_eq!(items.len(), 3, "three scenes with prose");

    let preset = CompilePreset::named("Standard Manuscript");
    for fmt in [
        ExportFormat::Markdown,
        ExportFormat::Html,
        ExportFormat::Fountain,
        ExportFormat::Epub,
        ExportFormat::Docx,
    ] {
        let bytes = compile::render(&items, &preset, fmt).unwrap();
        let out = f.root.join(format!("export.{}", fmt.extension()));
        std::fs::write(&out, &bytes).unwrap();
        assert!(out.exists() && std::fs::metadata(&out).unwrap().len() > 0, "{fmt:?} wrote nothing");
    }

    // Source files are untouched by compiling.
    let ids = scene_ids(&f.root);
    assert!(project::read_body(&f.root, &ids[0]).unwrap().contains("The well at Kharsa"));
}

// --- Phase 4 ----------------------------------------------------------------

#[test]
fn e2e_integrity_finds_the_paragraph_the_fixture_reused() {
    let f = fixture("integrity");
    let ids = scene_ids(&f.root);
    let subject = project::read_body(&f.root, &ids[2]).unwrap();

    let mut sources = Vec::new();
    for d in project::documents(&f.root).unwrap() {
        if d.id == ids[2] || d.path.is_none() {
            continue;
        }
        sources.push((d.id.clone(), d.title.clone(), project::read_body(&f.root, &d.id).unwrap()));
    }

    let hits = integrity::internal_similarity(&ids[2], &subject, &sources, 0.6, 5);
    assert!(!hits.is_empty(), "the deliberately reused opening should be found");
    assert_eq!(hits[0].source_title, "1. The Well");
    assert!(hits[0].score > 0.9);

    let rubric = integrity::rubric(&subject);
    assert_eq!(rubric.len(), 4);
    assert!(rubric.iter().all(|r| (0..=100).contains(&r.score)));
}

// --- Phases 8 and 9 ---------------------------------------------------------

#[test]
fn e2e_analysis_reads_the_real_binder() {
    let f = fixture("analysis");
    let docs = project::documents(&f.root).unwrap();

    let beats = analysis::plot_beats(&docs, "three_act");
    assert_eq!(beats.len(), 3);
    assert!(beats[0].position < beats[1].position && beats[1].position < beats[2].position);

    // Story days are 1, 12, 4 in narrative order, so chronology reorders them.
    let narrative = analysis::timeline(&docs, false);
    let chrono = analysis::timeline(&docs, true);
    assert_eq!(narrative.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), vec!["1. The Well", "2. The Descent", "3. The Second Well"]);
    assert_eq!(chrono.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), vec!["1. The Well", "3. The Second Well", "2. The Descent"]);
    assert!(chrono.iter().any(|e| e.out_of_order), "moved scenes are flagged");
    assert_eq!(analysis::timeline_gaps(&chrono, 5).len(), 1, "day 4 to day 12 is an eight-day gap");

    // Tamsin appears in two scenes, the cartographer in one of them.
    let graph = analysis::character_graph(&docs);
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.nodes.iter().find(|n| n.label == "Tamsin").unwrap().weight, 2);
    assert_eq!(graph.edges.len(), 1, "they share exactly one scene");

    let mut bodies = HashMap::new();
    for d in &docs {
        if d.path.is_some() {
            bodies.insert(d.id.clone(), project::read_body(&f.root, &d.id).unwrap());
        }
    }
    assert_eq!(analysis::pacing(&docs, &bodies).len(), 3);

    let clusters = analysis::suggest_clusters(&docs, &bodies, 0.5);
    assert!(!clusters.is_empty(), "the two chapters sharing an opening should cluster");
    assert_eq!(clusters[0].document_ids.len(), 2);
}

// --- Phase 5 ----------------------------------------------------------------

#[test]
fn e2e_agent_gateway_drives_the_real_project() {
    let f = fixture("agent");

    let listed = agent::dispatch(&f.root, "list_documents", &json!({})).unwrap();
    assert!(listed.value["count"].as_u64().unwrap() >= 3);

    let made = agent::dispatch(&f.root, "create_folder", &json!({ "name": "Act II" })).unwrap();
    let folder_id = made.value["created"]["id"].as_str().unwrap().to_string();
    agent::dispatch(&f.root, "create_chapter", &json!({ "parent_id": folder_id, "count": 2 })).unwrap();

    let docs = project::documents(&f.root).unwrap();
    assert_eq!(docs.iter().filter(|d| d.parent_id.as_deref() == Some(&folder_id)).count(), 2);

    // The gateway's similarity tool sees the same reuse the module does.
    let ids = scene_ids(&f.root);
    let sim = agent::dispatch(&f.root, "check_internal_similarity", &json!({ "id": ids[2], "threshold": 0.6 })).unwrap();
    assert!(!sim.value["matches"].as_array().unwrap().is_empty());

    // And a destructive tool still refuses without a human's confirmation.
    assert!(agent::dispatch(&f.root, "delete_document", &json!({ "id": ids[0] })).is_err());
    assert!(project::documents(&f.root).unwrap().iter().any(|d| d.id == ids[0]), "nothing was deleted");
}

// --- Phase 3, live -----------------------------------------------------------

/// Drives the assist path exactly as the UI does: read a real document, send it
/// to a real local model, get prose back. Skips loudly when Ollama is absent.
#[test]
fn e2e_assist_generates_from_a_real_document() {
    use aegisdraft_lib::ollama;

    let status = ollama::status();
    if !status.reachable {
        eprintln!("SKIP: Ollama not reachable — assist not exercised");
        return;
    }
    // Use the generation pool: an embedding model is small enough to sort first
    // and returns nothing when asked for prose.
    let Some(model) = ollama::generation_models(&status).first().map(|m| m.name.clone()) else {
        eprintln!("SKIP: no local generation model installed");
        return;
    };

    let f = fixture("assist");
    let ids = scene_ids(&f.root);
    let body = project::read_body(&f.root, &ids[0]).unwrap();
    assert!(!body.trim().is_empty());

    let out = ollama::generate(
        &model,
        &format!("Continue this scene:\n{body}"),
        Some("You are a novelist continuing an existing scene. Match the voice exactly. Return only prose."),
        120,
        0.7,
    )
    .expect("generation should succeed against a running Ollama");

    eprintln!("EVIDENCE e2e_assist: model={model} produced {} chars", out.trim().len());
    eprintln!("EVIDENCE e2e_assist first 160: {:?}", out.trim().chars().take(160).collect::<String>());
    assert!(!out.trim().is_empty(), "assist must return prose");

    // The document on disk is untouched: generation never writes.
    assert_eq!(project::read_body(&f.root, &ids[0]).unwrap(), body, "assist must not modify the manuscript");
}

/// Compile writes a real file to a real path, and leaves sources alone.
#[test]
fn e2e_compile_writes_files_a_reader_can_open() {
    let f = fixture("export");
    let filters = CompileFilters { skip_empty: true, ..Default::default() };
    let mut items = Vec::new();
    for doc in project::documents(&f.root).unwrap() {
        if !compile::included(&doc, &filters) {
            continue;
        }
        let body = project::read_body(&f.root, &doc.id).unwrap();
        items.push(compile::Item { doc, body });
    }

    let out = f.parent.join("book.epub");
    std::fs::write(&out, compile::render(&items, &CompilePreset::named("Ebook"), ExportFormat::Epub).unwrap()).unwrap();

    // Re-open it the way a reader's software would.
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
    assert!(names.contains(&"OEBPS/content.opf".to_string()));
    let mut opf = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("OEBPS/content.opf").unwrap(), &mut opf).unwrap();
    assert!(opf.contains("<dc:language>en</dc:language>"));
    eprintln!("EVIDENCE e2e_compile: {} bytes, {} entries", std::fs::metadata(&out).unwrap().len(), names.len());
}

/// The whole Phase 3 retrieval loop against a real project and a real embedding
/// model: chunk every document, embed, store, then ask a question whose answer
/// appears in only one scene and check that scene comes back first.
#[test]
fn e2e_index_and_search_a_real_project_with_live_embeddings() {
    use aegisdraft_lib::{ollama, rag};

    let status = ollama::status();
    if !status.reachable {
        eprintln!("SKIP: Ollama not reachable — indexing not exercised");
        return;
    }
    let Some(model) = ollama::embedding_models(&status).first().map(|m| m.name.clone()) else {
        eprintln!("SKIP: no embedding model installed — `ollama pull nomic-embed-text`");
        return;
    };

    let f = fixture("index");

    // Index exactly as the `index_project` command does.
    let docs = project::documents(&f.root).unwrap();
    let mut conn = store::open(&f.root).unwrap();
    let mut chunks_total = 0usize;
    let mut docs_total = 0usize;
    for doc in docs.iter().filter(|d| d.path.is_some() && d.word_count > 0) {
        let body = project::read_body(&f.root, &doc.id).unwrap();
        let texts = rag::chunk(&body, 220, 40).unwrap();
        if texts.is_empty() {
            continue;
        }
        let vectors = ollama::embed(&model, &texts).unwrap();
        let pairs: Vec<(String, Vec<f32>)> = texts.into_iter().zip(vectors).collect();
        chunks_total += rag::index_document(&mut conn, &doc.id, &pairs, &model, 0).unwrap();
        docs_total += 1;
    }
    let (stored_chunks, stored_docs) = rag::index_stats(&conn).unwrap();
    eprintln!("EVIDENCE index: model={model} docs={docs_total} chunks={chunks_total} stored=({stored_chunks},{stored_docs})");
    assert!(chunks_total > 0, "indexing produced no chunks");
    assert_eq!(stored_chunks as usize, chunks_total);

    // A question phrased nothing like the text that answers it.
    let q = ollama::embed(&model, &["Who was hurt, and who treated the injury?".to_string()]).unwrap();
    let hits = rag::search(&conn, &q[0], 3, None).unwrap();
    assert!(!hits.is_empty());
    eprintln!("EVIDENCE search: top={:?} score={:.4}", hits[0].text.chars().take(90).collect::<String>(), hits[0].score);
    assert!(
        hits[0].text.contains("descent") || hits[0].text.contains("surgeon"),
        "retrieval should surface the injury scene, got: {:?}",
        hits[0].text
    );

    // Scores are ordered and finite — a NaN here would silently scramble ranking.
    for pair in hits.windows(2) {
        assert!(pair[0].score >= pair[1].score, "results must be ranked");
    }
    assert!(hits.iter().all(|h| h.score.is_finite()));

    // Re-indexing one document replaces its chunks rather than duplicating.
    let first = docs.iter().find(|d| d.path.is_some() && d.word_count > 0).unwrap();
    let body = project::read_body(&f.root, &first.id).unwrap();
    let texts = rag::chunk(&body, 220, 40).unwrap();
    let vectors = ollama::embed(&model, &texts).unwrap();
    let pairs: Vec<(String, Vec<f32>)> = texts.into_iter().zip(vectors).collect();
    rag::index_document(&mut conn, &first.id, &pairs, &model, 1).unwrap();
    assert_eq!(rag::index_stats(&conn).unwrap().0, stored_chunks, "reindex must not duplicate");
}

#[test]
fn e2e_importing_a_folder_of_markdown_mirrors_it_into_the_binder() {
    let f = fixture("import");

    // A folder as an author would actually have it: numbered chapters, a
    // sub-folder, an editor's dotfile, and a stray image.
    let source = f.parent.join("Old Draft");
    std::fs::create_dir_all(source.join("Act II")).unwrap();
    std::fs::create_dir_all(source.join(".obsidian")).unwrap();
    std::fs::write(
        source.join("01-the-crossing.md"),
        "# The Crossing\n\nThey went over at first light, and the river took two mules.\n",
    )
    .unwrap();
    std::fs::write(
        source.join("Act II/02-after.md"),
        "The town on the far bank had been burning for a week.\n",
    )
    .unwrap();
    std::fs::write(source.join("map.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();
    std::fs::write(source.join(".obsidian/workspace.json"), "{}").unwrap();

    let before = project::documents(&f.root).unwrap();
    let report = import::run(&f.root, &source, None, DocKind::Scene).unwrap();

    assert_eq!(report.documents, 2);
    assert_eq!(report.folders, 1);
    assert!(report.words > 0);

    let after = project::documents(&f.root).unwrap();
    assert_eq!(
        after.len(),
        before.len() + 4,
        "two documents, one sub-folder, and the folder the import landed in"
    );

    // The binder mirrors the tree: the sub-folder sits under the import folder,
    // and the scene sits under the sub-folder.
    let holder = after.iter().find(|d| d.id == report.root_id).unwrap();
    assert_eq!(holder.title, "Old Draft");
    let act = after
        .iter()
        .find(|d| d.title == "Act II")
        .expect("the sub-folder must exist in the binder");
    assert_eq!(act.parent_id.as_deref(), Some(holder.id.as_str()));
    let after_doc = after
        .iter()
        .find(|d| d.title == "After")
        .expect("a file with no heading takes its title from the filename");
    assert_eq!(after_doc.parent_id.as_deref(), Some(act.id.as_str()));

    // The prose is on disk inside the project, readable as a file.
    let crossing = after.iter().find(|d| d.title == "The Crossing").unwrap();
    let rel = crossing.path.clone().expect("an imported scene has a file");
    let on_disk = std::fs::read_to_string(f.root.join(&rel)).unwrap();
    assert!(on_disk.contains("the river took two mules"));

    // Everything that did not come in is named.
    assert!(report.skipped.iter().any(|s| s.contains("map.png")));

    // The import is inert for everything that was already there.
    for doc in &before {
        let still = after.iter().find(|d| d.id == doc.id).expect("a document vanished");
        assert_eq!(still.path, doc.path);
        assert_eq!(still.parent_id, doc.parent_id);
    }

    // And a compile after the import carries the imported prose, with the
    // source files on disk unmodified by the compile.
    let filters = CompileFilters { skip_empty: true, ..Default::default() };
    let mut items = Vec::new();
    for doc in project::documents(&f.root).unwrap() {
        if !compile::included(&doc, &filters) {
            continue;
        }
        let body = project::read_body(&f.root, &doc.id).unwrap();
        items.push(compile::Item { doc, body });
    }
    let bytes = compile::render(&items, &CompilePreset::named("Standard Manuscript"), ExportFormat::Markdown)
        .unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("the river took two mules"), "imported prose must compile");
    assert!(text.contains("The well at Kharsa"), "the original prose must survive the import");
    assert_eq!(std::fs::read_to_string(f.root.join(&rel)).unwrap(), on_disk);
}

// --- Phase 3 ----------------------------------------------------------------

#[test]
fn e2e_style_profile_is_built_from_finished_work_and_reaches_the_prompt() {
    let f = fixture("style");

    // The fixture marks chapter one Done and the others Draft, which is exactly
    // the signal the profile is supposed to read.
    let (_, documents, from_finished) = style::sample(&f.root).unwrap();
    assert!(from_finished, "the fixture has finished work to learn from");
    assert_eq!(documents, 1);

    // Too little prose is refused rather than measured — one short chapter is
    // not a style, and saying otherwise would be the whole failure mode here.
    let refused = style::build(&f.root, None, false, 0).unwrap_err().to_string();
    assert!(refused.contains("not enough prose"), "{refused}");

    // Give it enough finished prose to measure.
    let long = project::create_document(&f.root, None, "3. The Road", DocKind::Scene).unwrap();
    let paragraph = "The road ran north out of the town and then it stopped pretending to. \
        Ruts. Grit. A wheel rim someone had left where it fell. She walked it anyway, \
        counting the water in her head and getting a different answer each time.\n\n";
    project::write_body(&f.root, &long.id, &paragraph.repeat(12)).unwrap();
    project::update_meta(&f.root, &long.id, None, None, Some("Done")).unwrap();

    let profile = style::build(&f.root, None, false, 1_700_000_000_000).unwrap();
    assert!(profile.from_finished_work);
    assert_eq!(profile.sample_documents, 2);
    assert!(profile.sample_words >= 200);
    assert!(profile.signals.mean_sentence_length > 0.0);
    assert!(profile.vocabulary_range > 0.0);
    assert!(
        profile.voice.is_none(),
        "built without a model, so it must claim no description"
    );

    // It is a real file at the project root, so a snapshot carries it and the
    // author can read it in a text editor.
    style::save(&f.root, &profile).unwrap();
    let on_disk = f.root.join(style::FILE);
    assert!(on_disk.exists());
    let raw = std::fs::read_to_string(&on_disk).unwrap();
    assert!(raw.contains("meanSentenceLength"), "stored as readable JSON");
    assert!(style::load(&f.root).is_some());

    snapshots::create(&f.root, "with a style profile").unwrap().unwrap();
    let touching = snapshots::list(&f.root, Some(style::FILE), 10).unwrap();
    assert!(
        !touching.is_empty(),
        "the profile must travel with the prose in a snapshot, not be ignored"
    );
    assert_eq!(touching[0].message, "with a style profile");

    // And it turns into instructions, not a data dump.
    let fragment = style::prompt_fragment(&profile);
    assert!(fragment.contains("Sentences average about"));
    assert!(fragment.contains("Do not smooth it out"));

    // The open document is measured against it.
    let body = project::read_body(&f.root, &long.id).unwrap();
    let d = style::distance(&profile, &body);
    assert!(!d.too_short);
    assert!(d.overall > 60, "its own prose scored {}", d.overall);
    assert!(!d.notes.is_empty());
}

// --- Phase 4 ----------------------------------------------------------------

#[test]
fn e2e_integrity_packet_carries_the_method_with_the_numbers() {
    let f = fixture("packet");
    let ids = scene_ids(&f.root);
    // The fixture's third chapter deliberately reuses chapter one's opening.
    let subject = ids.last().unwrap();
    let body = project::read_body(&f.root, subject).unwrap();

    let docs = project::documents(&f.root).unwrap();
    let mut sources = Vec::new();
    for d in docs.iter().filter(|d| &d.id != subject && d.path.is_some()) {
        sources.push((d.id.clone(), d.title.clone(), project::read_body(&f.root, &d.id).unwrap()));
    }

    // A packet with no web check must say so, not leave the section blank.
    assert!(websearch::trace(&f.root).is_empty());

    let packet = integrity::IntegrityPacket {
        generated_at: 1_700_000_000_000,
        project: f.root.display().to_string(),
        document_id: subject.clone(),
        document_title: "3. The Return".into(),
        word_count: project::count_words(&body),
        internal_matches: integrity::internal_similarity(subject, &body, &sources, 0.5, 5),
        threshold: 0.5,
        signals: integrity::writing_signals(&body),
        rubric: integrity::rubric(&body),
        web_checks: websearch::trace(&f.root),
        method: integrity::method_notes(),
    };

    assert!(!packet.internal_matches.is_empty(), "the fixture's reuse must be found");
    assert!(!packet.rubric.is_empty());

    // The JSON half: a machine can read every number.
    let json = serde_json::to_string_pretty(&packet).unwrap();
    let back: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(!back["internalMatches"].as_array().unwrap().is_empty());
    assert_eq!(back["threshold"], 0.5);
    assert!(back["method"].as_array().unwrap().len() >= 4);

    // The HTML half: a person can read it, and it states what it does not know.
    let html = integrity::packet_html(&packet);
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("3. The Return"));
    assert!(
        html.contains("does not determine who or what wrote"),
        "a report that omits its own limits is the failure mode here"
    );
    assert!(
        html.contains("No web check has been run"),
        "an empty section must say why it is empty"
    );

    // Prose goes through the escaper, so a chapter title with markup in it
    // cannot break — or inject into — the report someone else opens.
    let mut hostile = packet.clone();
    hostile.document_title = "<script>alert(1)</script>".into();
    let escaped = integrity::packet_html(&hostile);
    assert!(!escaped.contains("<script>alert"), "the title must be escaped");
    assert!(escaped.contains("&lt;script&gt;"));

    // Writing the pair leaves two real files.
    let out = f.root.join("report.json");
    std::fs::write(&out, &json).unwrap();
    std::fs::write(out.with_extension("html"), &html).unwrap();
    assert!(out.exists() && out.with_extension("html").exists());
}

#[test]
fn e2e_a_web_check_is_refused_without_confirmation_at_the_gateway() {
    let f = fixture("webguard");
    let refused = agent::dispatch(
        &f.root,
        "search_web",
        &json!({ "phrase": "the well at Kharsa was the only honest thing" }),
    );
    let message = refused.unwrap_err().to_string();
    assert!(
        message.contains("needs confirmation"),
        "the one path that leaves the machine must be guarded: {message}"
    );
    // And nothing was recorded, because nothing was sent.
    assert!(websearch::trace(&f.root).is_empty());
}

// --- Phase 7 ----------------------------------------------------------------

#[test]
fn e2e_an_agent_can_compile_but_only_with_confirmation() {
    let f = fixture("agentcompile");
    let out = f.parent.join("agent-manuscript.md");

    // Unconfirmed: refused, and no file appears.
    let refused = agent::dispatch(
        &f.root,
        "compile_manuscript",
        &json!({ "path": out.to_string_lossy(), "format": "markdown" }),
    );
    assert!(refused.unwrap_err().to_string().contains("needs confirmation"));
    assert!(!out.exists(), "a refused compile must write nothing");

    // Confirmed: it really writes, and the prose is the project's own.
    let done = agent::dispatch(
        &f.root,
        "compile_manuscript",
        &json!({ "path": out.to_string_lossy(), "format": "markdown", "confirmed": true }),
    )
    .unwrap();
    assert!(done.ok);
    assert!(out.exists());
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("The well at Kharsa"));
    assert_eq!(done.value["documents"].as_i64().unwrap(), 3);

    // Compiling does not touch the sources.
    let ids = scene_ids(&f.root);
    assert!(project::read_body(&f.root, &ids[0]).unwrap().contains("The well at Kharsa"));

    // The schema tells a model the gate exists, and that it writes over things.
    let schema = agent::schema("compile_manuscript");
    assert!(schema["properties"]["confirmed"].is_object());
    assert!(
        schema["properties"]["path"]["description"]
            .as_str()
            .unwrap()
            .contains("Overwrites"),
        "a tool that overwrites must say so in its own schema"
    );
}

// --- Phase 8 ----------------------------------------------------------------

#[test]
fn e2e_the_canvas_commits_a_layout_back_to_the_real_binder() {
    let f = fixture("canvas");
    let docs = project::documents(&f.root).unwrap();
    let act = docs.iter().find(|d| d.title == "Act I").unwrap();
    let mut scenes: Vec<_> = docs
        .iter()
        .filter(|d| d.parent_id.as_deref() == Some(act.id.as_str()))
        .collect();
    scenes.sort_by_key(|d| d.position);
    assert_eq!(scenes.len(), 3);

    // Lay the three chapters out backwards, in one row.
    for (i, doc) in scenes.iter().rev().enumerate() {
        canvas::place(&f.root, &doc.id, (i as f32) * 200.0, 40.0).unwrap();
    }
    // The sidecar is a real file in the project, so a snapshot carries it.
    assert!(f.root.join(canvas::DIR).join(canvas::FILE).exists());

    // Planning changes nothing.
    let plan = canvas::plan(&f.root, Some(&act.id)).unwrap();
    assert_eq!(plan.moves.len(), 2);
    let unchanged: Vec<String> = project::documents(&f.root)
        .unwrap()
        .into_iter()
        .filter(|d| d.parent_id.as_deref() == Some(act.id.as_str()))
        .map(|d| d.title)
        .collect();
    assert_eq!(unchanged[0], scenes[0].title, "planning must not move anything");

    // Through the agent gateway: refused unconfirmed, and still nothing moved.
    let refused = agent::dispatch(&f.root, "canvas_to_binder", &json!({ "parent_id": act.id }));
    assert!(refused.unwrap_err().to_string().contains("needs confirmation"));

    // `canvas_plan` is not guarded — it is a dry run, and an agent should be
    // able to look before it asks.
    let looked = agent::dispatch(&f.root, "canvas_plan", &json!({ "parent_id": act.id })).unwrap();
    assert_eq!(looked.value["plan"]["moves"].as_array().unwrap().len(), 2);

    // Confirmed: the binder really reorders.
    agent::dispatch(
        &f.root,
        "canvas_to_binder",
        &json!({ "parent_id": act.id, "confirmed": true }),
    )
    .unwrap();
    let after: Vec<String> = project::documents(&f.root)
        .unwrap()
        .into_iter()
        .filter(|d| d.parent_id.as_deref() == Some(act.id.as_str()))
        .map(|d| d.title)
        .collect();
    let expected: Vec<String> = scenes.iter().rev().map(|d| d.title.clone()).collect();
    assert_eq!(after, expected);

    // The prose is untouched — a reorder moves rows, not files' contents.
    for doc in &scenes {
        let body = project::read_body(&f.root, &doc.id).unwrap();
        assert!(!body.is_empty(), "{} lost its prose in a reorder", doc.title);
    }

    // And it is walk-back-able.
    let history = snapshots::list(&f.root, None, 10).unwrap();
    assert!(history.iter().any(|s| s.message.contains("Before committing the canvas")));
}

// --- Block identity and provenance ------------------------------------------

/// UF-01: write, edit, reopen. Ids survive; only the changed block moves on.
#[test]
fn e2e_uf01_editing_a_paragraph_keeps_every_other_block_identical() {
    let f = fixture("uf01");
    let doc = project::create_document(&f.root, None, "Chapter", DocKind::Scene).unwrap();

    let one = "The well at Kharsa was the only honest thing in the town and it was going dry.";
    let two = "She counted what she had left, and then she counted it again more slowly.";
    let three = "Below them the salt went on being white in the dark, mile after mile of it.";
    project::write_body(&f.root, &doc.id, &format!("{one}\n\n{two}\n\n{three}")).unwrap();

    let conn = store::open(&f.root).unwrap();
    let first = blocks::list(&conn, &doc.id).unwrap();
    assert_eq!(first.len(), 3);
    assert!(first.iter().all(|b| b.revision == 1));
    let ids: Vec<String> = first.iter().map(|b| b.id.clone()).collect();
    drop(conn);

    // Edit only the middle paragraph.
    let two_edited = "She counted what she had left, and then she counted it again twice more.";
    project::write_body(&f.root, &doc.id, &format!("{one}\n\n{two_edited}\n\n{three}")).unwrap();

    let conn = store::open(&f.root).unwrap();
    let after = blocks::list(&conn, &doc.id).unwrap();
    assert_eq!(after.iter().map(|b| b.id.clone()).collect::<Vec<_>>(), ids, "ids must not move");
    assert_eq!(after[0].revision, 1);
    assert_eq!(after[1].revision, 2, "only the edited paragraph advances");
    assert_eq!(after[2].revision, 1);

    // Every event so far was watched happening.
    let chain = provenance::history(&conn, &ids[1]).unwrap();
    assert_eq!(chain.len(), 2);
    assert_eq!(chain[0].action, provenance::Action::Create);
    assert_eq!(chain[1].action, provenance::Action::Rewrite);
    assert!(chain.iter().all(|e| e.confidence == provenance::Confidence::Observed));
    assert!(provenance::verify_chain(&conn).unwrap().is_none());
}

/// UF-02: the vim case. External edits keep ids and are recorded as inferred;
/// a reflow is not an edit at all.
#[test]
fn e2e_uf02_an_edit_made_outside_the_app_is_recorded_as_inferred() {
    let f = fixture("uf02");
    let doc = project::create_document(&f.root, None, "Chapter", DocKind::Scene).unwrap();
    let one = "The road ran north out of the town and then it stopped pretending to be a road.";
    let two = "Ruts and grit and a wheel rim someone had left lying exactly where it fell.";
    project::write_body(&f.root, &doc.id, &format!("{one}\n\n{two}")).unwrap();

    let conn = store::open(&f.root).unwrap();
    let before = blocks::list(&conn, &doc.id).unwrap();
    let ids: Vec<String> = before.iter().map(|b| b.id.clone()).collect();
    drop(conn);

    let file = f.root.join(project::documents(&f.root).unwrap()
        .into_iter().find(|d| d.id == doc.id).unwrap().path.unwrap());

    // A reflow only: the same words, hard-wrapped. This must produce nothing.
    let wrapped = format!("The road ran north out of the town\nand then it stopped pretending to be a road.\n\n{two}");
    std::fs::write(&file, &wrapped).unwrap();
    project::reconcile_all(&f.root).unwrap();

    let conn = store::open(&f.root).unwrap();
    assert_eq!(
        provenance::history(&conn, &ids[0]).unwrap().len(),
        1,
        "hard-wrapping in an external editor is not a rewrite"
    );
    drop(conn);

    // Now a real external edit.
    let edited = "The road ran north out of the town and then it stopped pretending to be one.";
    std::fs::write(&file, format!("{edited}\n\n{two}")).unwrap();
    project::reconcile_all(&f.root).unwrap();

    let conn = store::open(&f.root).unwrap();
    let after = blocks::list(&conn, &doc.id).unwrap();
    assert_eq!(after[0].id, ids[0], "the id must survive an edit made in another editor");
    let chain = provenance::history(&conn, &ids[0]).unwrap();
    let last = chain.last().unwrap();
    assert_eq!(last.actor_type, provenance::ActorType::External);
    assert_eq!(
        last.confidence,
        provenance::Confidence::Inferred,
        "we did not watch this happen, and the ledger must not pretend we did"
    );
    let who = provenance::classify(&chain);
    assert!(who.uncertain);
    assert!(who.disclosure_line.contains("reconstructed"));
}

/// UF-03: imported prose is external material, not the author's writing.
#[test]
fn e2e_uf03_imported_prose_is_attributed_to_the_import() {
    let f = fixture("uf03");
    let source = f.parent.join("Old Draft");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("01-crossing.md"),
        "# The Crossing\n\nThey went over at first light, and the river took two of the mules.\n",
    )
    .unwrap();

    import::run(&f.root, &source, None, DocKind::Scene).unwrap();

    let conn = store::open(&f.root).unwrap();
    let doc = project::documents(&f.root).unwrap().into_iter()
        .find(|d| d.title == "The Crossing").unwrap();
    let imported = blocks::list(&conn, &doc.id).unwrap();
    assert!(!imported.is_empty());
    for b in &imported {
        let who = provenance::classify(&provenance::history(&conn, &b.id).unwrap());
        assert_eq!(who.origin, provenance::Origin::External, "block {}", b.id);
        assert!(!who.unrecorded);
    }
}

/// NEG-14 / UF-06: the laundering guard. An agent's prose must never be
/// recorded as the author's own work.
#[test]
fn e2e_uf06_an_agent_write_is_never_recorded_as_human_authorship() {
    let f = fixture("uf06");
    let actor = provenance::Actor::model("llama3.2:3b", json!({"via": "test"}));

    agent::dispatch_as(
        &f.root,
        "save_to_binder",
        &json!({
            "title": "Desert lore",
            "category": "Worldbuilding",
            "content": "The salt flats west of Kharsa were a sea once, and the maps still say so."
        }),
        actor,
    )
    .unwrap();

    let conn = store::open(&f.root).unwrap();
    let doc = project::documents(&f.root).unwrap().into_iter()
        .find(|d| d.title == "Desert lore").unwrap();
    let written = blocks::list(&conn, &doc.id).unwrap();
    assert!(!written.is_empty());

    for b in &written {
        let chain = provenance::history(&conn, &b.id).unwrap();
        assert!(!chain.is_empty(), "agent-written prose must leave a record");
        assert!(
            chain.iter().all(|e| e.actor_type != provenance::ActorType::Human),
            "an agent's writing recorded as the author's own is the worst failure this \
             ledger can have: {chain:?}"
        );
        let who = provenance::classify(&chain);
        assert_eq!(who.origin, provenance::Origin::Ai);
        assert_eq!(who.models, vec!["llama3.2:3b"]);
    }

    // And the default path still attributes to the author, so the parameter is
    // doing the work rather than everything being marked as a model.
    let mine = project::create_document(&f.root, None, "Mine", DocKind::Scene).unwrap();
    project::write_body(&f.root, &mine.id, "I wrote this sentence myself, at the keyboard.").unwrap();
    let conn = store::open(&f.root).unwrap();
    let block = &blocks::list(&conn, &mine.id).unwrap()[0];
    assert_eq!(
        provenance::classify(&provenance::history(&conn, &block.id).unwrap()).origin,
        provenance::Origin::Human
    );
}

/// PV-18 at project scale, and MIG-02: a pre-existing manuscript is honest
/// about having no record rather than claiming its author wrote it.
#[test]
fn e2e_a_mixed_manuscript_separates_who_wrote_what() {
    let f = fixture("mixed");

    // Prose that arrived without going through the app — a document written
    // before the ledger existed, or edited in another editor. This is what a
    // real upgrade looks like, and it must never be claimed as anyone's work.
    let legacy = project::create_document(&f.root, None, "Legacy", DocKind::Scene).unwrap();
    let legacy_file = f.root.join(
        project::documents(&f.root).unwrap().into_iter()
            .find(|d| d.id == legacy.id).unwrap().path.unwrap(),
    );
    std::fs::write(
        &legacy_file,
        "A paragraph that predates the ledger entirely and has no recorded author at all.",
    )
    .unwrap();
    project::reconcile_all(&f.root).unwrap();

    let conn = store::open(&f.root).unwrap();
    let legacy_blocks = blocks::list(&conn, &legacy.id).unwrap();
    assert_eq!(legacy_blocks.len(), 1);
    let who = provenance::classify(&provenance::history(&conn, &legacy_blocks[0].id).unwrap());
    assert_eq!(
        who.origin,
        provenance::Origin::Unknown,
        "text that appeared without being observed has no known author"
    );
    assert!(who.uncertain, "and the ledger says the lineage was reconstructed");
    drop(conn);

    // Now add one of each kind on top.
    let mine = project::create_document(&f.root, None, "Mine", DocKind::Scene).unwrap();
    project::write_body(&f.root, &mine.id, "A sentence I typed myself into the manuscript.").unwrap();
    agent::dispatch_as(
        &f.root,
        "save_to_binder",
        &json!({"title": "Theirs", "content": "A sentence a model wrote into the manuscript."}),
        provenance::Actor::model("m", json!({})),
    )
    .unwrap();

    let conn = store::open(&f.root).unwrap();
    let s = provenance::summarise(&conn, None).unwrap();
    assert_eq!(s.ai, 1, "one block a model wrote");
    assert!(s.human >= 1, "at least the sentence typed at the keyboard");
    assert!(s.unknown >= 1, "and the legacy paragraph, claimed by nobody");
    assert_eq!(
        s.blocks,
        s.human + s.ai + s.external + s.mixed + s.unknown,
        "every block is counted exactly once"
    );
    assert!(s.chain_intact);
    assert_eq!(s.models, vec!["m"]);
}

/// UF-11: K2P-OS's "P-184 across 47 revisions". One id, a long life.
#[test]
fn e2e_uf11_one_block_survives_fifty_revisions() {
    let f = fixture("uf11");
    let doc = project::create_document(&f.root, None, "Long", DocKind::Scene).unwrap();
    let stem = "The rim of the basin held the last of the heat the way a kiln holds it after the fire";

    project::write_body(&f.root, &doc.id, &format!("{stem} goes out.")).unwrap();
    let conn = store::open(&f.root).unwrap();
    let id = blocks::list(&conn, &doc.id).unwrap()[0].id.clone();
    drop(conn);

    for i in 1..=50 {
        project::write_body(&f.root, &doc.id, &format!("{stem} goes out, revision {i}.")).unwrap();
    }

    let conn = store::open(&f.root).unwrap();
    let live = blocks::list(&conn, &doc.id).unwrap();
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].id, id, "fifty revisions, one identity");
    assert_eq!(live[0].revision, 51, "the block counts every save");

    // The ledger does not. Fifty saves inside one sitting are one editing
    // session, not fifty events — the editor autosaves every 800ms, and a
    // per-save ledger would run to millions of rows over a book.
    let chain = provenance::history(&conn, &id).unwrap();
    assert!(
        chain.len() < 5,
        "a continuous session must not become {} events",
        chain.len()
    );
    assert_eq!(chain[0].action, provenance::Action::Create);

    // What was lost is only the intermediate states. Who wrote it, when they
    // started, and what it says now are all still answerable.
    assert_eq!(
        chain[0].parent_revision,
        Some(1),
        "the event records the revision it started from"
    );
    assert!(provenance::verify_chain(&conn).unwrap().is_none());
    assert_eq!(
        provenance::classify(&chain).origin,
        provenance::Origin::Human
    );
}

/// UF-10: tampering is detected, and the project still opens.
#[test]
fn e2e_uf10_a_tampered_ledger_is_detected_and_the_app_still_works() {
    let f = fixture("tamper");
    let doc = project::create_document(&f.root, None, "Chapter", DocKind::Scene).unwrap();
    project::write_body(&f.root, &doc.id, "A sentence that a person wrote at a keyboard today.").unwrap();
    project::write_body(&f.root, &doc.id, "A sentence that a person wrote at a keyboard on Tuesday.").unwrap();

    let conn = store::open(&f.root).unwrap();
    assert!(provenance::verify_chain(&conn).unwrap().is_none());
    // Rewrite history to claim a model's work was the author's.
    conn.execute("UPDATE events SET actor_type = 'ai_model' WHERE seq = 1", []).unwrap();
    let broken = provenance::verify_chain(&conn).unwrap().expect("tampering must be caught");
    assert_eq!(broken.seq, 1);
    drop(conn);

    // The project still opens and still works.
    project::open(&f.root).unwrap();
    let docs = project::documents(&f.root).unwrap();
    assert!(docs.iter().any(|d| d.id == doc.id));
    let conn = store::open(&f.root).unwrap();
    assert!(!provenance::summarise(&conn, None).unwrap().chain_intact);
}

// --- Knowledge to Publication -----------------------------------------------

/// The chain the architecture is built on: a source becomes claims, a claim
/// becomes something a paragraph cites, and a retracted source can then name
/// every paragraph that depends on it.
#[test]
fn e2e_a_source_becomes_claims_that_a_paragraph_can_cite() {
    let f = fixture("k2p");

    // 1. Register two sources that disagree.
    let paper = f.parent.join("smith-2025.md");
    std::fs::write(
        &paper,
        "Constraint Satisfaction in Practice\nSmith, J. and Okafor, A.\n2025\n\ndoi:10.5555/abcd\n\n         We found that symbolic constraints improve consistency by a wide margin in every \
         configuration we tested during the study.",
    )
    .unwrap();
    let (smith, dup) = sources::register_file(&f.root, &paper, 1).unwrap();
    assert!(!dup);
    assert_eq!(smith.doi.as_deref(), Some("10.5555/abcd"));
    assert_eq!(smith.year, Some(2025));

    let other = f.parent.join("lin-2026.md");
    std::fs::write(
        &other,
        "A Replication Study\nLin, R.\n2026\n\n         We found that symbolic constraints do not improve consistency in any configuration \
         we tested during our replication of the original study.",
    )
    .unwrap();
    let (lin, _) = sources::register_file(&f.root, &other, 2).unwrap();

    // The extracted text is a real file inside the project.
    assert!(f.root.join(smith.text_path.clone().unwrap()).exists());
    // And a citation is generated, never typed.
    let cite = sources::citation(&smith);
    assert!(cite.contains("(2025)") && cite.contains("doi.org/10.5555/abcd"), "{cite}");

    // 2. Claims, extracted and verified against the source text.
    let conn = store::open(&f.root).unwrap();
    knowledge::save_extraction(
        &conn,
        &knowledge::validate_extraction(
            r#"{"concepts":[{"name":"Symbolic constraints"}],
                "claims":[{"text":"Symbolic constraints improve consistency in every configuration",
                           "kind":"result",
                           "excerpt":"symbolic constraints improve consistency by a wide margin",
                           "concepts":["Symbolic constraints"]}]}"#,
            &sources::read_text(&f.root, &smith).unwrap(),
            &smith.id,
            "test-model",
            10,
        )
        .unwrap(),
    )
    .unwrap();
    knowledge::save_extraction(
        &conn,
        &knowledge::validate_extraction(
            r#"{"claims":[{"text":"Symbolic constraints do not improve consistency in any configuration",
                           "kind":"result",
                           "excerpt":"symbolic constraints do not improve consistency in any configuration",
                           "concepts":[]}]}"#,
            &sources::read_text(&f.root, &lin).unwrap(),
            &lin.id,
            "test-model",
            11,
        )
        .unwrap(),
    )
    .unwrap();

    let all = knowledge::claims(&conn, None).unwrap();
    assert_eq!(all.len(), 2);

    // 3. The two disagreeing claims are candidates for a contradiction check,
    //    and it costs no model call to find that out.
    let pairs = knowledge::candidate_pairs(&all);
    assert!(!pairs.is_empty(), "claims about the same thing from different sources");

    // 4. A paragraph cites one of them.
    let chapter = project::create_document(&f.root, None, "Chapter", DocKind::Scene).unwrap();
    drop(conn);
    project::write_body(
        &f.root,
        &chapter.id,
        "Symbolic constraints have been reported to improve consistency across configurations.",
    )
    .unwrap();

    let conn = store::open(&f.root).unwrap();
    let block = blocks::list(&conn, &chapter.id).unwrap()[0].id.clone();
    let smith_claim = all.iter().find(|c| c.source_id == smith.id).unwrap();
    knowledge::cite(&conn, &block, &smith_claim.id, 20).unwrap();

    // 5. The retraction query: if Smith is withdrawn, what depends on it?
    let affected = knowledge::blocks_depending_on(&conn, &smith.id).unwrap();
    assert_eq!(affected, vec![block.clone()]);
    assert!(knowledge::blocks_depending_on(&conn, &lin.id).unwrap().is_empty());

    // 6. Citing is itself an event, so the paragraph's lineage shows it.
    let chain = provenance::history(&conn, &block).unwrap();
    assert!(chain.iter().any(|e| e.source_refs.contains(&smith_claim.id)));

    // 7. Gaps name what is thin, with no model involved.
    let gaps = knowledge::gaps(&conn).unwrap();
    assert!(gaps.iter().any(|g| g.kind == "single_source_concept"));
}

/// Retrieval and the brief work with no model and no network at all.
#[test]
fn e2e_research_retrieval_and_briefing_need_no_model() {
    let f = fixture("research");

    sources::register_text(
        &f.root,
        "Replication basics",
        "Logical replication is a method of replicating data objects and their changes based upon \
         their replication identity, which is usually a primary key.",
        sources::SourceKind::Text,
        None,
        None,
        1,
    )
    .unwrap();
    sources::register_text(
        &f.root,
        "Harvest records",
        "The wheat harvest failed in the third year of the drought and the granaries emptied.",
        sources::SourceKind::Text,
        None,
        None,
        2,
    )
    .unwrap();

    let hits = research::local_passages(&f.root, "what does logical replication replicate", 5).unwrap();
    assert!(!hits.is_empty());
    assert_eq!(hits[0].source_title, "Replication basics");
    assert!(!hits[0].from_web, "nothing left this machine");

    // The same path the agent gateway offers, unguarded because it is local.
    let found = agent::dispatch(
        &f.root,
        "search_sources",
        &json!({ "query": "logical replication identity", "limit": 3 }),
    )
    .unwrap();
    assert!(!found.value["passages"].as_array().unwrap().is_empty());

    // A brief with no evidence says so rather than writing around it.
    let empty = research::Brief {
        question: "What is known about something absent?".into(),
        plan: vec![],
        findings: vec![],
        contradictions: vec![],
        synthesis: None,
        bibliography: vec![],
        model: "none".into(),
        used_web: false,
        at: 0,
        method: "Nothing left this machine.".into(),
    };
    let md = research::to_markdown(&empty);
    assert!(md.contains("No grounded answer"));

    // Saving a brief attributes it to the model, not to the author.
    let saved = research::save_to_binder(&f.root, &empty).unwrap();
    let conn = store::open(&f.root).unwrap();
    for b in blocks::list(&conn, &saved.id).unwrap() {
        let who = provenance::classify(&provenance::history(&conn, &b.id).unwrap());
        assert_ne!(
            who.origin,
            provenance::Origin::Human,
            "a generated brief is not the author's writing"
        );
    }
}

/// UF-07 and UF-08: accepting a proposal is recorded; pasting one by hand is
/// not, and the app says so rather than pretending otherwise.
#[test]
fn e2e_uf07_accepting_a_proposal_records_the_model_that_wrote_it() {
    let f = fixture("accept");
    let doc = project::create_document(&f.root, None, "Chapter", DocKind::Scene).unwrap();
    project::write_body(
        &f.root,
        &doc.id,
        "She counted what she had left, and then she counted it again more slowly.",
    )
    .unwrap();

    let proposal = "The salt went on being white in the dark, mile after mile of it, all the way \
                    to the edge of what anyone had bothered to map.";
    let added = project::accept_proposal(
        &f.root,
        &doc.id,
        proposal,
        "llama3.2:3b",
        json!({ "temperature": 0.7 }),
    )
    .unwrap();

    let conn = store::open(&f.root).unwrap();

    // The accepted paragraph: written by the model, taken by the author.
    let chain = provenance::history(&conn, &added.id).unwrap();
    assert_eq!(chain.len(), 2, "{chain:?}");
    assert_eq!(chain[0].action, provenance::Action::Create);
    assert_eq!(chain[0].actor_type, provenance::ActorType::AiModel);
    assert_eq!(chain[0].model.as_deref(), Some("llama3.2:3b"));
    assert!(chain[0].model_params.as_ref().unwrap().contains("0.7"));
    assert_eq!(chain[1].action, provenance::Action::Accept);
    assert_eq!(chain[1].actor_type, provenance::ActorType::Human);

    let who = provenance::classify(&chain);
    assert_eq!(who.origin, provenance::Origin::Ai);
    assert_eq!(who.disclosure_line, "AI-generated text with human review");
    assert!(!who.uncertain && !who.unrecorded);

    // The author's own sentence is untouched and still theirs. Accepting a
    // proposal must not relabel the prose around it.
    let all = blocks::list(&conn, &doc.id).unwrap();
    assert_eq!(all.len(), 2);
    let mine = all.iter().find(|b| b.id != added.id).unwrap();
    assert_eq!(
        provenance::classify(&provenance::history(&conn, &mine.id).unwrap()).origin,
        provenance::Origin::Human
    );

    // A document-wide summary separates them rather than averaging them away.
    let s = provenance::summarise(&conn, Some(&doc.id)).unwrap();
    assert_eq!((s.blocks, s.human, s.ai), (2, 1, 1));
    assert_eq!(s.models, vec!["llama3.2:3b"]);
    drop(conn);

    // UF-08: the same text pasted by hand is indistinguishable from typing, and
    // is recorded as the author's. This is the known hole, and it is a fact the
    // report must disclose rather than a bug to hide.
    let pasted = project::create_document(&f.root, None, "Pasted", DocKind::Scene).unwrap();
    project::write_body(&f.root, &pasted.id, proposal).unwrap();
    let conn = store::open(&f.root).unwrap();
    let block = &blocks::list(&conn, &pasted.id).unwrap()[0];
    assert_eq!(
        provenance::classify(&provenance::history(&conn, &block.id).unwrap()).origin,
        provenance::Origin::Human,
        "manual insertion cannot be observed, so it reads as the author's"
    );

    // Empty proposals and folders are refused rather than half-written.
    assert!(project::accept_proposal(&f.root, &doc.id, "   ", "m", json!({})).is_err());
    let folder = project::create_document(&f.root, None, "A folder", DocKind::Folder).unwrap();
    assert!(project::accept_proposal(&f.root, &folder.id, "text", "m", json!({})).is_err());
}

/// Builds a demo project on disk for screen capture. Not a test of behaviour —
/// it is how the UI gets something real to render. Gated so it never runs in a
/// normal suite.
#[test]
fn seed_demo_project() {
    if std::env::var("AEGIS_SEED").as_deref() != Ok("1") {
        return;
    }
    let parent = std::path::PathBuf::from(std::env::var("AEGIS_SEED_DIR").unwrap());
    std::fs::create_dir_all(&parent).unwrap();
    let root = std::path::PathBuf::from(
        project::create(&parent, "The Salt Road", "nonfiction").unwrap().path,
    );

    // The template seeds an empty opening scene; the app auto-selects the first
    // document with a file, so remove it or the demo opens on a blank page.
    for d in project::documents(&root).unwrap() {
        if d.kind != DocKind::Folder && d.word_count == 0 {
            project::delete_document(&root, &d.id).unwrap();
        }
    }

    // Prose the author wrote.
    let manuscript = project::documents(&root).unwrap().into_iter()
        .find(|d| d.title == "Manuscript").map(|d| d.id);
    let ch = project::create_document(
        &root, manuscript.as_deref(), "1. The Well at Kharsa", DocKind::Scene,
    ).unwrap();
    project::write_body(
        &root,
        &ch.id,
        concat!(
            "The well at Kharsa was the only honest thing in the town and it was going dry.\n\n",
            "She counted what she had left, and then she counted it again more slowly.\n\n",
            "Below them the salt went on being white in the dark, mile after mile of it.",
        ),
    )
    .unwrap();
    project::update_meta(&root, &ch.id, Some("Tamsin buys water."), Some("Plot"), Some("Done")).unwrap();

    // A paragraph a model wrote, and the author took.
    project::accept_proposal(
        &root,
        &ch.id,
        "The pumps had stopped in the night and nobody had thought it worth saying so out loud.",
        "llama3.2:3b",
        json!({ "temperature": 0.7, "mode": "continue" }),
    )
    .unwrap();

    // Research sources, and claims extracted from them.
    let (smith, _) = sources::register_text(
        &root,
        "Groundwater Depletion in Arid Basins",
        "Smith, J. and Okafor, A.\n2025\n\ndoi:10.5555/abcd\n\n         We found that unmetered extraction lowers the water table by a wide margin in every \
         basin we surveyed during the study period.",
        sources::SourceKind::Text,
        None,
        None,
        project::now_ms(),
    )
    .unwrap();
    let (lin, _) = sources::register_text(
        &root,
        "A Replication Study of Basin Extraction",
        "Lin, R.\n2026\n\n         We found that unmetered extraction does not lower the water table in any basin we \
         surveyed, once seasonal rainfall is controlled for.",
        sources::SourceKind::Text,
        None,
        None,
        project::now_ms(),
    )
    .unwrap();

    let conn = store::open(&root).unwrap();
    for (src, reply) in [
        (&smith, r#"{"concepts":[{"name":"Unmetered extraction","description":"drawing groundwater without measurement"}],
           "claims":[{"text":"Unmetered extraction lowers the water table in every basin surveyed","kind":"result",
                      "excerpt":"unmetered extraction lowers the water table by a wide margin","concepts":["Unmetered extraction"]}]}"#),
        (&lin, r#"{"claims":[{"text":"Unmetered extraction does not lower the water table once rainfall is controlled for","kind":"result",
                      "excerpt":"unmetered extraction does not lower the water table in any basin","concepts":[]}]}"#),
    ] {
        let text = sources::read_text(&root, src).unwrap();
        let e = knowledge::validate_extraction(reply, &text, &src.id, "llama3.2:3b", project::now_ms()).unwrap();
        knowledge::save_extraction(&conn, &e).unwrap();
    }

    // A paragraph that cites one of them.
    let block = blocks::list(&conn, &ch.id).unwrap()[0].id.clone();
    let claim = knowledge::claims(&conn, Some(&smith.id)).unwrap()[0].id.clone();
    knowledge::cite(&conn, &block, &claim, project::now_ms()).unwrap();

    // A paragraph with no record at all, as if written before the ledger.
    let legacy = project::create_document(
        &root, manuscript.as_deref(), "2. The Road North", DocKind::Scene,
    ).unwrap();
    let rel = project::documents(&root).unwrap().into_iter()
        .find(|d| d.id == legacy.id).unwrap().path.unwrap();
    std::fs::write(
        root.join(&rel),
        "The road ran north out of the town and then it stopped pretending to be a road.",
    )
    .unwrap();

    println!("SEEDED {}", root.display());
}

// --- Deep Continuity --------------------------------------------------------

/// The canonical continuity error, caught: a fact established in chapter two,
/// contradicted in chapter three, found without a model.
#[test]
fn e2e_continuity_catches_the_contradiction_the_book_established_earlier() {
    let f = fixture("continuity");

    // Chapter two breaks the cartographer's leg, and it goes into canon.
    let docs = project::documents(&f.root).unwrap();
    let canon_folder = docs.iter().find(|d| d.title == "Canon").unwrap();
    let canon = project::create_document(&f.root, Some(&canon_folder.id), "Facts", DocKind::Canon)
        .unwrap();
    project::append_canon_entry(
        &f.root,
        &canon.id,
        "The cartographer's left leg was broken on the descent",
        "2. The Descent",
    )
    .unwrap();
    project::append_canon_entry(
        &f.root,
        &canon.id,
        "Water is rationed at four measures a day",
        "1. The Well",
    )
    .unwrap();

    // A character sheet, so entities are the project's own, not guesses.
    project::create_sheet(&f.root, "The Cartographer", DocKind::Sheet, None, None).unwrap();

    // Chapter three forgets.
    let ch3 = project::create_document(&f.root, None, "3. The Return", DocKind::Scene).unwrap();
    project::write_body(
        &f.root,
        &ch3.id,
        "The cartographer sprinted away on his broken left leg, which the descent had cost him, \
         and did not once look back at the rim of the basin.",
    )
    .unwrap();

    let report = continuity::report(&f.root, &ch3.id).unwrap();

    // The rule it is near, surfaced first — and the unrelated one is not.
    assert_eq!(report.canon.canon_entries, 2);
    assert!(!report.canon.candidates.is_empty());
    assert!(
        report.canon.candidates[0].fact.contains("left leg"),
        "{:?}",
        report.canon.candidates
    );
    assert_eq!(report.canon.candidates[0].source, "2. The Descent");
    assert!(!report.canon.candidates.iter().any(|c| c.fact.contains("rationed")));

    // The character is recognised because the author wrote a sheet for them.
    assert!(
        report.entities.iter().any(|e| e.name == "The Cartographer" && e.kind == "character"),
        "{:?}",
        report.entities
    );

    // And it is retrieval, not a verdict — the report says so on its face.
    assert!(report.canon.method.contains("nothing here is a verdict"));
}

/// A promise the book has not kept, and a gap recorded rather than invented.
#[test]
fn e2e_the_ledger_finds_an_unpaid_plant_and_a_debt_is_written_down() {
    let f = fixture("ledger");
    let one = project::create_document(&f.root, None, "1. The Well", DocKind::Scene).unwrap();
    let two = project::create_document(&f.root, None, "2. The Descent", DocKind::Scene).unwrap();
    project::update_extended(&f.root, &one.id, None, None, None, Some("stone-marker, missing-map"), None)
        .unwrap();
    project::update_extended(&f.root, &two.id, None, None, None, None, Some("stone-marker")).unwrap();

    let ledger = continuity::check_foreshadowing_ledger(&project::documents(&f.root).unwrap());
    assert_eq!(ledger.unpaid.len(), 1);
    assert_eq!(ledger.unpaid[0].id, "missing-map");
    assert_eq!(ledger.unpaid[0].planted_in[0].1, "1. The Well");
    assert_eq!(ledger.complete.len(), 1);

    // Through the gateway, unguarded — it reads and changes nothing.
    let out = agent::dispatch(&f.root, "check_foreshadowing_ledger", &json!({})).unwrap();
    assert_eq!(out.value["ledger"]["unpaid"].as_array().unwrap().len(), 1);

    // A model that finds a hole records it instead of filling it. The debt
    // names the model, because a model's reading and an author's are different
    // kinds of claim.
    agent::dispatch_as(
        &f.root,
        "log_propagation_debt",
        &json!({
            "gap": "The book never establishes where the marker stone came from",
            "found_in": "1. The Well"
        }),
        provenance::Actor::model("llama3.2:3b", json!({})),
    )
    .unwrap();

    let debts = continuity::debts(&f.root);
    assert_eq!(debts.len(), 1);
    assert_eq!(debts[0].found_by, "llama3.2:3b");
    assert!(debts[0].resolved_at.is_none());
    assert!(f.root.join(continuity::DEBT_FILE).exists(), "a file the author can read");

    // The same gap logged by the author is the same debt, not a second one.
    agent::dispatch(
        &f.root,
        "log_propagation_debt",
        &json!({ "gap": "the book never establishes where the marker stone came from" }),
    )
    .unwrap();
    assert_eq!(continuity::debts(&f.root).len(), 1);
}
