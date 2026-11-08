# Media runtime initialization regression

**Date:** 2026-11-08. **Observed baseline:** `7c20b98`. **Correction:** `c817bc4`.

## Failure and diagnosis

The workspace gate run during agent-first plan integration stalled in
`crates/saturn-render/tests/audio_edits.rs`. The same four tests passed with
`--test-threads=1`, while a debugger-launched parallel run reproduced the stall.
The initial workspace run was terminated and failed; it is not passing evidence.

The native backtrace showed:

- PCM decoding loading the playback plugin through `uridecodebin`, waiting in
  `g_type_add_interface_static`.
- Master-bounce initialization entering `ges_init` and `gst_discoverer_new`,
  then waiting for that playback plugin.
- Other audio-preparation threads waiting for the same plugin.

PCM decoding called `gst::init()` outside the shared initialization cell. The
cell itself accepted different closures: frame extraction only initialized
GStreamer, hardware detection omitted the audio-plugin element, and rendering
initialized GES and all custom elements. A frame-first call could therefore
prevent full initialization permanently, independently of the concurrency race.

[Beat-grid CI](https://github.com/zvirox/easy-edits-pro/actions/runs/42462752102)
subsequently hit its 30-minute job limit. Its completed job log shows the same
four `audio_edits` tests still running until cancellation; licences and Python
harness jobs passed. No remote backtrace was captured, so the precise native
lock diagnosis comes from the local reproducer.

## Correction

`saturn_render::init_media_runtime()` owns a private process-wide `OnceLock`.
It completes GStreamer, GES and all four built-in element registrations before
any participating caller proceeds. PCM/frame decode, master bounce, export,
hardware discovery and desktop media import/probe/preview/synthetic paths use
the same function. Hardware discovery reports initialization failure as an
unavailable result instead of querying an uninitialized registry.

Initialization alone is serialized. Independent decoding remains concurrent;
the existing GES render gate is retained. New direct GStreamer pipeline or
discoverer callers in the application must use this boundary first. Decode-only
entry points now initialize the same GES runtime as rendering and return its
initialization errors rather than continuing with a partially ready runtime.

## Regression evidence

`crates/saturn-render/tests/runtime_init.rs` starts a fresh child process for
frame-first, audio-first, hardware-first and simultaneous entry. Each child
decodes synthetic PNG/WAV fixtures and prepares a real GES master bounce, checks
pixels/PCM and −14 LUFS output, and verifies all built-in element registrations.
The parent kills and reaps a child exceeding 60 seconds, so native deadlocks
produce a bounded test failure. Fixture creation does not initialize GStreamer.

- **Before the correction:** the new test failed deterministically in frame-first
  mode with `missing saturngraphicsblend`.
- **After the correction:** all four cold-start modes passed (8.40 seconds total);
  the existing four parallel audio-edit tests passed (4.93 seconds).
- Command: `cargo test -p saturn-render --test runtime_init --test audio_edits -- --nocapture`.

The [shared progress log](../AI_NATIVE_PLAN_CHECKLIST.md#progress-log) records
the final workspace/desktop and frontend gates. The cold-start matrix is native
regression evidence, not a proof of every possible GStreamer/plugin interleaving.
