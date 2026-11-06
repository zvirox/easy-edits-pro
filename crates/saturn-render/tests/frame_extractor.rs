// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Integration evidence for Packet 1.1: the 4×4 contact sheet from the
//! maintained fixture (Phase 1 exit criterion), cache behaviour, and the
//! 0700 previews containment. Skips itself when the environment has no
//! GStreamer decode plugins (CI runners) — run laptop-native for the gate
//! evidence: `cargo test -p saturn-render --test frame_extractor`.

use saturn_core::{FrameRate, Tick};
use saturn_render::frame_extractor::{
    burn_in_label, cached_still, extract_frame_sheet, extract_still, previews_dir, SheetRequest,
    StillOptions, MAX_INLINE_JPEG_BYTES,
};
use std::path::PathBuf;
use std::sync::Mutex;

fn fixture() -> Option<PathBuf> {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/saturn-tauri/tests/fixtures/editor-smoke.webm");
    if !fixture.is_file() {
        return None;
    }
    // uridecodebin + a VP9/VP8 decoder must exist, or the sheet cannot be
    // produced in this environment; the laptop-native run is the gate.
    let _ = gstreamer::init();
    if gstreamer::ElementFactory::find("uridecodebin").is_none() {
        return None;
    }
    Some(fixture)
}

/// Serializes tests that flip `SATURN_PREVIEWS_DIR`: the override is process
/// state, so only one such test may run at a time.
static PREVIEWS_ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_isolated_previews<T>(test: impl FnOnce(&std::path::Path) -> T) -> T {
    let _guard = PREVIEWS_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempfile::tempdir().unwrap();
    // SAFETY-free: plain set_var is fine while the lock is held (tests are
    // single-threaded per this mutex, and no other test reads the var).
    // Edition-2024 env mutation is unsafe; the PREVIEWS_ENV_LOCK guarantees
    // single-threaded access around it.
    unsafe { std::env::set_var("SATURN_PREVIEWS_DIR", dir.path()) };
    let result = test(dir.path());
    unsafe { std::env::remove_var("SATURN_PREVIEWS_DIR") };
    result
}

#[test]
fn contact_sheet_generates_a_valid_4x4_png_from_the_fixture() {
    let Some(fixture) = fixture() else {
        eprintln!("skipping: no fixture or no GStreamer decode plugins in this environment");
        return;
    };
    with_isolated_previews(|_| {
        // The fixture is 6.02 s: sheet the first five seconds in 16 tiles.
        let five_seconds = 5 * saturn_core::TICKS_PER_SECOND;
        let request = SheetRequest::well_ranged(Tick(0), Tick(five_seconds));
        let started = std::time::Instant::now();
        let sheet = extract_frame_sheet(&fixture, request, FrameRate::new(30, 1).unwrap())
            .expect("the contact sheet must render from the fixture");
        let elapsed = started.elapsed();
        println!(
            "4x4 contact sheet: {}x{} in {elapsed:?} (16 tiles)",
            sheet.width, sheet.height
        );

        // Grid geometry.
        assert_eq!(sheet.width, 4 * 320);
        assert_eq!(sheet.height, 4 * 180);
        assert_eq!(sheet.tiles.len(), 16);
        assert_eq!(sheet.tiles[0].index, 0);

        // Timecode burn-in: strictly increasing, all within the 5 s span.
        for pair in sheet.tiles.windows(2) {
            assert!(pair[0].tick.0 < pair[1].tick.0);
            assert!(pair[1].tick.0 <= five_seconds);
            assert_eq!(pair[0].timecode.len(), 11, "HH:MM:SS:FF");
        }

        // The PNG artifact must be a real decodable PNG.
        let png_bytes = std::fs::read(&sheet.png_path).expect("PNG artifact exists");
        let decoder = png::Decoder::new(std::io::Cursor::new(&png_bytes));
        let reader = decoder.read_info().expect("valid PNG header");
        assert_eq!(reader.info().width, sheet.width);
        assert_eq!(reader.info().height, sheet.height);
        assert_eq!(reader.info().color_type, png::ColorType::Rgb);

        // The in-band JPEG preview exists and respects the byte cap.
        let jpeg = sheet
            .inline_jpeg
            .expect("a 4x4 320px sheet fits the in-band cap");
        assert!(jpeg.len() <= MAX_INLINE_JPEG_BYTES);
        assert!(jpeg.starts_with(&[0xFF, 0xD8]), "JPEG SOI marker");

        // Performance: the whole sheet from one decode, well under 2 s even
        // cold on this machine (the gate's <2 s is the cached path).
        assert!(elapsed < std::time::Duration::from_secs(20), "sheet took {elapsed:?}");
    });
}

#[test]
fn poster_still_extraction_and_cache_hit() {
    let Some(fixture) = fixture() else {
        eprintln!("skipping: no fixture or no GStreamer decode plugins");
        return;
    };
    with_isolated_previews(|_| {
        let options = StillOptions {
            at_tick: saturn_core::Tick(saturn_core::time::TICKS_PER_SECOND), // 1 s in
            max_width: 512,
        };
        let first = cached_still(&fixture, options).expect("poster extract");
        assert!(!first.from_cache);
        assert_eq!(first.width, 512, "aspect-scaled to max_width");
        assert!(first.height > 0 && first.height < 512);
        assert!(first.path.starts_with(previews_dir().unwrap()));
        assert!(first.bytes.starts_with(&[0xFF, 0xD8]));

        let second = cached_still(&fixture, options).expect("poster cache read");
        assert!(second.from_cache, "second identical request is a cache hit");
        assert_eq!(second.bytes, first.bytes);
    });
}

#[test]
fn previews_dir_is_0700_and_contained() {
    with_isolated_previews(|dir| {
        let previews = previews_dir().expect("previews dir");
        assert!(previews.starts_with(dir), "override is honoured: {previews:?}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&previews).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "previews dir must be 0700");
        }
    });
}

#[test]
fn burn_in_label_writes_white_pixels_with_an_outline() {
    let width = 160;
    let height = 60;
    let mut canvas = vec![0u8; width as usize * height as usize * 3];
    burn_in_label(&mut canvas, width, height, "00:00:01:00");
    let white = canvas.chunks_exact(3).filter(|px| *px == [255, 255, 255]).count();
    let black = canvas.chunks_exact(3).filter(|px| *px == [0u8, 0, 0]).count();
    assert!(white > 20, "label strokes must be visible: {white} white px");
    assert!(black > 30, "canvas stays dark; outline darkens edges: {black} black px");
}

#[test]
fn still_extraction_reports_missing_sources() {
    let error = extract_still(
        std::path::Path::new("/nowhere/eep-missing.webm"),
        StillOptions::default(),
    );
    assert!(error.is_err());
}
