# Beta release-notes template

Format for v0.1.0-beta.1 and later. Beta notes differ from alpha notes in one
way: **breaking-risk sections are mandatory**, because beta users are expected
to keep real projects across releases. Alpha notes may compress these into a
paragraph; beta notes may not.

Fill every `TBD` before publishing; a note item is only added when its claim
has recorded evidence (docs/distribution.md rules apply verbatim).

~~~markdown
## [0.1.0-beta.1] - YYYY-MM-DD

<One paragraph: what this release is, headline features since the last
release, and the phase (beta scope per planning.md 6e).>

### Breaking risks (read before upgrading)

- **GStreamer version floor.** Minimum tested runtime: GStreamer 1.20 / GES
  1.20 (Debian 12 ships 1.22; the CI builders install 1.20 via
  ubuntu-22.04). Older FFMPEG/GStreamer plugin sets render black or fail at
  pipeline link. TBD: exact oldest version actually launched against this
  release.
- **Proxy format.** Proxies are single-profile 1280x720 VP9 WebM previews
  attached per asset; there is no preset system (Proxy Presets is recorded
  na). Projects created with a different proxy tool keep their attachments
  but TB D: state whether re-attach/re-verify is required after upgrade.
- **Project schema.** Current schema version: TBD (see
  `PROJECT_SCHEMA_VERSION` in crates/saturn-core). Loading older projects
  auto-migrates; loading a NEWER file on an older app build is rejected.
- **Absolute media paths.** Projects embed absolute paths at import; moving
  media shows clips offline until relinked. Unchanged from alpha, restated
  because beta users carry projects across machines.
- **Anything else removed or renamed in this release**: TBD.

### Install and launch evidence

- Reuse the evidence table from docs/distribution.md verbatim - the note
  never upgrades "build succeeded" into "works on your machine".

### Commands and parity

- Catalogue size and shipped count (from docs/reference/command-catalogue.json).
- Tier coverage table (copy from docs/PARITY.md, it is generated).

### Known limits snapshot

- Copy the current README "Known limitations" list; do not paraphrase it
  into optimism.
~~~

Keep a Changelog ordering and SemVer adherence stay unchanged from
alpha releases.
