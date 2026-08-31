/**
 * Sample project used when the UI runs in a plain browser instead of the Tauri
 * shell. The content is the same sample manuscript as the design canvas
 * (design/Main.dc.html), so a design review and a code review look at the same
 * thing. Nothing here is written to disk.
 */

import type { DocumentRow } from './types'
import { countWords } from './format'

export const DEMO_BODIES: Record<string, string> = {
  ch1: `The well at Kharsa was the only honest thing in the town and it was going dry, which the town had decided to treat as a rumour rather than a measurement.

Tamsin bought two skins of water with money she did not have, and while the factor was counting what she had given him she watched a man on the far side of the courtyard lower a knotted cord into the well and write down what came back up.

Nobody else in Kharsa was writing anything down. That was how she knew he was not from here, and that he would be leaving.`,

  ch2: `The descent took the leg in the second hour, on ground that had held six men before him and would hold six after, which he seemed to find more insulting than the break itself.

The surgeon splinted it badly, apologised for the splint, and said four weeks at best, and he is not a patient man.

He would not say who had set it the first time. He would not say there had been a first time. Tamsin looked at the old ridge in the bone through the skin and did not ask again.`,

  ch3: `A tariff in the basin is not a price. It is a number of days, and the days are counted by someone who is not you.

Nine, the factor said, and wrote it on the back of her own receipt so that she would have to carry the proof of it herself. Nine days to cross and clear, and after that the wind turns and the crossing is closed and the debt does not close with it.`,

  ch4: `The rim of the basin held the last of the heat the way a kiln holds it, grudgingly and all at once. Tamsin counted forty paces out to the marker stone and stopped counting when the wind erased her fortieth footprint before she had finished making it.

Below them the salt went on being white in the dark. Not glowing — she wanted to be exact about that later, when the cartographer asked her to write it down — just refusing to be black like everything else.

"We cross at moonrise or we do not cross," he said. He had said it four times since noon, each time as though the thought had only that moment arrived and surprised him.

He sprinted for the marker stone the moment the light failed, favouring nothing, and Tamsin let him go. The wind took the sound of him in three strides. She stayed where she was and did the arithmetic she had been avoiding since Kharsa: two skins of water, nine days of tariff owed, one man who lied about maps for a living.

When the moon came up it came up wrong — too far south, too thin, the colour of a coin left out in weather. She had crossed the basin twice before and both times the moon had risen over the shoulder of the Rim. She checked the marker stone. She checked it again.

The stone had been moved.`,

  ch5: `Nobody in the basin gave the wind a name until it took something from them, and then everybody had a different one for it, and the cartographer wrote all of them down as though the disagreement were the useful part.

Tamsin waited until he had finished the page before she told him about the stone.`,

  ch6: '',

  tamsin: `## Role

Protagonist. Salt-factor, formerly bonded.

## Wants

To clear the tariff before the crossing closes.

## Continuity facts

Left-handed. Scar across the right palm from chapter 1. Cannot swim. Refers to the cartographer only by trade, never by name, until chapter 9.`,

  carto: '',
  basin: '',
  salt: '',
  clippings: '',
}

function doc(
  id: string,
  parentId: string | null,
  title: string,
  kind: DocumentRow['kind'],
  position: number,
  extra: Partial<DocumentRow> = {},
): DocumentRow {
  const body = DEMO_BODIES[id] ?? ''
  return {
    id,
    parentId,
    title,
    kind,
    path: kind === 'folder' ? null : `manuscript/scenes/${id}.md`,
    position,
    synopsis: null,
    label: null,
    status: null,
    wordCount: countWords(body),
    updatedAt: Date.now() - 86_400_000,
    ...extra,
  }
}

export const DEMO_DOCS: DocumentRow[] = [
  doc('manuscript', null, 'Manuscript', 'folder', 0),
  doc('act1', 'manuscript', 'Act I — Ember', 'folder', 1),
  doc('ch1', 'act1', '1. The Well at Kharsa', 'scene', 2, {
    synopsis: 'Tamsin buys water she cannot pay for and meets the cartographer, who is measuring a well that everyone else is drinking from.',
    label: 'Plot',
    status: 'Done',
  }),
  doc('ch2', 'act1', '2. What the Cartographer Kept', 'scene', 3, {
    synopsis: 'His leg is broken on the descent and splinted badly. Four weeks at best, the surgeon says.',
    label: 'Character',
    status: 'Done',
  }),
  doc('ch3', 'act1', '3. Salt and Debt', 'scene', 4, {
    synopsis: 'The tariff is explained. Nine days until the crossing closes.',
    label: 'Setup',
    status: 'Done',
  }),
  doc('act2', 'manuscript', 'Act II — The Crossing', 'folder', 5),
  doc('ch4', 'act2', '4. Nightfall at the Rim', 'scene', 6, {
    synopsis: 'They wait out the heat on the rim. The moon rises in the wrong place. Tamsin discovers the marker stone has been moved.',
    label: 'Plot',
    status: 'Draft',
    updatedAt: Date.now() - 120_000,
  }),
  doc('ch5', 'act2', '5. The Wind Takes a Name', 'scene', 7, {
    synopsis: 'Every name the basin has for the wind, written down in one column.',
    label: 'Plot',
    status: 'Draft',
  }),
  doc('ch6', 'act2', '6. Untitled scene', 'scene', 8, { status: 'Empty' }),
  doc('characters', null, 'Characters', 'folder', 9),
  doc('tamsin', 'characters', 'Tamsin Reyes', 'sheet', 10, { synopsis: 'Protagonist. Salt-factor, formerly bonded.' }),
  doc('carto', 'characters', 'The Cartographer', 'sheet', 11),
  doc('worldbuilding', null, 'Worldbuilding', 'folder', 12),
  doc('basin', 'worldbuilding', 'The Kharsa Basin', 'lore', 13),
  doc('salt', 'worldbuilding', 'Salt Roads and Tariffs', 'lore', 14),
  doc('research', null, 'Research', 'folder', 15),
  doc('clippings', 'research', 'Evaporite geology (clippings)', 'research', 16),
]
