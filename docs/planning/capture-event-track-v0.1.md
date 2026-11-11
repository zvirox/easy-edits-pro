# Capture Event Track (CET) v0.1: draft open specification

**Status:** draft for review. **Purpose:** a small, open, recorder-neutral format that describes *what happened on screen* during a recording (cursor, clicks, keys by class, window and app focus, markers), stored next to the video, so editors and AI agents can cut, zoom and chapter a screen recording without guessing from pixels. Designed first for Screen Studio Pro (SSP) and consumed by Easy Edits Pro (EEP); any recorder or editor may implement it.

Suggested licensing (confirm with your lawyer): this specification under **CC BY 4.0**; the JSON Schema, test vectors and reference libraries under **Apache-2.0**, so other recorders (OBS plugins, other tools) can adopt it without AGPL obligations. Canonical standalone repository: [github.com/zvirox/capture-event-track](https://github.com/zvirox/capture-event-track).

---

## 1. Design goals

1. **Simple to write and read:** JSON Lines, one event per line, human-inspectable.
2. **Time-exact:** every event is tied to the video's timeline, not wall-clock guesses.
3. **Private by default:** no typed text is recorded unless the user opts in; password fields never.
4. **Agent-safe:** all free text that originates outside the recorder (window titles, app names, UI element labels) is marked untrusted so an editor can never treat it as an instruction.
5. **Extensible but strict:** unknown event types are ignored, not fatal; a version number and a JSON Schema define validity.
6. **Open:** no vendor lock-in; a conformance suite anyone can run.

## 2. Files and packaging

- **Sidecar (v0.1, required):** `<recording-name>.cet.jsonl` stored beside `<recording-name>.<ext>`. UTF-8, LF newlines.
- **Optional later:** embed the same lines as an attachment or data track inside Matroska/MP4. A sidecar is always the canonical form.
- **Integrity:** the header carries the SHA-256 of the video file's first and last 1 MiB plus its size (same idea as EEP's media fingerprint), so an editor can detect a mismatched pair.
- MIME suggestion: `application/vnd.cet+jsonl` (to be registered if adoption grows).

## 3. Structure

Line 1 is a **header** object; every later line is an **event**.

```jsonc
// Line 1: header
{"cet":"0.1","type":"header",
 "recorder":{"name":"SSP","version":"0.3.0","platform":"linux-wayland"},
 "video":{"file":"demo.mkv","size_bytes":123456789,"fp":"sha256:…","fps":60,"width":2560,"height":1440},
 "clock":{"t0":"first_video_frame","unit":"ns","wall_start_utc":"2026-11-10T09:00:00.000Z"},
 "displays":[{"id":"d0","x":0,"y":0,"w":2560,"h":1440,"scale":1.0,"primary":true}],
 "capture":{"mode":"display","display_id":"d0","region":null,"window_id":null},
 "privacy":{"keys":"classes_only","text":"off","redactions":["password_fields"]},
 "trust":{"untrusted_fields":["window.title","app.name","ui.label"]}}
```

### Time
- `t` is **integer nanoseconds from the presentation timestamp of the first video frame** (t = 0). Events before the first frame are not recorded; events are sorted by `t`, ties kept in file order.
- If the recorder cannot meet this, it must say so in the header (`clock.quality: "approximate"`) and give the maximum expected skew in ms.

