# Model licences gate

No model weights are bundled with Easy Edits Pro today. **Before any model
is bundled, auto-downloaded by a feature, or wired into a commercial**

| Name | Source / URL | Upstream licence | Commercial OK? | Bundled by | Notes / restrictions |
|---|---|---|---|---|---|
| *(none yet)* | | | — | | |

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
