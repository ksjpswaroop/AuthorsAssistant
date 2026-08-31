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

    // create() already made Manuscript / Characters / Worldbuilding / Research.
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

    println!("{}", summary.path);
}
