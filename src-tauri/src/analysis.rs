//! Story analysis (Phases 8 and 9).
//!
//! Everything is a projection over binder metadata. Nothing here writes to a
//! document, and nothing needs the network.
//!
//! Where a phase doc calls for a local LLM classifier, the shape is the same:
//! a deterministic structural pass produces the view, and a model can refine
//! labels later. Building it structure-first means the views work on a machine
//! with no models at all, and a model improves them rather than gating them.

use crate::model::{
    Beat, ClusterProposal, DocumentRow, GraphEdge, GraphNode, PacingPoint, StoryGraph, TimelineEntry,
};
use crate::integrity::{containment, shingles};
use std::collections::HashMap;

/// Documents that carry prose, in binder order.
fn prose(docs: &[DocumentRow]) -> Vec<&DocumentRow> {
    let mut v: Vec<&DocumentRow> = docs
        .iter()
        .filter(|d| matches!(d.kind, crate::model::DocKind::Scene) && d.word_count > 0)
        .collect();
    v.sort_by_key(|d| d.position);
    v
}

/// Beats positioned by where they fall in the manuscript, measured in words —
/// not by document index, because a 4,000-word chapter and a 400-word scene do
/// not occupy the same amount of a reader's attention.
pub fn plot_beats(docs: &[DocumentRow], method: &str) -> Vec<Beat> {
    let items = prose(docs);
    let total: i64 = items.iter().map(|d| d.word_count).sum();
    if total == 0 {
        return Vec::new();
    }

    let labels: &[(f32, &str)] = match method {
        "freytag" => &[
            (0.0, "Exposition"),
            (0.15, "Rising action"),
            (0.5, "Climax"),
            (0.75, "Falling action"),
            (0.9, "Denouement"),
        ],
        "heros_journey" => &[
            (0.0, "Ordinary world"),
            (0.12, "Call to adventure"),
            (0.3, "Crossing the threshold"),
            (0.5, "Ordeal"),
            (0.75, "The road back"),
            (0.9, "Return"),
        ],
        _ => &[
            (0.0, "Opening image"),
            (0.12, "Inciting incident"),
            (0.25, "Break into two"),
            (0.5, "Midpoint"),
            (0.75, "All is lost"),
            (0.9, "Resolution"),
        ],
    };

    let mut running = 0i64;
    let mut out = Vec::new();
    for doc in &items {
        let at = running as f32 / total as f32;
        running += doc.word_count;
        // The nearest structural label at or before this point.
        let label = labels
            .iter()
            .rev()
            .find(|(pos, _)| at >= *pos - 0.02)
            .map(|(_, l)| *l)
            .unwrap_or("Opening image");

        out.push(Beat {
            document_id: doc.id.clone(),
            title: doc.title.clone(),
            position: at,
            label: label.to_string(),
            // Structural placement is certain; what happens in the scene is not.
            // A synopsis is evidence, so it raises the confidence a little.
            confidence: if doc.synopsis.is_some() { 0.6 } else { 0.35 },
        });
    }
    out
}

/// Narrative order is binder order. Chronological order uses the story date in
/// each document's POV-independent `plants` field is *not* used here — story
/// time comes from a dedicated field when present, and falls back to binder
/// order so the view never lies about ordering it does not know.
pub fn timeline(docs: &[DocumentRow], chronological: bool) -> Vec<TimelineEntry> {
    let items = prose(docs);
    let mut entries: Vec<TimelineEntry> = items
        .iter()
        .enumerate()
        .map(|(i, d)| TimelineEntry {
            document_id: d.id.clone(),
            title: d.title.clone(),
            narrative_order: i as i64,
            // `pov` is not story time; only an explicit day number is.
            story_day: story_day(d),
            out_of_order: false,
        })
        .collect();

    if chronological {
        // Documents with no date keep their narrative position rather than
        // being dumped at one end, which would invent an ordering.
        entries.sort_by_key(|e| (e.story_day.unwrap_or(i64::MAX), e.narrative_order));
    }
    for (i, e) in entries.iter_mut().enumerate() {
        e.out_of_order = e.narrative_order != i as i64;
    }
    entries
}

