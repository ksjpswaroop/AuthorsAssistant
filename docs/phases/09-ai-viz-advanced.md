# Phase 9: AI Visualization Advanced and Analytics (v2)

Purpose: extend Phase 8 with mind maps, flowcharts, story-bible infographics, semantic scene clustering, and pacing/redundancy/gap analytics—agent-tool-ready for Hermes. Status: **not started**. Months 17–18. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 8, Phase 3 embeddings, and Phase 4 similarity concepts. Does not edit phases 01–05 or [AGENTS.md](../AGENTS.md) wholesale.

## Goals

- AI mind maps for plot threads, themes, and character arcs
- Flowcharts for branching plot logic and chronology
- Story-bible infographics (family tree, magic system, location legends)
- Scene clustering → proposed smart collections
- Pacing, redundancy, and metadata gap analysis views
- Wire tools so App Controller / Deep Continuity can invoke them (cross-link from [TOOLS.md](../TOOLS.md#v2-tools))

## Checklist

### Mind maps and flowcharts

- [ ] Mind map from binder structure, synopsis text, or query (“theme connections”, “Character X arcs”)
- [ ] Interactive graph (React Flow / D3); click node → open binder doc
- [ ] Tool: `generate_mind_map(query, root?) -> graph`
- [ ] Flowchart from scene decision points / branching plots
- [ ] Chronological vs narrative flowchart modes
- [ ] Tool: `generate_flowchart(scenes?, type) -> diagram`

### Story-bible infographics

- [ ] Family tree from character relationship metadata
- [ ] Magic-system / rule flowchart from worldbuilding docs
- [ ] Location map legend / reference sheet from location sheets
- [ ] Tool: `generate_infographic(type, source_ids?) -> artifact`

### Clustering and collections

- [ ] Embed synopses + keywords; cluster (e.g. HDBSCAN / threshold cosine)
- [ ] UI: proposed groups (“confrontation scenes”) → accept as collection
- [ ] Detect near-duplicate / redundant scenes
- [ ] Tool: `suggest_clusters(threshold) -> list[CollectionProposal]`

### Analytics (integrity synergies)

- [ ] Pacing / tension curve across chapters (local model or heuristic + viz)
- [ ] Redundancy clusters highlighted on corkboard/outliner
- [ ] Gap analysis: e.g. “no scenes tagged romance in Act 2”
- [ ] Exportable analytics snapshot JSON for reports

### Agent wiring

- [ ] Expose all Phase 9 tools in sidecar OpenAPI-style schemas
- [ ] Command bar examples: “Map theme connections”, “Suggest collections for confrontations”, “Show pacing dips”
- [ ] Deep Continuity may attach mind-map / graph evidence to plot-hole reports (optional)
- [ ] Do not rewrite AGENTS.md; document hooks here + TOOLS v2 section only

## Acceptance criteria

- Mind map for “Character X” shows connected scenes/themes; clicking opens the scene
- `suggest_clusters` proposes groups that accept into a Phase 6 collection without duplicating files
- Infographic “family_tree” generates from character sheets and saves under Research/Worldbuilding
- Pacing view flags a flat middle act; gap analysis lists missing keyword coverage by act
- All tools run locally; network not required

## Out of scope

- Replacing Phase 8 core viz (this phase builds on it)
- Cloud diagram hosts or uploading manuscript for viz
- Full rewrite of Hermes agent persona docs

## Notes

Keep visualizations as projections. Clustering writes **collections** (virtual), not new scene files, unless the author explicitly duplicates. Snapshot sidecars may store analytics filter state alongside Phase 8 canvas JSON.
