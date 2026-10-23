//! The infinite canvas, and committing it back to the binder (Phase 8).
//!
//! Every other view in this app is a projection: the binder is the truth and
//! the corkboard, outliner and timeline are ways of looking at it. The canvas is
//! the one place that runs the other way — the author arranges cards in space
//! and then asks for that arrangement to *become* the binder order.
//!
//! That inversion is the whole risk, so it is done in two explicit halves:
//!
//! - `save` / `load` keep positions in `.aegis/viz/canvas.json`. Moving a card
//!   changes nothing but that file. The binder does not notice.
//! - `plan` reads the positions and works out what the binder order *would*
//!   become, without touching anything. `commit` applies a plan it is handed.
//!
//! So the author always sees the reorder before it happens, and a card nudged
//! by accident cannot silently rewrite the reading order of a book.
//!
//! # Reading order on a plane
//!
//! Cards laid out in space have no inherent sequence, so one has to be chosen,
//! and it has to be the one people expect. This uses **rows, then columns**:
//! cards within `ROW_TOLERANCE` of each other vertically are one row, read
//! left to right; rows run top to bottom. That is how a person reads a wall of
//! index cards, and it means a deliberate two-column layout does not get
//! interleaved into nonsense.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{err, AppResult};
use crate::model::DocumentRow;
use crate::project;

/// Cards whose vertical centres are within this many pixels are the same row.
/// Roughly one card height: closer and a slightly-nudged card starts a new row,
/// further and two genuine rows merge.
const ROW_TOLERANCE: f32 = 90.0;

pub const DIR: &str = ".aegis/viz";
pub const FILE: &str = "canvas.json";

fn canvas_path(root: &Path) -> PathBuf {
    root.join(DIR).join(FILE)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasCard {
    pub id: String,
    pub x: f32,
    pub y: f32,
    /// Optional free-text grouping the author typed on the card.
    #[serde(default)]
    pub group: Option<String>,
}

/// A note the author put on the board. Not a document, and never one: a note is
/// thinking beside the work, and promoting it to a binder entry would make every
/// stray thought something to file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasNote {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub text: String,
    /// A colour name from the app's palette, chosen by the author.
    #[serde(default)]
    pub tone: Option<String>,
}

/// An arrow between two things on the board. Either end may be a card or a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasArrow {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Canvas {
    #[serde(default)]
    pub cards: Vec<CanvasCard>,
    /// Notes and arrows are **not** part of the reading order. A note between
    /// two scenes must not silently become a scene when the layout is
    /// committed, so `reading_order` never sees them.
    #[serde(default)]
    pub notes: Vec<CanvasNote>,
    #[serde(default)]
    pub arrows: Vec<CanvasArrow>,
    #[serde(default)]
    pub updated_at: i64,
}

