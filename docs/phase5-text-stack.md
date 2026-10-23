# Phase 5 spike — text stack for `saturn-text`

**Date:** 2026-10-22 · **Status:** design decision record for the text-rendering workstream
**Context:** FilmCraft parity Phase 5 (graphics, text, captions), as scoped at the time
**Decision required by the plan:** shape with `rustybuzz` + `swash`, or adopt
`cosmic-text` if it covers vertical text. Required capabilities: ligatures, kerning,
RTL, CJK, vertical layout, variable fonts.

## Decision

**Adopt `cosmic-text` (0.19) as the `saturn-text` foundation.** Implement vertical
text as a thin layer on top, inside `saturn-text`.

## Evidence (docs.rs, cosmic-text 0.19.0, 2026-09-23)

| Requirement | cosmic-text 0.19 | Verdict |
|---|---|---|
| Ligatures / kerning | Shaping via `harfrust` 0.5 (the maintained HarfBuzz port; `rustybuzz`'s successor) with `Shaping::Advanced` | ✅ covered |
| Kerning pairs / OpenType features | `Feature`/`FeatureTag` APIs | ✅ covered |
| RTL + bidi | `unicode-bidi` 0.3, `ShapeSpan` bidirectional processing, `BidiParagraphs` | ✅ covered |
| CJK | `unicode-script` segmentation + `fontdb` fallback + platform fallback lists | ✅ covered |
| Vertical layout (tategaki) | **Not built in.** No vertical writing mode in the public API; `Feature` can request `vert`/`vrt2` glyph substitution, but column stacking is on us | ⚠️ thin layer in `saturn-text` |
| Variable fonts | `skrifa` 0.40 re-export + swash scaling; `Weight`/`Stretch`/`Style` and `FontFeatures` exposed. Axis-by-axis (`wght`/`opsz`) setting is not a first-class API | ⚠️ verify at integration with a fixture; fall back to direct `swash` variation settings if needed |
| Tracking / leading | `LetterSpacing`, `Metrics` (font size + line height) | ✅ covered |
| Rasterization | `SwashCache` → glyph images; render-scale, not screen-scale | ✅ matches the plan's RGBA-layer requirement |
| Font discovery / user fonts | `fontdb` (system + in-memory loads) | ✅ covered |
| Editing (caret, hit-test) | `Editor`, `Cursor`, `Motion`, hit-test via `LayoutCursor` | ✅ bonus: on-canvas text editing gets caret support for free |

## Why not raw `rustybuzz` + `swash`

`cosmic-text` is precisely that stack (harfrust is rustybuzz's lineage) **plus**
paragraph bidi, font fallback, line breaking, alignment, editing, and caches.
Hand-rolling those for the Essential Graphics panel would add months for no
behaviour difference. The one gap (vertical writing mode) is bounded, testable
work we would have had to write in either scenario.

## Consequences

1. `saturn-text` depends on `cosmic-text` (default features; `swash` rasterization on).
2. Its first deliverable is `shape_rgba(text, style, scale) -> RgbaLayer` for the wgpu
   compositor: rasterize at render scale, cache glyphs in `SwashCache`, return an
   alpha layer the compositor tints/composites.
3. Vertical text: shape runs with `vert`/`vrt2` features per glyph and stack
   columns top-to-bottom in `saturn-text`; fixtures: JP vertical caption sample +
   RTL Arabic + CJK horizontal (plan's Phase 5 test list).
4. Variable fonts: integration test with a variable font fixture asserting the
   `wght` axis changes advance widths; if cosmic-text's API proves insufficient,
   patch at the swash layer (documented escape hatch, no crate rewrite).
5. Render cost guard: shape-cache per (text, style, scale); the 30 fps gate from
   the parity plan applies to the first on-canvas text frame.

**Spike verdict: GO for cosmic-text. Revisit only if vertical-text fixtures fail
after the bounded layer above (escalation: rustybuzz + swash + custom layout,
estimated +3 agent-days).**
