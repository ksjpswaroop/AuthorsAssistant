//! Creates a project folder with a few scenes, for testing the UI without
//! clicking through the launch flow.
//!
//!     cargo run --example seed_project -- /tmp/AegisDemo "The Salt Road"
//!
//! Pair it with AEGISDRAFT_OPEN to launch straight into the studio.

use aegisdraft_lib::model::DocKind;
use aegisdraft_lib::project;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let parent = PathBuf::from(args.next().unwrap_or_else(|| "/tmp".to_string()));
    let name = args.next().unwrap_or_else(|| "Demo Book".to_string());

    std::fs::create_dir_all(&parent).expect("could not create the parent folder");
    let summary = project::create(&parent, &name, "novel").expect("could not create the project");
    let root = PathBuf::from(&summary.path);

    // create() already made Manuscript / Characters / Worldbuilding / Canon / Research.
    // Nest inside them rather than adding a second set at the root.
    let existing = project::documents(&root).unwrap();
    let folder_named = |name: &str| {
        existing
            .iter()
            .find(|d| d.title == name && d.kind == DocKind::Folder)
            .map(|d| d.id.clone())
    };
    let manuscript = folder_named("Manuscript");
    let act = project::create_document(&root, manuscript.as_deref(), "Act I — Ember", DocKind::Folder).unwrap();

    let scenes = [
        (
            "1. The Well at Kharsa",
            "The well at Kharsa was the only honest thing in the town and it was going dry, which the town had decided to treat as a rumour rather than a measurement.\n\nTamsin bought two skins of water with money she did not have, and watched a man on the far side of the courtyard lower a knotted cord into the well and write down what came back up.",
            "Tamsin buys water she cannot pay for and meets the cartographer.",
            "Plot",
            "Done",
        ),
        (
            "2. What the Cartographer Kept",
            "The descent took the leg in the second hour, on ground that had held six men before him and would hold six after, which he seemed to find more insulting than the break itself.\n\nThe surgeon splinted it badly, apologised for the splint, and said four weeks at best.",
            "His leg is broken on the descent and splinted badly.",
            "Character",
            "Done",
        ),
        (
            "3. Nightfall at the Rim",
            "The rim of the basin held the last of the heat the way a kiln holds it, grudgingly and all at once. Tamsin counted forty paces out to the marker stone and stopped counting when the wind erased her fortieth footprint before she had finished making it.\n\nBelow them the salt went on being white in the dark. Not glowing — she wanted to be exact about that later — just refusing to be black like everything else.\n\nWhen the moon came up it came up wrong. She checked the marker stone. She checked it again.\n\nThe stone had been moved.",
            "The moon rises in the wrong place. The marker stone has been moved.",
            "Plot",
            "Draft",
        ),
    ];

    for (title, body, synopsis, label, status) in scenes {
        let doc = project::create_document(&root, Some(&act.id), title, DocKind::Scene).unwrap();
        project::write_body(&root, &doc.id, body).unwrap();
        project::update_meta(&root, &doc.id, Some(synopsis), Some(label), Some(status)).unwrap();
    }

    let cast = folder_named("Characters");
    let tamsin = project::create_document(&root, cast.as_deref(), "Tamsin Reyes", DocKind::Sheet).unwrap();
    project::write_body(
        &root,
        &tamsin.id,
        "## Role\n\nProtagonist. Salt-factor, formerly bonded.\n\n## Continuity facts\n\nLeft-handed. Scar across the right palm. Cannot swim.",
    )
    .unwrap();

    let canon = folder_named("Canon");
    let facts = project::create_document(&root, canon.as_deref(), "Tamsin — facts", DocKind::Canon).unwrap();
    project::write_body(
        &root,
        &facts.id,
        "## Continuity facts\n\n- Tamsin Reyes is left-handed. Source: Characters/Tamsin Reyes.\n- Tamsin has a scar across the right palm. Source: 1. The Well at Kharsa.\n",
    )
    .unwrap();

    // Link sheets to the scenes they appear in, so the character graph has real
    // edges rather than an empty canvas.
    let scene_ids: Vec<String> = project::documents(&root)
        .unwrap()
        .into_iter()
        .filter(|d| d.kind == DocKind::Scene && d.word_count > 0)
        .map(|d| d.id)
        .collect();

    for id in &scene_ids {
        project::link(&root, &tamsin.id, id, "appears_in").unwrap();
    }
    let carto = project::create_sheet(&root, "The Cartographer", DocKind::Sheet, None, None).unwrap();
    for id in scene_ids.iter().take(2) {
        project::link(&root, &carto.id, id, "appears_in").unwrap();
    }
    let basin =
        project::create_sheet(&root, "The Kharsa Basin", DocKind::Location, None, scene_ids.first().map(|s| s.as_str()))
            .unwrap();
    let _ = basin;

    // Story dates in the notes, so the chronological timeline has something to
    // reorder — day 12 before day 4 is deliberately out of narrative order.
    for (id, day) in scene_ids.iter().zip([1i64, 12, 4]) {
        project::update_extended(&root, id, Some(&format!("Takes place on Day {day}.")), Some(2000), Some("Tamsin"), None, None)
            .unwrap();
        project::set_keywords(&root, id, &["tamsin".to_string(), "kharsa".to_string()]).unwrap();
    }

    // A deliberately reused paragraph, so integrity and clustering have a real
    // finding rather than an empty result.
    if let Some(first) = scene_ids.first() {
        let echo = project::create_document(&root, None, "7. The Second Well", DocKind::Scene).unwrap();
        let original = project::read_body(&root, first).unwrap();
        let opening = original.split("\n\n").next().unwrap_or_default().to_string();
        project::write_body(
            &root,
            &echo.id,
            &format!("{opening}\n\nShe had written the same sentence about the same well twice, a year apart."),
        )
        .unwrap();
        project::update_meta(&root, &echo.id, Some("An echo of chapter one, probably to cut."), Some("Plot"), Some("Draft"))
            .unwrap();
    }

    println!("{}", summary.path);
}
