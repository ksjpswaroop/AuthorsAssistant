//! Scrivenings (Phase 6): edit a folder's children as one continuous document.
//!
//! The dangerous half is the way back. When the author saves, the combined text
//! must split into exactly the pieces it came from and land in the right files.
//! If the separators no longer line up — someone deleted one, or pasted a block
//! containing the marker — the only safe answer is to refuse and keep every
//! file as it was. A partial write here silently shreds a chapter.

use crate::error::{err, AppResult};

/// Written between documents in the stitched buffer. Chosen to be something an
/// author would not type by accident, and stable so a saved buffer round-trips.
pub const SEPARATOR: &str = "\n\n<!-- aegis:scrivening -->\n\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub id: String,
    pub body: String,
}

/// Joins pieces into one editable buffer.
pub fn stitch(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|p| p.body.as_str())
        .collect::<Vec<_>>()
        .join(SEPARATOR)
}

/// Splits an edited buffer back into one body per document, in the same order
/// it was stitched. Refuses anything that does not line up exactly.
pub fn split(combined: &str, ids: &[String]) -> AppResult<Vec<Piece>> {
    if ids.is_empty() {
        return err("nothing to split");
    }
    let parts: Vec<&str> = combined.split(SEPARATOR).collect();
    if parts.len() != ids.len() {
        return err(format!(
            "this view held {} documents but the text now has {} sections — \
             a separator was changed, so nothing was saved",
            ids.len(),
            parts.len()
        ));
    }
    Ok(ids
        .iter()
        .zip(parts)
        .map(|(id, body)| Piece { id: id.clone(), body: body.trim_matches('\n').to_string() })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(id: &str, body: &str) -> Piece {
        Piece { id: id.into(), body: body.into() }
    }

    #[test]
    fn stitch_then_split_returns_the_same_pieces() {
        let pieces = vec![piece("a", "First scene."), piece("b", "Second scene."), piece("c", "Third.")];
        let ids: Vec<String> = pieces.iter().map(|p| p.id.clone()).collect();
        let combined = stitch(&pieces);
        assert_eq!(split(&combined, &ids).unwrap(), pieces);
    }

    #[test]
    fn edits_inside_one_section_land_only_in_that_document() {
        let pieces = vec![piece("a", "Untouched."), piece("b", "Old text."), piece("c", "Also untouched.")];
        let ids: Vec<String> = pieces.iter().map(|p| p.id.clone()).collect();
        let edited = stitch(&pieces).replace("Old text.", "New and longer text.");

        let out = split(&edited, &ids).unwrap();
        assert_eq!(out[0].body, "Untouched.");
        assert_eq!(out[1].body, "New and longer text.");
        assert_eq!(out[2].body, "Also untouched.");
    }

    #[test]
    fn a_deleted_separator_refuses_the_whole_save() {
        let pieces = vec![piece("a", "One."), piece("b", "Two."), piece("c", "Three.")];
        let ids: Vec<String> = pieces.iter().map(|p| p.id.clone()).collect();
        // The author selected across a boundary and typed over it.
        let mangled = stitch(&pieces).replacen(SEPARATOR, " ", 1);

        let result = split(&mangled, &ids);
        assert!(result.is_err(), "a lost boundary must not be guessed at");
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("nothing was saved"), "the message must say the save was refused: {msg}");
    }

    #[test]
    fn an_extra_separator_also_refuses() {
        let pieces = vec![piece("a", "One."), piece("b", "Two.")];
        let ids: Vec<String> = pieces.iter().map(|p| p.id.clone()).collect();
        let mangled = format!("{}{}pasted", stitch(&pieces), SEPARATOR);
        assert!(split(&mangled, &ids).is_err());
    }

    #[test]
    fn an_emptied_section_is_allowed_and_stays_empty() {
        // Clearing a scene is a legitimate edit, not a corruption.
        let pieces = vec![piece("a", "Keep."), piece("b", "Delete me."), piece("c", "Keep too.")];
        let ids: Vec<String> = pieces.iter().map(|p| p.id.clone()).collect();
        let edited = stitch(&pieces).replace("Delete me.", "");

        let out = split(&edited, &ids).unwrap();
        assert_eq!(out[1].body, "", "an emptied section saves as empty");
        assert_eq!(out[0].body, "Keep.");
        assert_eq!(out[2].body, "Keep too.");
    }

    #[test]
    fn a_single_document_round_trips() {
        let pieces = vec![piece("only", "Just the one.")];
        let ids: Vec<String> = pieces.iter().map(|p| p.id.clone()).collect();
        assert_eq!(split(&stitch(&pieces), &ids).unwrap(), pieces);
    }

    #[test]
    fn splitting_with_no_ids_is_refused() {
        assert!(split("anything", &[]).is_err());
    }
}
