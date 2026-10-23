//! Custom sheet templates (K2P-OS §5, Phase 6).
//!
//! `create_sheet` seeded every character sheet from one fixed shape. That shape
//! is a guess about what a book needs, and it is wrong for most books: a
//! historical novel wants dates and sources, a technical book wants an API
//! surface, a screenplay wants nothing of the sort.
//!
//! So a template is a **Markdown file in the project**, under
//! `.aegis/templates/`. The author edits it in any editor, it travels with the
//! project, and it survives this app. The built-in shapes remain as the
//! starting point and as the fallback when nothing is defined.
//!
//! # Placeholders, and why there are only three
//!
//! `{{name}}`, `{{date}}` and `{{kind}}`. Anything more becomes a small
//! templating language nobody asked for, and a sheet is a starting point an
//! author fills in — not a form a machine renders.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{err, AppResult};
use crate::model::DocKind;

pub const DIR: &str = ".aegis/templates";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    /// The filename stem, and how it is chosen: `character`, `location`, or
    /// anything the author adds.
    pub id: String,
    pub name: String,
    /// Which kind of document this template makes.
    pub kind: DocKind,
    pub body: String,
    /// False for the two shipped shapes, which exist even before a project has
    /// a templates folder.
    pub custom: bool,
}

/// The shipped starting points. Kept in code so a brand-new project has them
/// without needing files written first.
pub fn built_in(kind: DocKind) -> &'static str {
    match kind {
        DocKind::Location => {
            "# {{name}}

## What it is

## What it looks like

## Who is there

## Continuity facts

- 

## First appears

"
        }
        _ => {
            "# {{name}}

## Role

## Wants

## Refuses to do

## Continuity facts

- 

## First appears

"
        }
    }
}

fn dir(root: &Path) -> std::path::PathBuf {
    root.join(DIR)
}

/// Every template available: the shipped shapes, plus anything the author has
/// written, with a custom template of the same id winning.
pub fn list(root: &Path) -> Vec<Template> {
    let mut out = vec![
        Template {
            id: "character".into(),
            name: "Character".into(),
            kind: DocKind::Sheet,
            body: built_in(DocKind::Sheet).to_string(),
            custom: false,
        },
        Template {
            id: "location".into(),
            name: "Location".into(),
            kind: DocKind::Location,
            body: built_in(DocKind::Location).to_string(),
            custom: false,
        },
    ];

    let Ok(entries) = std::fs::read_dir(dir(root)) else {
        return out;
    };
    let mut found: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    found.sort();

    for path in found {
        let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
            continue;
        };
        let Ok(body) = std::fs::read_to_string(&path) else { continue };
        // The kind is the template's own business, declared on its first line
        // as an HTML comment so it never shows in the rendered sheet.
        let kind = declared_kind(&body).unwrap_or(DocKind::Sheet);
        let name = title_of(&body).unwrap_or_else(|| humanise(&stem));

        match out.iter_mut().find(|t| t.id == stem) {
            // An author's version of `character` replaces the shipped one
            // rather than sitting beside it: two templates with one name is a
            // choice nobody wants to make twice.
            Some(existing) => {
                existing.body = body;
                existing.name = name;
                existing.kind = kind;
                existing.custom = true;
            }
            None => out.push(Template { id: stem, name, kind, body, custom: true }),
        }
    }
    out
}

/// `<!-- kind: location -->` on any of the first few lines.
fn declared_kind(body: &str) -> Option<DocKind> {
    for line in body.lines().take(5) {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("<!--").and_then(|r| r.strip_suffix("-->")) {
            if let Some(value) = rest.trim().strip_prefix("kind:") {
                return Some(DocKind::parse(value.trim()));
            }
        }
    }
    None
}

/// A template names itself in its own first heading, minus the placeholder.
fn title_of(body: &str) -> Option<String> {
    body.lines()
        .find(|l| l.trim_start().starts_with("# "))
        .map(|l| l.trim_start().trim_start_matches("# ").replace("{{name}}", "").trim().to_string())
        .filter(|t| !t.is_empty())
}