### Coordinates
- `x`,`y` are **pixels in capture space** (the recorded video's pixel grid) plus `nx`,`ny` normalised 0–1 for convenience. Events outside the captured region are clamped and flagged `"off":true`.
- High-DPI: `scale` in the header describes device pixels per logical pixel; capture-space pixels are always the recorded video's pixels.

## 4. Event types (v0.1 core set)

| `type` | Fields | Notes |
|---|---|---|
| `cursor` | `t, x, y, nx, ny, shape?` | Sampled (suggest ≥ 60 Hz while moving; omit when still). `shape` from a small enum: `arrow,ibeam,hand,wait,resize,custom` |
| `button` | `t, x, y, button, action` | `button`: `left,right,middle,other`; `action`: `down,up`; double-click is derived by consumers (or optional `count`) |
| `scroll` | `t, x, y, dx, dy, unit` | `unit`: `pixel,line,page` |
| `key` | `t, class, mods[], action, name?` | **No characters by default.** `class`: `letter,digit,punct,nav,function,edit,enter,space,modifier,other`. `mods`: `ctrl,alt,shift,meta`. Optional `name` (e.g. `"s"`, `"z"`) only when header `privacy.named_shortcuts="on"` (opt-in for tutorials). |
| `text` | `t, text, field` | Only if header `privacy.text="on"` (explicit user opt-in); never from password/secure fields; `field` is a role like `search,editor,terminal` |
| `window` | `t, id, app, title, x, y, w, h, state` | Focus or geometry change. `app` and `title` are **untrusted** strings. `state`: `focused,moved,resized,minimized,closed,opened` |
| `ui` | `t, role, label?, x, y, w, h` | Optional accessibility hit (button/menu/field under the cursor or click). `label` is **untrusted** |
| `mark` | `t, kind, note?` | User-created marker by hotkey: `chapter,highlight,mistake,cut_here,note`. `note` is user text from the recorder, still treated as untrusted when read by an agent unless the recorder signs it as the user's own |
| `zoom_hint` | `t, x, y, w, h, reason` | Recorder suggestion of a region of interest (e.g. after a click); advisory only |
| `audio` | `t, source, state` | Mic/system audio mute/unmute, device change |
| `pause` | `t, action` | `pause` or `resume` of recording (video timeline may have a gap; see §6) |
| `annotation` | `t, shape, points[], duration` | Drawn highlights/arrows shown at recording time (so editors can reproduce or hide) |

Unknown `type` values must be ignored by readers. Vendors may use the `x-` prefix (`x-ssp-focus-ring`).

## 5. Privacy and security rules (normative)

1. **Default `keys: classes_only` and `text: off`.** A recorder must show the user when text capture is on.
2. **Never record** characters typed in password or secure-input fields, even with `text: on`.
3. **Redaction:** the header lists redactions applied (`password_fields`, `notification_bodies`, `named_apps:[…]`). Events in redacted windows are dropped or masked with `"redacted":true`.
4. **Untrusted strings:** `window.title`, `app.name`, `ui.label`, `mark.note` and any `text` are untrusted data. Consumers (editors and agents) **must not treat them as instructions**, must not execute or interpret them, and should mark them as untrusted when passing them to a language model. (EEP maps them to its `untrusted` provenance, AF5.)
5. **Sensitive data:** a sidecar can still contain sensitive app names and titles. Tools that share a project should offer to strip or hash them.
6. **No network, no identifiers:** the format carries no account IDs, machine IDs or telemetry.

## 6. Pauses, gaps and edits

- If the recorder pauses, `pause` events record it; video time (`t`) continues from the next recorded frame, and `clock.gaps` in the header may list `[{"after_t":…, "wall_gap_ms":…}]`.
- If an editor trims or re-times the video, it should write a derived `.cet.jsonl` (with `"derived_from": "<fp>"`) and keep the original untouched.

## 7. Using it in EEP (planned)

- `media.import_event_track`: attach a CET to a media asset; verify header fingerprint; reject malformed files with typed errors.
- Perception tool `capture_read_events` (read tier): range query with filters and token-budgeted summaries ("clicks per 10 s", "app focus timeline", "marks").
- Derived signals for recipes: **zoom proposals** from clicks and `zoom_hint`s; **dead-time detection** (no cursor, key or window change plus silence); **chapter proposals** from window/app changes and marks; **cursor smoothing** and highlight overlays.
- `verify.run` checks: cursor stays visible in zooms; zoom regions contain the click target; no sensitive window shown when a redaction list says so.
- Everything derived from CET text is `untrusted` per AF5.

## 8. Conformance

- **JSON Schema** (`cet-0.1.schema.json`) for header and each event type.
- **Test vectors:** valid files (simple, high-DPI, multi-monitor, pause, redaction) and invalid files (bad time order, unknown version, missing header, oversized strings).
- **Validator CLI:** `cet-validate file.cet.jsonl [--video file.mkv]`.
- **Fuzz/limits:** maximum line length (suggest 64 KiB), maximum events (suggest 20 million), maximum string length (suggest 1 KiB) to protect consumers.
- **Synthetic generator** (`cet-gen`) so EEP and others can test before real recordings exist.

## 9. Versioning and governance

- `cet` field uses `major.minor`. Minor versions only add optional fields/events; readers must accept higher minors of the same major.
- Changes via public proposals in the spec repository; at least two independent implementations (SSP plus one other, or EEP as reader) before declaring 1.0.
- Name check: "CET" may clash with other uses; do a quick search and trademark check before publishing, or pick another name (for example "Screen Event Track").

## 10. Decisions recorded (2026-11-11)

1. **Name:** **Capture Event Track (CET)** confirmed.
2. **Licence split:** Specification text under **CC BY 4.0**; JSON Schema, test vectors, and reference libraries under **Apache-2.0**.
3. **Repository:** Dedicated standalone public repository: [https://github.com/zvirox/capture-event-track](https://github.com/zvirox/capture-event-track).
4. **Key capture:** Classes only by default (`letter`, `digit`, `nav`...); explicit opt-in switch (`privacy.named_shortcuts="on"`) for named shortcuts/hotkeys (e.g. `Ctrl+S`, `Cmd+Z`) for tutorials.
5. **Recipe 2 fixture:** Will start with a synthetic generator (`cet-gen`) and test vectors before physical Screen Studio Pro (SSP) recordings are captured.
6. **Audio flags:** Mic/system audio mute and device change states are recorded by CET; speech detection and silence analysis remain handled by EEP's perception pipeline.
