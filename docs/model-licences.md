# Model licences gate

No model weights are bundled with Easy Edits Pro today. **Before any model
is bundled, auto-downloaded by a feature, or wired into a commercial**

| Name | Source / URL | Upstream licence | Commercial OK? | Bundled by | Notes / restrictions |
|---|---|---|---|---|---|
| whisper.cpp (inference engine, built from source) | https://github.com/ggerganov/whisper.cpp | MIT | ✅ | No — compiled via `whisper-rs` (Unlicense bindings) | Licence text recorded in `licenses/whisper.cpp-MIT.txt`; NOTICE has the jfk.wav test-fixture entry |
| Whisper ggml `tiny.en` / `tiny` / `base.en` / `base` / `small.en` / `small` / `medium.en` / `medium` / `large-v3-turbo` | `https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-<name>.bin` (canonical upstream host) | MIT (OpenAI Whisper is MIT; ggml conversions upstream in the same repo) | ✅ | No — **fetch on first use** (plan-review §92), SHA-256-pinned in `crates/saturn-perception/src/models.rs` | Weights verified against upstream LFS SHA-256 at revision `5359861c739e955e79d9a303bcbc70fb988958b1` (checked 2026-11-06). Never bundled; offline machines place the file manually. |

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