fn humanise(stem: &str) -> String {
    let mut out = String::new();
    for word in stem.split(['-', '_']).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(c) = chars.next() {
            out.extend(c.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() { stem.to_string() } else { out }
}

pub fn get(root: &Path, id: &str) -> Option<Template> {
    list(root).into_iter().find(|t| t.id == id)
}

/// Fill a template. Three placeholders, deliberately.
pub fn render(body: &str, name: &str, kind: DocKind, now_iso: &str) -> String {
    let filled = body
        .replace("{{name}}", name)
        .replace("{{date}}", now_iso)
        .replace("{{kind}}", kind.as_str());
    // The kind declaration is instruction to this app, not content for a sheet.
    filled
        .lines()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with("<!--") && t.contains("kind:") && t.ends_with("-->"))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_start()
        .to_string()
}

/// Write a template. Creating one from a built-in shape is how an author
/// customises: they get the shipped text to edit rather than a blank file.
pub fn save(root: &Path, id: &str, body: &str) -> AppResult<Template> {
    let id = crate::project::slugify(id);
    if id == "untitled" || id.is_empty() {
        return err("a template needs a name");
    }
    if body.trim().is_empty() {
        return err("a template with no body would make an empty sheet");
    }
    std::fs::create_dir_all(dir(root))?;
    std::fs::write(dir(root).join(format!("{id}.md")), body)?;
    get(root, &id).ok_or_else(|| crate::error::AppError("the template vanished after saving".into()))
}

/// Delete a custom template. The shipped shapes come back when the file goes,
/// which is the honest behaviour: they were never removed, only overridden.
pub fn forget(root: &Path, id: &str) -> AppResult<()> {
    let path = dir(root).join(format!("{}.md", crate::project::slugify(id)));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "aegis-tpl-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = crate::project::create(&base, "Templates", "novel").unwrap();
        (base, std::path::PathBuf::from(summary.path))
    }

    #[test]
    fn a_new_project_has_the_shipped_shapes_and_nothing_else() {
        let (base, root) = scratch("builtin");
        let all = list(&root);
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|t| !t.custom));
        assert_eq!(all[0].id, "character");
        assert_eq!(all[1].kind, DocKind::Location);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn an_authors_template_replaces_the_shipped_one_of_the_same_name() {
        let (base, root) = scratch("override");
        save(
            &root,
            "character",
            "# {{name}}\n\n## Sources\n\n## Dates\n\n## Handedness\n",
        )
        .unwrap();

        let all = list(&root);
        assert_eq!(all.len(), 2, "an override replaces, it does not add: {all:?}");
        let character = all.iter().find(|t| t.id == "character").unwrap();
        assert!(character.custom);
        assert!(character.body.contains("Handedness"));
        assert!(!character.body.contains("Refuses to do"), "the shipped text is gone");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_new_template_declares_its_own_kind_and_names_itself() {
        let (base, root) = scratch("new");
        save(
            &root,
            "Field Site",
            "<!-- kind: location -->\n# {{name}} — Field Site\n\n## Coordinates\n\n## Access\n",
        )
        .unwrap();

        let made = get(&root, "field-site").expect("slugified id");
        assert_eq!(made.kind, DocKind::Location);
        assert_eq!(made.name, "— Field Site", "named from its own heading");
        assert!(made.custom);
        assert_eq!(list(&root).len(), 3);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn rendering_fills_the_three_placeholders_and_hides_the_declaration() {
        let out = render(
            "<!-- kind: location -->\n# {{name}}\n\nMade {{date}} as a {{kind}}.\n",
            "Kharsa",
            DocKind::Location,
            "2026-09-01",
        );
        assert!(out.starts_with("# Kharsa"), "{out}");
        assert!(out.contains("Made 2026-09-01 as a location."));
        assert!(!out.contains("kind:"), "the declaration is instruction, not content: {out}");
        assert!(!out.contains("{{"), "no placeholder survives: {out}");
    }

    #[test]
    fn deleting_a_custom_template_restores_the_shipped_one() {
        let (base, root) = scratch("delete");
        save(&root, "character", "# {{name}}\n\n## Only this\n").unwrap();
        assert!(get(&root, "character").unwrap().custom);

        forget(&root, "character").unwrap();
        let back = get(&root, "character").unwrap();
        assert!(!back.custom, "the shipped shape was overridden, never removed");
        assert!(back.body.contains("Refuses to do"));

        // Deleting one that was never there is not an error.
        forget(&root, "never-existed").unwrap();
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn an_empty_or_unnamed_template_is_refused() {
        let (base, root) = scratch("refuse");
        assert!(save(&root, "character", "   ").is_err());
        assert!(save(&root, "---", "# body").is_err());
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_template_is_a_file_the_author_can_edit_outside_the_app() {
        let (base, root) = scratch("file");
        save(&root, "character", "# {{name}}\n\n## Mine\n").unwrap();
        let path = root.join(DIR).join("character.md");
        assert!(path.exists());

        // Edited in another editor, and picked up on the next read.
        std::fs::write(&path, "# {{name}}\n\n## Edited elsewhere\n").unwrap();
        assert!(get(&root, "character").unwrap().body.contains("Edited elsewhere"));
        std::fs::remove_dir_all(&base).ok();
    }
}