pub fn load(root: &Path) -> Canvas {
    std::fs::read_to_string(canvas_path(root))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(root: &Path, canvas: &Canvas) -> AppResult<()> {
    let path = canvas_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = canvas.clone();
    out.updated_at = project::now_ms();
    std::fs::write(&path, serde_json::to_string_pretty(&out)?)?;
    Ok(())
}

/// Place one card, keeping everything else as it was.
pub fn place(root: &Path, id: &str, x: f32, y: f32) -> AppResult<Canvas> {
    let mut canvas = load(root);
    match canvas.cards.iter_mut().find(|c| c.id == id) {
        Some(card) => {
            card.x = x;
            card.y = y;
        }
        None => canvas.cards.push(CanvasCard { id: id.to_string(), x, y, group: None }),
    }
    save(root, &canvas)?;
    Ok(canvas)
}

/// Add or move a note.
pub fn place_note(
    root: &Path,
    id: Option<&str>,
    x: f32,
    y: f32,
    text: &str,
    tone: Option<&str>,
) -> AppResult<CanvasNote> {
    if text.trim().is_empty() {
        return err("a note with nothing written on it is not a note");
    }
    let mut canvas = load(root);
    let note = match id.and_then(|i| canvas.notes.iter_mut().find(|n| n.id == i)) {
        Some(existing) => {
            existing.x = x;
            existing.y = y;
            existing.text = text.trim().to_string();
            existing.tone = tone.map(str::to_string);
            existing.clone()
        }
        None => {
            let note = CanvasNote {
                id: format!("note_{}", crate::project::new_id()),
                x,
                y,
                text: text.trim().to_string(),
                tone: tone.map(str::to_string),
            };
            canvas.notes.push(note.clone());
            note
        }
    };
    save(root, &canvas)?;
    Ok(note)
}

pub fn remove_note(root: &Path, id: &str) -> AppResult<()> {
    let mut canvas = load(root);
    canvas.notes.retain(|n| n.id != id);
    // An arrow to a note that no longer exists would draw into nothing.
    canvas.arrows.retain(|a| a.from != id && a.to != id);
    save(root, &canvas)
}

/// Draw an arrow. Either end may be a card or a note; both must exist.
pub fn draw_arrow(root: &Path, from: &str, to: &str, label: Option<&str>) -> AppResult<CanvasArrow> {
    if from == to {
        return err("an arrow from something to itself says nothing");
    }
    let mut canvas = load(root);
    let exists = |id: &str| {
        canvas.cards.iter().any(|c| c.id == id) || canvas.notes.iter().any(|n| n.id == id)
    };
    if !exists(from) || !exists(to) {
        return err("an arrow needs both ends on the board");
    }
    if canvas.arrows.iter().any(|a| a.from == from && a.to == to) {
        return err("those two are already connected");
    }
    let arrow = CanvasArrow {
        id: format!("arrow_{}", crate::project::new_id()),
        from: from.to_string(),
        to: to.to_string(),
        label: label.map(str::to_string).filter(|l| !l.trim().is_empty()),
    };
    canvas.arrows.push(arrow.clone());
    save(root, &canvas)?;
    Ok(arrow)
}

pub fn remove_arrow(root: &Path, id: &str) -> AppResult<()> {
    let mut canvas = load(root);
    canvas.arrows.retain(|a| a.id != id);
    save(root, &canvas)
}

/// Forget every placement. The canvas falls back to binder order.
pub fn clear(root: &Path) -> AppResult<()> {
    let path = canvas_path(root);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reading the layout
// ---------------------------------------------------------------------------

/// Sort placed cards into reading order: rows top to bottom, and within a row
/// left to right.
pub fn reading_order(cards: &[CanvasCard]) -> Vec<String> {
    let mut sorted: Vec<&CanvasCard> = cards.iter().collect();
    // Sort by y first so rows can be found by scanning.
    sorted.sort_by(|a, b| a.y.total_cmp(&b.y));

    let mut rows: Vec<Vec<&CanvasCard>> = Vec::new();
    for card in sorted {
        // A row is anchored on its first card, not on a running average: an
        // average drifts as cards are added, so a long row can slowly swallow
        // the row below it.
        match rows.last_mut() {
            Some(row) if (card.y - row[0].y).abs() <= ROW_TOLERANCE => row.push(card),
            _ => rows.push(vec![card]),
        }
    }

    let mut out = Vec::new();
    for mut row in rows {
        row.sort_by(|a, b| a.x.total_cmp(&b.x));
        out.extend(row.into_iter().map(|c| c.id.clone()));
    }
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveStep {
    pub id: String,
    pub title: String,
    pub from: i64,
    pub to: i64,
}

/// What committing the canvas would do to the binder. Changes nothing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitPlan {
    /// The folder whose children would be reordered.
    pub parent_id: Option<String>,
    pub parent_title: String,
    /// Only the cards that would actually move.
    pub moves: Vec<MoveStep>,
    /// Placed cards that are no longer in that folder, named so the author is
    /// told rather than left wondering why a card had no effect.
    pub orphans: Vec<String>,
    /// Documents in the folder that were never placed. They keep their relative
    /// order and follow the placed ones.
    pub unplaced: Vec<String>,
}

/// Work out the reorder without applying it.
pub fn plan(root: &Path, parent_id: Option<&str>) -> AppResult<CommitPlan> {
    let canvas = load(root);
    if canvas.cards.is_empty() {
        return err("no cards have been placed on the canvas yet");
    }
    let docs = project::documents(root)?;
    let by_id: HashMap<&str, &DocumentRow> = docs.iter().map(|d| (d.id.as_str(), d)).collect();

    let mut siblings: Vec<&DocumentRow> = docs
        .iter()
        .filter(|d| d.parent_id.as_deref() == parent_id)
        .collect();
    siblings.sort_by_key(|d| d.position);

    let placed_order = reading_order(&canvas.cards);
    let sibling_ids: Vec<&str> = siblings.iter().map(|d| d.id.as_str()).collect();

    let mut orphans = Vec::new();
    let mut ordered: Vec<&str> = Vec::new();
    for id in &placed_order {
        if sibling_ids.contains(&id.as_str()) {
            ordered.push(sibling_ids[sibling_ids.iter().position(|s| s == id).unwrap()]);
        } else {
            orphans.push(
                by_id
                    .get(id.as_str())
                    .map(|d| d.title.clone())
                    .unwrap_or_else(|| format!("a document that no longer exists ({id})")),
            );
        }
    }

    // Anything in the folder that was never placed keeps its order and follows.
    let mut unplaced = Vec::new();
    for doc in &siblings {
        if !ordered.contains(&doc.id.as_str()) {
            ordered.push(doc.id.as_str());
            unplaced.push(doc.title.clone());
        }
    }

    let mut moves = Vec::new();
    for (to, id) in ordered.iter().enumerate() {
        let doc = by_id[id];
        let from = siblings.iter().position(|d| d.id == doc.id).unwrap() as i64;
        if from != to as i64 {
            moves.push(MoveStep {
                id: doc.id.clone(),
                title: doc.title.clone(),
                from,
                to: to as i64,
            });
        }
    }

    Ok(CommitPlan {
        parent_id: parent_id.map(str::to_string),
        parent_title: parent_id
            .and_then(|p| by_id.get(p).map(|d| d.title.clone()))
            .unwrap_or_else(|| "the top level".to_string()),
        moves,
        orphans,
        unplaced,
    })
}

/// Apply a plan. Snapshots first, because a reorder is the kind of change an
/// author wants to be able to walk back from.
pub fn commit(root: &Path, parent_id: Option<&str>) -> AppResult<CommitPlan> {
    let plan = plan(root, parent_id)?;
    if plan.moves.is_empty() {
        return Ok(plan);
    }
    let _ = crate::snapshots::create(root, "Before committing the canvas layout");

    // Applied in target order: each move shifts the ones after it, so working
    // front to back means every later index is still correct when it is used.
    let mut sorted = plan.moves.clone();
    sorted.sort_by_key(|m| m.to);
    for step in &sorted {
        project::move_document(root, &step.id, parent_id, step.to)?;
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DocKind;

    fn card(id: &str, x: f32, y: f32) -> CanvasCard {
        CanvasCard { id: id.into(), x, y, group: None }
    }

    #[test]
    fn reading_order_is_rows_then_columns_the_way_a_person_reads_a_wall() {
        // Two rows of three, deliberately out of order in the list.
        let cards = vec![
            card("c", 500.0, 10.0),
            card("f", 500.0, 300.0),
            card("a", 10.0, 20.0),
            card("e", 260.0, 305.0),
            card("b", 260.0, 15.0),
            card("d", 10.0, 310.0),
        ];
        assert_eq!(reading_order(&cards), vec!["a", "b", "c", "d", "e", "f"]);
    }

    #[test]
    fn a_slightly_uneven_row_is_still_one_row() {
        // Hand-placed cards are never perfectly aligned.
        let cards = vec![card("a", 10.0, 0.0), card("b", 200.0, 40.0), card("c", 400.0, 12.0)];
        assert_eq!(reading_order(&cards), vec!["a", "b", "c"]);
    }

    #[test]
    fn two_deliberate_columns_are_not_interleaved() {
        // A column on the left and a column on the right, well separated
        // vertically: reading rows-first must not zip them together.
        let cards = vec![
            card("left-1", 0.0, 0.0),
            card("left-2", 0.0, 200.0),
            card("right-1", 600.0, 0.0),
            card("right-2", 600.0, 200.0),
        ];
        assert_eq!(reading_order(&cards), vec!["left-1", "right-1", "left-2", "right-2"]);
    }

    fn scratch(tag: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "aegis-canvas-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = project::create(&base, "Canvas", "novel").unwrap();
        (base, PathBuf::from(summary.path))
    }

    #[test]
    fn placements_round_trip_through_the_sidecar_and_clear_away() {
        let (base, root) = scratch("io");
        assert!(load(&root).cards.is_empty());

        place(&root, "one", 10.0, 20.0).unwrap();
        place(&root, "two", 200.0, 20.0).unwrap();
        place(&root, "one", 15.0, 25.0).unwrap();

        let canvas = load(&root);
        assert_eq!(canvas.cards.len(), 2, "moving a card must not add a second one");
        let one = canvas.cards.iter().find(|c| c.id == "one").unwrap();
        assert_eq!((one.x, one.y), (15.0, 25.0));
        assert!(canvas.updated_at > 0);

        // It is a real file inside the project, so a snapshot carries it.
        assert!(root.join(DIR).join(FILE).exists());

        clear(&root).unwrap();
        assert!(load(&root).cards.is_empty());
        clear(&root).unwrap(); // clearing twice is not an error

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_plan_says_what_would_change_and_changes_nothing() {
        let (base, root) = scratch("plan");
        let folder = project::create_document(&root, None, "Act I", DocKind::Folder).unwrap();
        let mut ids = Vec::new();
        for title in ["One", "Two", "Three"] {
            ids.push(
                project::create_document(&root, Some(&folder.id), title, DocKind::Scene)
                    .unwrap()
                    .id,
            );
        }

        // Lay them out reversed.
        place(&root, &ids[2], 0.0, 0.0).unwrap();
        place(&root, &ids[1], 200.0, 0.0).unwrap();
        place(&root, &ids[0], 400.0, 0.0).unwrap();

        let plan = plan(&root, Some(&folder.id)).unwrap();
        assert_eq!(plan.parent_title, "Act I");
        assert_eq!(plan.moves.len(), 2, "the middle card does not move: {:?}", plan.moves);
        assert!(plan.orphans.is_empty());
        assert!(plan.unplaced.is_empty());

        // Nothing moved yet.
        let before: Vec<String> = project::documents(&root)
            .unwrap()
            .into_iter()
            .filter(|d| d.parent_id.as_deref() == Some(folder.id.as_str()))
            .map(|d| d.title)
            .collect();
        assert_eq!(before, vec!["One", "Two", "Three"]);

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn committing_reorders_the_binder_to_match_the_layout() {
        let (base, root) = scratch("commit");
        let folder = project::create_document(&root, None, "Act I", DocKind::Folder).unwrap();
        let mut ids = Vec::new();
        for title in ["One", "Two", "Three", "Four"] {
            ids.push(
                project::create_document(&root, Some(&folder.id), title, DocKind::Scene)
                    .unwrap()
                    .id,
            );
        }

        // Reverse them on the canvas.
        for (i, id) in ids.iter().rev().enumerate() {
            place(&root, id, (i as f32) * 200.0, 0.0).unwrap();
        }

        commit(&root, Some(&folder.id)).unwrap();
        let after: Vec<String> = project::documents(&root)
            .unwrap()
            .into_iter()
            .filter(|d| d.parent_id.as_deref() == Some(folder.id.as_str()))
            .map(|d| d.title)
            .collect();
        assert_eq!(after, vec!["Four", "Three", "Two", "One"]);

        // A second commit of the same layout is a no-op, not a reshuffle.
        let again = commit(&root, Some(&folder.id)).unwrap();
        assert!(again.moves.is_empty(), "{:?}", again.moves);

        // The reorder is walk-back-able.
        let history = crate::snapshots::list(&root, None, 10).unwrap();
        assert!(
            history.iter().any(|s| s.message.contains("Before committing the canvas")),
            "a reorder must be snapshotted first: {:?}",
            history.iter().map(|s| &s.message).collect::<Vec<_>>()
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn an_unplaced_document_keeps_its_place_rather_than_being_lost() {
        let (base, root) = scratch("unplaced");
        let folder = project::create_document(&root, None, "Act I", DocKind::Folder).unwrap();
        let mut ids = Vec::new();
        for title in ["One", "Two", "Three"] {
            ids.push(
                project::create_document(&root, Some(&folder.id), title, DocKind::Scene)
                    .unwrap()
                    .id,
            );
        }
        // Only two of the three are placed, reversed.
        place(&root, &ids[1], 0.0, 0.0).unwrap();
        place(&root, &ids[0], 200.0, 0.0).unwrap();

        let plan = plan(&root, Some(&folder.id)).unwrap();
        assert_eq!(plan.unplaced, vec!["Three"], "the author must be told");

        commit(&root, Some(&folder.id)).unwrap();
        let after: Vec<String> = project::documents(&root)
            .unwrap()
            .into_iter()
            .filter(|d| d.parent_id.as_deref() == Some(folder.id.as_str()))
            .map(|d| d.title)
            .collect();
        assert_eq!(after, vec!["Two", "One", "Three"], "an unplaced scene follows, it is not dropped");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_card_for_a_document_in_another_folder_is_reported_not_dragged_in() {
        let (base, root) = scratch("orphan");
        let a = project::create_document(&root, None, "Act I", DocKind::Folder).unwrap();
        let b = project::create_document(&root, None, "Act II", DocKind::Folder).unwrap();
        let inside = project::create_document(&root, Some(&a.id), "Inside", DocKind::Scene).unwrap();
        let elsewhere =
            project::create_document(&root, Some(&b.id), "Elsewhere", DocKind::Scene).unwrap();

        place(&root, &elsewhere.id, 0.0, 0.0).unwrap();
        place(&root, &inside.id, 200.0, 0.0).unwrap();

        let plan = plan(&root, Some(&a.id)).unwrap();
        assert_eq!(plan.orphans, vec!["Elsewhere"]);

        commit(&root, Some(&a.id)).unwrap();
        let moved = project::documents(&root)
            .unwrap()
            .into_iter()
            .find(|d| d.id == elsewhere.id)
            .unwrap();
        assert_eq!(
            moved.parent_id.as_deref(),
            Some(b.id.as_str()),
            "committing a layout must not move a document between folders"
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn an_empty_canvas_refuses_rather_than_flattening_the_binder() {
        let (base, root) = scratch("empty");
        let message = plan(&root, None).unwrap_err().to_string();
        assert!(message.contains("no cards"), "{message}");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn notes_are_thinking_beside_the_work_and_never_become_documents() {
        let (base, root) = scratch("notes");
        let folder = project::create_document(&root, None, "Act I", DocKind::Folder).unwrap();
        let mut ids = Vec::new();
        for title in ["One", "Two"] {
            ids.push(
                project::create_document(&root, Some(&folder.id), title, DocKind::Scene).unwrap().id,
            );
        }
        // Reversed on the board, with a note sitting between them.
        place(&root, &ids[1], 0.0, 0.0).unwrap();
        place(&root, &ids[0], 400.0, 0.0).unwrap();
        let note = place_note(&root, None, 200.0, 0.0, "does this beat land?", Some("amber")).unwrap();

        let canvas = load(&root);
        assert_eq!(canvas.notes.len(), 1);
        assert_eq!(canvas.notes[0].text, "does this beat land?");
        assert_eq!(canvas.notes[0].tone.as_deref(), Some("amber"));

        // The note sits between the two cards in reading order, and must not
        // become a third document when the layout is committed.
        let plan = plan(&root, Some(&folder.id)).unwrap();
        assert!(
            plan.orphans.is_empty(),
            "a note is not an orphaned card, it is not a card at all: {:?}",
            plan.orphans
        );
        commit(&root, Some(&folder.id)).unwrap();
        let after: Vec<String> = project::documents(&root)
            .unwrap()
            .into_iter()
            .filter(|d| d.parent_id.as_deref() == Some(folder.id.as_str()))
            .map(|d| d.title)
            .collect();
        assert_eq!(after, vec!["Two", "One"], "the note changed nothing about the order");
        assert_eq!(after.len(), 2, "and created nothing");

        // Editing a note in place keeps its id.
        let same = place_note(&root, Some(&note.id), 250.0, 40.0, "answered: yes", None).unwrap();
        assert_eq!(same.id, note.id);
        assert_eq!(load(&root).notes.len(), 1);
        assert_eq!(load(&root).notes[0].text, "answered: yes");

        // A note with nothing on it is refused.
        assert!(place_note(&root, None, 0.0, 0.0, "   ", None).is_err());

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn arrows_connect_things_that_are_on_the_board_and_nothing_else() {
        let (base, root) = scratch("arrows");
        let scene = project::create_document(&root, None, "One", DocKind::Scene).unwrap();
        place(&root, &scene.id, 0.0, 0.0).unwrap();
        let note = place_note(&root, None, 200.0, 0.0, "why does she go back?", None).unwrap();

        let arrow = draw_arrow(&root, &scene.id, &note.id, Some("unanswered")).unwrap();
        assert_eq!(load(&root).arrows.len(), 1);
        assert_eq!(arrow.label.as_deref(), Some("unanswered"));

        // Both ends must exist, an arrow to itself says nothing, and the same
        // pair is not connected twice.
        assert!(draw_arrow(&root, &scene.id, "nowhere", None).is_err());
        assert!(draw_arrow(&root, &scene.id, &scene.id, None).is_err());
        assert!(draw_arrow(&root, &scene.id, &note.id, None).is_err());

        // Removing a note takes its arrows with it — an arrow into nothing
        // would draw a line to the origin.
        remove_note(&root, &note.id).unwrap();
        assert!(load(&root).notes.is_empty());
        assert!(load(&root).arrows.is_empty(), "an arrow must not outlive its end");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_canvas_written_before_notes_existed_still_loads() {
        let (base, root) = scratch("legacy");
        std::fs::create_dir_all(root.join(DIR)).unwrap();
        // Exactly the shape the app wrote before notes and arrows.
        std::fs::write(
            root.join(DIR).join(FILE),
            r#"{"cards":[{"id":"a","x":1.0,"y":2.0}],"updatedAt":5}"#,
        )
        .unwrap();
        let canvas = load(&root);
        assert_eq!(canvas.cards.len(), 1, "an older layout must not be dropped");
        assert!(canvas.notes.is_empty() && canvas.arrows.is_empty());
        std::fs::remove_dir_all(&base).ok();
    }

}
