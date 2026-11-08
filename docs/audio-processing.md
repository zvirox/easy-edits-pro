# Offline audio processing (Packets 4.2 and 5.1)

The `audio_process` MCP tool and desktop agent bridge prepare an edit outside
the project lock, then apply a single undoable Rust-core transaction. Save the
project first. Generated 48 kHz float WAVs live in the sibling
`<project-stem>.saturn-audio/` directory. Keep that directory with the project.
Failed preparation or rejected application removes the newly generated files;
successful edits retain them, including across undo/redo and save/reopen.
Unused derived files are not garbage-collected automatically.

The tool compares a digest of the project snapshot before applying its result.
If the project changed, it returns an error and the caller must rerun analysis.
The low-level catalogue commands accept prepared payloads (`audio`, `inserts`,
or automation `points`, plus `expected_digest`); call `audio_process` to obtain
and apply these rather than inventing payloads for `command_run`/`command_batch`.
The agent bridge exposes all operations. J/L split edits also have a desktop
menu action, which prepares the raw `timeline.asymmetric_trim` request.

## Create a J/L split edit

```json
{"operation":"asymmetric_trim","track_id":1,"clip_index":1,
 "video_delta":0,"audio_delta":-127008000000,"fade_ticks":10160640000}
```

Select the incoming clip of an adjacent video pair. This example leads picture
with sound by 0.5 seconds (J-cut), with a 40 ms centered constant-power crossfade.
Both offsets are relative to the original cut. A later sound cut makes an L-cut.
Omitting `fade_ticks` uses 40 ms; zero gives a hard sound cut. The pair's outer
edges stay fixed while the picture cut rolls. The menu is **Sequence > Trim >
Create J/L Split Edit...** and accepts offsets/fade duration in seconds.

The command detaches both clips' sound into one editable stereo 48 kHz float
WAV on a new audio track. Original picture clips have `audio_enabled: false`,
so both native and browser playback omit their audio. Clip gain is baked into
the WAV; track gain/mute/solo/group membership is retained. The fade itself is
baked: edit the resulting clip normally, or undo and rerun to change the cut
or fade. One undo restores the entire pair and removes the new track from the
document; its file stays available for redo. Moving the saved project folder
with its `.saturn-audio` sibling retains the derived media.

The pair must span at most 120 seconds, with mono/stereo source audio, known
durations and enough source handles. Locked tracks/groups, overlaps, gaps,
retiming, linked clips, multicam, compounds, disabled/detached sound, effects,
keyframes, tracking, transitions, audio processing, automation, pan and bus
routing are rejected. Static clip colour adjustments are retained. Fade
duration is at most 5 seconds and at least two audio samples when nonzero;
leave at least one sample on either side of the fade. Audio boundaries are
quantized to 48 kHz, while picture timing remains in project ticks. Constant
power describes the fade weights; correlated signals can still sum louder.

## Align two microphones

```json
{"operation":"phase_align","ref_track_id":2,"target_track_id":3}
```

Two distinct, unlocked audio tracks must each contain one enabled, unretimed
audio-media clip, without transitions or compound clips. Their timeline ranges
must overlap. GCC-PHAT measures a constant delay on up to 65,536 mono samples
of that overlap (raw sources, before track effects). The search is bounded to
100 ms or one quarter of the analysis window, whichever is smaller. Silence,
weak agreement and ambiguous peaks return errors. Telemetry includes measured
lag, polarity and post-correction correlation.

A 65-tap fractional-delay filter replaces the target's source with stereo PCM;
its timeline placement and track processing remain editable. Missing edge
samples are zero-filled. This corrects a constant offset, not clock drift.
Source ranges and the target replacement are limited to 120 seconds.

## Fill internal gaps

```json
{"operation":"room_tone","track_id":2,"room_tone_media_id":9,
 "profile_start_tick":0,"profile_end_tick":254016000000,
 "min_gap_ms":100,"crossfade_ms":30}
```

Choose a known speech-free noise bed from project audio media. The profile is
bounded to 10 seconds and must supply at least 1,024 decoded samples. Spectral
power averaging and random-phase overlap-add synthesis produce fresh mono room
tone with the profile's AC RMS and spectral color. Silence/DC profiles fail.
The tool inserts media into internal, empty gaps; it does not extend the track
or overwrite dialogue. Maximum 256 gaps and 120 seconds total generated audio.