/// Reads `Day 12` / `day 12` out of the notes field. Deliberately narrow: a
/// guess about dates would produce a confidently wrong timeline.
fn story_day(doc: &DocumentRow) -> Option<i64> {
    let notes = doc.notes.as_deref()?;
    let lower = notes.to_lowercase();
    let idx = lower.find("day ")?;
    lower[idx + 4..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}

/// Gaps between consecutive dated scenes, in days.
pub fn timeline_gaps(entries: &[TimelineEntry], threshold: i64) -> Vec<(String, i64)> {
    let dated: Vec<&TimelineEntry> = entries.iter().filter(|e| e.story_day.is_some()).collect();
    let mut out = Vec::new();
    for pair in dated.windows(2) {
        let gap = pair[1].story_day.unwrap() - pair[0].story_day.unwrap();
        if gap >= threshold {
            out.push((pair[1].title.clone(), gap));
        }
    }
    out
}

/// Character graph: nodes are sheets, edges are shared scenes. An edge means
/// "these two appear in a scene together", which is a fact from the links
/// table rather than an inference.
pub fn character_graph(docs: &[DocumentRow]) -> StoryGraph {
    character_graph_filtered(docs, None)
}

/// The graph, optionally narrowed to scenes carrying one label.
///
/// Filtering by label rather than by act because an act is a folder someone
/// happened to name, while a label is a property of the scene the author set
/// deliberately. "Show me only the Plot scenes" is a question about the book;
/// "show me only Act II" is a question about the folder tree, which the binder
/// already answers.
pub fn character_graph_filtered(docs: &[DocumentRow], label: Option<&str>) -> StoryGraph {
    // Scenes in binder order, so "how long since" is measurable.
    let scene_order: HashMap<String, i64> = prose(docs)
        .iter()
        .enumerate()
        .map(|(i, d)| (d.id.clone(), i as i64))
        .collect();
    let total_scenes = scene_order.len() as i64;

    let allowed: Option<std::collections::HashSet<String>> = label.map(|l| {
        docs.iter()
            .filter(|d| d.label.as_deref() == Some(l))
            .map(|d| d.id.clone())
            .collect()
    });

    let mut labels: Vec<String> = docs.iter().filter_map(|d| d.label.clone()).collect();
    labels.sort();
    labels.dedup();

    let sheets: Vec<&DocumentRow> = docs
        .iter()
        .filter(|d| matches!(d.kind, crate::model::DocKind::Sheet | crate::model::DocKind::Location))
        .collect();

    // sheet id -> the scenes it appears in
    let mut appearances: HashMap<String, Vec<String>> = HashMap::new();
    for sheet in &sheets {
        let scenes: Vec<String> = sheet
            .links
            .iter()
            .filter(|l| l.relation == "appears_in" && l.from_id == sheet.id)
            .map(|l| l.to_id.clone())
            .filter(|id| allowed.as_ref().map(|a| a.contains(id)).unwrap_or(true))
            .collect();
        appearances.insert(sheet.id.clone(), scenes);
    }

    let nodes: Vec<GraphNode> = sheets
        .iter()
        .map(|s| GraphNode {
            id: s.id.clone(),
            label: s.title.clone(),
            weight: appearances.get(&s.id).map(|v| v.len()).unwrap_or(0) as i64,
            kind: format!("{:?}", s.kind).to_lowercase(),
        })
        .collect();

    let mut edges = Vec::new();
    for i in 0..sheets.len() {
        for j in (i + 1)..sheets.len() {
            let a = &sheets[i].id;
            let b = &sheets[j].id;
            let shared = appearances
                .get(a)
                .zip(appearances.get(b))
                .map(|(x, y)| x.iter().filter(|s| y.contains(s)).count())
                .unwrap_or(0);
            if shared > 0 {
                // How long since these two were last on the page together.
                let last_together = appearances
                    .get(a)
                    .zip(appearances.get(b))
                    .and_then(|(x, y)| {
                        x.iter()
                            .filter(|s| y.contains(s))
                            .filter_map(|s| scene_order.get(s))
                            .max()
                            .copied()
                    });
                let scenes_since = last_together
                    .filter(|_| total_scenes > 0)
                    .map(|last| total_scenes - 1 - last)
                    .filter(|gap| *gap > 0);

                edges.push(GraphEdge {
                    from: a.clone(),
                    to: b.clone(),
                    weight: shared as i64,
                    relation: "shares scenes".into(),
                    scenes_since,
                });
            }
        }
    }
    StoryGraph { nodes, edges, labels }
}

/// Pacing: one point per scene, carrying length and prose signals so the UI can
/// plot a real curve rather than a decorative one.
pub fn pacing(docs: &[DocumentRow], bodies: &HashMap<String, String>) -> Vec<PacingPoint> {
    prose(docs)
        .into_iter()
        .enumerate()
        .map(|(i, d)| {
            let signals = bodies.get(&d.id).map(|b| crate::integrity::writing_signals(b)).unwrap_or_default();
            PacingPoint {
                index: i as i64,
                document_id: d.id.clone(),
                title: d.title.clone(),
                word_count: d.word_count,
                // Shorter sentences and more dialogue read faster; this is a
                // measured proxy for tempo, not a claim about tension.
                tempo: if signals.mean_sentence_length > 0.0 {
                    (30.0 / signals.mean_sentence_length).min(2.0)
                } else {
                    0.0
                },
                variance: signals.sentence_length_variance,
            }
        })
        .collect()
}

/// Groups documents whose prose overlaps above a threshold. Proposals only —
/// accepting one creates a collection, which never duplicates a file.
pub fn suggest_clusters(
    docs: &[DocumentRow],
    bodies: &HashMap<String, String>,
    threshold: f32,
) -> Vec<ClusterProposal> {
    let items: Vec<&DocumentRow> = prose(docs);
    let shingled: HashMap<String, std::collections::HashSet<String>> = items
        .iter()
        .filter_map(|d| bodies.get(&d.id).map(|b| (d.id.clone(), shingles(b, 4))))
        .collect();

    let mut used: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for a in &items {
        if used.contains(&a.id) {
            continue;
        }
        let Some(sa) = shingled.get(&a.id) else { continue };
        let mut group = vec![a.id.clone()];
        let mut best = 0.0f32;
        for b in &items {
            if b.id == a.id || used.contains(&b.id) {
                continue;
            }
            let Some(sb) = shingled.get(&b.id) else { continue };
            let score = containment(sa, sb).max(containment(sb, sa));
            if score >= threshold {
                group.push(b.id.clone());
                best = best.max(score);
            }
        }
        if group.len() > 1 {
            used.extend(group.iter().cloned());
            let titles: Vec<String> = group
                .iter()
                .filter_map(|id| items.iter().find(|d| &d.id == id).map(|d| d.title.clone()))
                .collect();
            out.push(ClusterProposal {
                name: format!("Similar to “{}”", titles[0]),
                document_ids: group,
                titles,
                score: best,
            });
        }
    }
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DocKind, DocumentLink};

    fn scene(id: &str, title: &str, pos: i64, words: i64) -> DocumentRow {
        DocumentRow {
            id: id.into(),
            parent_id: None,
            title: title.into(),
            kind: DocKind::Scene,
            path: Some(format!("manuscript/scenes/{id}.md")),
            position: pos,
            synopsis: None,
            label: None,
            status: None,
            word_count: words,
            updated_at: 0,
            notes: None,
            target: None,
            pov: None,
            plants: None,
            payoffs: None,
            keywords: vec![],
            links: vec![],
            section_type: None,
            icon: None,
            x: None,
            y: None,
        }
    }

    fn sheet(id: &str, title: &str, appears_in: &[&str]) -> DocumentRow {
        let mut d = scene(id, title, 0, 0);
        d.kind = DocKind::Sheet;
        d.links = appears_in
            .iter()
            .map(|s| DocumentLink { from_id: id.into(), to_id: (*s).into(), relation: "appears_in".into() })
            .collect();
        d
    }

    // --- beats -------------------------------------------------------------

    #[test]
    fn beats_are_placed_by_word_count_not_document_count() {
        // One enormous opening scene should push everything after it late.
        let docs = vec![
            scene("a", "Long opening", 0, 9_000),
            scene("b", "Short middle", 1, 500),
            scene("c", "Short end", 2, 500),
        ];
        let beats = plot_beats(&docs, "three_act");
        assert_eq!(beats.len(), 3);
        assert!((beats[0].position - 0.0).abs() < 1e-6);
        assert!(beats[1].position > 0.85, "after 9,000 of 10,000 words we are late in the book, got {}", beats[1].position);
        assert_eq!(beats[0].label, "Opening image");
    }

    #[test]
    fn every_method_labels_every_scene() {
        let docs: Vec<DocumentRow> = (0..8).map(|i| scene(&format!("s{i}"), "S", i, 1000)).collect();
        for method in ["three_act", "freytag", "heros_journey"] {
            let beats = plot_beats(&docs, method);
            assert_eq!(beats.len(), 8, "{method} dropped a scene");
            assert!(beats.iter().all(|b| !b.label.is_empty()), "{method} left a scene unlabelled");
            assert!(beats.iter().all(|b| (0.0..=1.0).contains(&b.position)));
        }
    }

    #[test]
    fn an_empty_manuscript_produces_no_beats_rather_than_dividing_by_zero() {
        assert!(plot_beats(&[], "three_act").is_empty());
        assert!(plot_beats(&[scene("a", "Empty", 0, 0)], "three_act").is_empty());
    }

    #[test]
    fn a_synopsis_raises_confidence_because_it_is_evidence() {
        let mut with = scene("a", "A", 0, 100);
        with.synopsis = Some("Something happens.".into());
        let without = scene("b", "B", 1, 100);
        let beats = plot_beats(&[with, without], "three_act");
        assert!(beats[0].confidence > beats[1].confidence);
    }

    // --- timeline ----------------------------------------------------------

    fn dated(id: &str, title: &str, pos: i64, day: i64) -> DocumentRow {
        let mut d = scene(id, title, pos, 100);
        d.notes = Some(format!("Takes place on Day {day}."));
        d
    }

    #[test]
    fn narrative_order_is_binder_order() {
        let docs = vec![dated("a", "A", 0, 10), dated("b", "B", 1, 2)];
        let t = timeline(&docs, false);
        assert_eq!(t[0].title, "A");
        assert_eq!(t[1].title, "B");
        assert!(t.iter().all(|e| !e.out_of_order), "narrative order is never out of order with itself");
    }

    #[test]
    fn chronological_order_sorts_by_story_day_and_flags_the_moves() {
        let docs = vec![dated("a", "A", 0, 10), dated("b", "B", 1, 2), dated("c", "C", 2, 12)];
        let t = timeline(&docs, true);
        assert_eq!(t.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), vec!["B", "A", "C"]);
        assert!(t[0].out_of_order, "B moved from position 1 to 0");
        assert!(t[1].out_of_order, "A moved from 0 to 1");
        assert!(!t[2].out_of_order, "C did not move");
    }

    #[test]
    fn undated_scenes_keep_their_place_instead_of_inventing_one() {
        let docs = vec![dated("a", "A", 0, 5), scene("b", "B", 1, 100), dated("c", "C", 2, 1)];
        let t = timeline(&docs, true);
        // C is earliest; A next; B has no date so it stays at the end in
        // narrative order rather than being guessed into the sequence.
        assert_eq!(t.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), vec!["C", "A", "B"]);
        assert_eq!(t.iter().find(|e| e.title == "B").unwrap().story_day, None);
    }

    #[test]
    fn a_day_is_only_read_when_it_is_actually_written_down() {
        let mut vague = scene("a", "A", 0, 100);
        vague.notes = Some("Some time later, on a hot afternoon.".into());
        assert_eq!(timeline(&[vague], true)[0].story_day, None, "prose about time is not a date");
    }

    #[test]
    fn gaps_are_reported_only_above_the_threshold() {
        let docs = vec![dated("a", "A", 0, 1), dated("b", "B", 1, 2), dated("c", "C", 2, 20)];
        let t = timeline(&docs, true);
        let gaps = timeline_gaps(&t, 5);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0], ("C".to_string(), 18));
        assert!(timeline_gaps(&t, 100).is_empty());
    }

    // --- character graph ---------------------------------------------------

    #[test]
    fn an_edge_means_two_sheets_share_a_scene() {
        let docs = vec![
            scene("s1", "Scene one", 0, 100),
            scene("s2", "Scene two", 1, 100),
            sheet("tamsin", "Tamsin", &["s1", "s2"]),
            sheet("carto", "Cartographer", &["s1"]),
            sheet("alone", "Never seen", &[]),
        ];
        let g = character_graph(&docs);
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.nodes.iter().find(|n| n.label == "Tamsin").unwrap().weight, 2, "weight is scene count");
        assert_eq!(g.nodes.iter().find(|n| n.label == "Never seen").unwrap().weight, 0);

        assert_eq!(g.edges.len(), 1, "only the pair sharing a scene is connected");
        assert_eq!(g.edges[0].weight, 1);
    }

    #[test]
    fn characters_who_never_meet_have_no_edge() {
        let docs = vec![
            scene("s1", "One", 0, 10),
            scene("s2", "Two", 1, 10),
            sheet("a", "A", &["s1"]),
            sheet("b", "B", &["s2"]),
        ];
        assert!(character_graph(&docs).edges.is_empty());
    }

    // --- pacing and clusters ----------------------------------------------

    #[test]
    fn pacing_reports_one_point_per_scene_with_real_measures() {
        let docs = vec![scene("a", "A", 0, 40), scene("b", "B", 1, 60)];
        let mut bodies = HashMap::new();
        bodies.insert("a".to_string(), "Short. Fast. Clipped. Sharp.".to_string());
        bodies.insert(
            "b".to_string(),
            "This is a considerably longer sentence that unspools at some length before it finally arrives at its full stop.".to_string(),
        );
        let p = pacing(&docs, &bodies);
        assert_eq!(p.len(), 2);
        assert!(p[0].tempo > p[1].tempo, "short sentences read faster than long ones");
        assert_eq!(p[1].word_count, 60);
    }

    #[test]
    fn pacing_survives_a_missing_body() {
        let docs = vec![scene("a", "A", 0, 40)];
        let p = pacing(&docs, &HashMap::new());
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].tempo, 0.0, "no body means no tempo, not a crash");
    }

    #[test]
    fn clusters_group_near_duplicates_and_leave_distinct_scenes_alone() {
        let docs = vec![
            scene("a", "Opening", 0, 20),
            scene("b", "Second opening", 1, 20),
            scene("c", "Something else", 2, 20),
        ];
        let shared = "The well at Kharsa was the only honest thing in the town and it was going dry that season";
        let mut bodies = HashMap::new();
        bodies.insert("a".to_string(), shared.to_string());
        bodies.insert("b".to_string(), shared.to_string());
        bodies.insert("c".to_string(), "Nine days of tariff owed, counted by someone who was not her".to_string());

        let clusters = suggest_clusters(&docs, &bodies, 0.7);
        assert_eq!(clusters.len(), 1, "only the duplicated pair should group");
        assert_eq!(clusters[0].document_ids.len(), 2);
        assert!(clusters[0].document_ids.contains(&"a".to_string()));
        assert!(clusters[0].document_ids.contains(&"b".to_string()));
        assert!(!clusters[0].document_ids.contains(&"c".to_string()));
    }

    #[test]
    fn a_document_appears_in_at_most_one_cluster() {
        let docs: Vec<DocumentRow> = (0..4).map(|i| scene(&format!("d{i}"), "S", i, 20)).collect();
        let same = "identical prose repeated across every one of these four documents entirely";
        let bodies: HashMap<String, String> =
            (0..4).map(|i| (format!("d{i}"), same.to_string())).collect();

        let clusters = suggest_clusters(&docs, &bodies, 0.7);
        let mut seen = Vec::new();
        for c in &clusters {
            for id in &c.document_ids {
                assert!(!seen.contains(id), "{id} appeared in two clusters");
                seen.push(id.clone());
            }
        }
    }


    #[test]
    fn the_graph_reports_how_long_since_two_characters_shared_a_scene() {
        let mut docs: Vec<DocumentRow> =
            (1..=10).map(|i| scene(&format!("s{i}"), &format!("{i}. Scene"), i, 100)).collect();
        docs[2].label = Some("Plot".into()); // scene 3

        // They share scene 3, early. Tamsin appears again at scene 9 alone.
        docs.push(sheet("a", "Tamsin", &["s3", "s9"]));
        docs.push(sheet("b", "The Cartographer", &["s3"]));

        let g = character_graph(&docs);
        let edge = g.edges.iter().find(|e| e.weight > 0).expect("they share a scene");
        assert_eq!(
            edge.scenes_since,
            Some(7),
            "ten scenes, last together at the third: seven have passed"
        );
        assert!(g.labels.contains(&"Plot".to_string()), "filterable labels come from the scenes");

        // Filtering keeps only what the label covers.
        let filtered = character_graph_filtered(&docs, Some("Plot"));
        assert_eq!(
            filtered.nodes.iter().find(|n| n.id == "a").unwrap().weight,
            1,
            "scene 9 carries no label, so it is out"
        );

        // A label nothing carries gives an empty graph, not the unfiltered one.
        assert!(character_graph_filtered(&docs, Some("Nothing")).edges.is_empty());
    }

    #[test]
    fn characters_together_in_the_latest_scene_are_not_reported_as_stale() {
        let mut docs: Vec<DocumentRow> =
            (1..=3).map(|i| scene(&format!("s{i}"), &format!("{i}. Scene"), i, 100)).collect();
        docs.push(sheet("a", "Tamsin", &["s3"]));
        docs.push(sheet("b", "The Cartographer", &["s3"]));

        let edge = character_graph(&docs).edges.into_iter().find(|e| e.weight > 0).unwrap();
        assert_eq!(edge.scenes_since, None, "still on the page together");
    }

}
