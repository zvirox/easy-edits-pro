# saturn-project

Project templates and search bins for Easy Edits Pro.

## Features

- **Project Templates** — Predefined tracks, bins, settings, sequences
- **Search Bins** — Live queries with smart folder auto-population
- **Metadata Index** — Fast search by rating, tags, media type, duration

## Project Templates

```rust
use saturn_project::{ProjectTemplate, SearchBin, SearchQuery};

let template = ProjectTemplate {
    name: "YouTube 1080p",
    description: "Optimized for YouTube uploads",
    settings: SequenceSettings {
        frame_rate: FrameRate::Fps30,
        resolution: (1920, 1080),
        color_space: ColorSpace::Rec709,
        ..Default::default()
    },
    tracks: vec![
        TrackTemplate { kind: TrackKind::Video, count: 3, name_prefix: "V" },
        TrackTemplate { kind: TrackKind::Audio, count: 2, name_prefix: "A" },
        TrackTemplate { kind: TrackKind::Subtitle, count: 1, name_prefix: "S" },
    ],
    bins: vec![
        BinTemplate { name: "Footage", query: SearchQuery::media_type(MediaType::Video) },
        BinTemplate { name: "Audio", query: SearchQuery::media_type(MediaType::Audio) },
        BinTemplate { name: "Graphics", query: SearchQuery::media_type(MediaType::Image) },
        BinTemplate { name: "Music", query: SearchQuery::tags(vec!["music"]).rating(4) },
    ],
};
```

## Search Bins

```rust
let search_bin = SearchBin::new("Best Takes")
    .query(SearchQuery::new()
        .rating(4..=5)
        .tag("interview")
        .duration(10.0..=60.0)
        .media_type(MediaType::Video))
    .live(true); // Auto-updates as media is added

// Subscribe to results
let results = search_bin.evaluate(&project.media_pool);
```

## Running Tests

```bash
cargo test -p saturn-project
```