**Current `crossfade_ms` behavior:** equal-power fade-in/out *inside* each gap,
clamped to half its duration. This is not a crossfade with adjoining dialogue
and does not remove discontinuities already present at dialogue cuts. Automatic
speech-free profile selection and dialogue-edge crossfades remain follow-ups.

## Duck music

```json
{"operation":"duck","dialogue_track_id":2,"music_track_id":3,
 "duck_db":-12,"lookahead_ms":40,"hold_ms":150,"release_ms":300,
 "threshold_dbfs":-38}
```

Ten-millisecond RMS windows on raw dialogue source audio key the music volume
curve. This is energy detection, not speech recognition: noise or music on the
designated dialogue track can also trigger it. Analysis ignores dialogue track
gain, mute and effects. The sequence is limited to 120 seconds; the selected
audio tracks use the same enabled/unretimed/non-compound/no-transition rules.
Both music automation lanes must be empty because interpolation and playback
mode are shared by volume and pan. Existing static music gain is included.

Activity regions are merged with hold/release to avoid pumping on short pauses.
The attack is up to 20 ms within the lookahead; zero lookahead uses a one-sample
step. The resulting linear volume curve is persisted in `Read` mode and follows
the existing preview/export automation path.

## Normalize the master

```json
{"operation":"normalize_master","standard":"streaming","true_peak_dbtp":-1.0}
```

The GES export mix is rendered to lossless stereo float WAV, measured with
K-weighted dual-gated integrated loudness, gain-adjusted and limited with a
stereo-linked 5 ms lookahead / 100 ms release. Four-times, 65-tap windowed-sinc
reconstruction measures inter-sample peaks instead of the old sample-peak-plus-5%
approximation. Residual peak correction and measured gain iterations must meet
both targets within eight passes or the operation fails.

Targets: `streaming` -14 LUFS, `ebu_r128` -23 LUFS, `atsc_a85` -24 LUFS;
requested peak ceiling -12 through -0.1 dBTP. The full sequence must span
0.4–120 seconds and all tracks must be unlocked. Telemetry reports before/after
loudness, reconstructed peak and pass count. This measures the GES export graph,
including its existing rendering limitations; browser playback is approximate.

**Undoable bounce:** original tracks are retained, muted and unsoloed; a new
audible master track contains the normalized mix. Muted video tracks retain
their pictures in GES. Further source edits require a new bounce. A second
bounce excludes muted originals. Lossy delivery encoding can change true peak:
remeasure the encoded delivery file when its peak ceiling matters.

## Deferred cinema stems

Dialogue/music/ambience/SFX separation remains pending a verified, license-cleared
model exposing those outputs. The proposed music-separation model produces
vocals/drums/bass/other; those outputs are not relabelled as cinema stems.

## Verification

- `saturn-core/tests/asymmetric_trim.rs`: signed J/L timing, source mapping,
  locks/handles/overflow rejection, one-step history, stale rejection and v15 migration.
- `saturn-render/tests/asymmetric_trim.rs`: real AV fixtures, crossfade power,
  rendered picture-cut timing, export vs bounce-only audio below -90 dBFS,
  audible doubled-source negative control, persistence and failure cleanup.
- Browser scheduler tests keep detached picture visible, omit source audio and
  retain legacy attached sound. E2E exercises the menu's request/error contract.

- `saturn-audio-dsp/tests/phase_align.rs`: independently generated 1.7 ms delay,
  >45 dB alignment-residual improvement, inverted polarity, channel isolation,
  ambiguity rejection and zero padding.
- `saturn-audio-dsp/tests/audio_processing.rs`: duck timing and zero-lookahead
  step, hold/release, linked limiting, room-tone RMS/spectral color and fades.
- `saturn-render/tests/audio_master.rs`: loudness targets, limiting-required
  inter-sample-peak fixture, independent steady-state reference and external
  FFmpeg EBU meter when installed (explicit skip otherwise).
- `saturn-render/tests/audio_edits.rs`: real WAV decoding and GES lossless mix,
  atomic application, history, persistence, stale-result cleanup, failed-gap
  cleanup, symlink rejection and repeat-bounce mute behavior.
