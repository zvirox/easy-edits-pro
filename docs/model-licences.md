# Model licences gate

No model weights are bundled with Easy Edits Pro today. **Before any model
is bundled, auto-downloaded by a feature, or wired into a commercial**

| Name | Source / URL | Upstream licence | Commercial OK? | Bundled by | Notes / restrictions |
|---|---|---|---|---|---|
| whisper.cpp (inference engine, built from source) | https://github.com/ggerganov/whisper.cpp | MIT | ✅ | No — compiled via `whisper-rs` (Unlicense bindings) | Licence text recorded in `licenses/whisper.cpp-MIT.txt`; NOTICE has the jfk.wav test-fixture entry |
| Whisper ggml `tiny.en` / `tiny` / `base.en` / `base` / `small.en` / `small` / `medium.en` / `medium` / `large-v3-turbo` | `https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-<name>.bin` (canonical upstream host) | MIT (OpenAI Whisper is MIT; ggml conversions upstream in the same repo) | ✅ | No — **fetch on first use** (plan-review §92), SHA-256-pinned in `crates/saturn-perception/src/models.rs` | Weights verified against upstream LFS SHA-256 at revision `5359861c739e955e79d9a303bcbc70fb988958b1` (checked 2026-11-06). Never bundled; offline machines place the file manually. |
| ONNX Runtime (inference engine, prebuilt binary via `ort`/`ort-sys` `download-binaries`) | Build-time fetch from pyke's CDN (`https://cdn.pyke.cc/onnxruntime/`, linked from https://onnxruntime.ai / the `ort` crate docs) — MIT | ✅ (MIT; copyright transfers recorded in ONNX Runtime's THIRD-PARTY notices upstream) | No — fetched by the build (never committed to the repo) | `ort = 2.0.0-rc.15` (MIT OR Apache-2.0) in `crates/saturn-perception/Cargo.toml`; runtime version pinned by ort-sys (ONNX Runtime 1.31 series). CPU execution provider only. |
| SigLIP `base-patch16-224` ONNX exports (vision + text encoders) | `https://huggingface.co/Xenova/siglip-base-patch16-224/resolve/4649052661e53c7000355844105f8a1792088239/onnx/{vision,text}_model_quantized.onnx` (canonical upstream host; transformers.js org export of `google/siglip-base-patch16-224`) | Apache-2.0 (inherited from the upstream Google model card at revision `7fd15f0689c79d79e38b1c2e2e2370a7bf2761ed` — the export repo tags the same base model) | ✅ | No — **fetch on first use** (plan-review §92), SHA-256-pinned in `crates/saturn-perception/src/siglip_model.rs` (`vision_model_quantized.onnx` 99,499,129 B `ef14a954…`, `text_model_quantized.onnx` 111,475,220 B `ad0329b1…`, `tokenizer.json` 2,398,744 B `4a17c975…`; verified 2026-11-06). Powers `media_semantic_query`. Never bundled. |

## Rules (binding for every packet — see AGENTS.md provenance rules)

1. **Add a row here in the same change** that introduces the model.
2. Non-commercial (CC-BY-NC, "research only", GPL-only weights) and
   unlicensed weights are **never** accepted without the maintainer's
   written approval recorded next to the row. A commercial feature must not
   depend on a weight that cannot be licensed commercially.
3. Verify the licence from the model's own source (HF card, repo LICENSE) at
   adoption time — treat secondhand claims as unconfirmed.
4. Record the exact weight revision/commit hash, not just the model name.
5. If the model ships as a separate optional download ("fetch on first
   run"), still fill this table before wiring the feature; record the
   endpoint the download comes from (it must be the upstream's canonical
   host, not a mirror of unknown provenance).
6. Re-check on licence changes: if upstream relabels a weight
   non-commercial, the EEP feature depending on it must be disabled in
   commercial builds until replaced.
