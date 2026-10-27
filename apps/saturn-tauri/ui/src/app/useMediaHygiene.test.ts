import assert from "node:assert/strict";
import test from "node:test";
import { shouldAttachPreparedProxy, videoAssetsMissingProxy } from "./useMediaHygiene.ts";

// E.4 seam: Create Proxies vs attached-but-possibly-stale proxy paths. The
// engine contract (pinned in tests/media.rs) is that attach validates the
// file exists; these predicates pin the UI-side "which assets to serve" and
// "which job results change state" decisions.

const pool = [
  { id: 1, kind: "video", path: "/a.mp4" },
  { id: 2, kind: "video", path: "/b.mp4", proxy_path: "/cache/b.webm" },
  { id: 3, kind: "video", path: "/c.mp4", proxy_path: null },
  { id: 4, kind: "audio", path: "/d.wav" },
  { id: 5, kind: "image", path: "/e.png" },
];

test("videoAssetsMissingProxy serves exactly the video assets without a proxy", () => {
  const pending = videoAssetsMissingProxy(pool);
  assert.deepEqual(
    pending.map((asset) => asset.id),
    [1, 3],
    "an attached proxy (even a stale path) and non-video assets are skipped",
  );
});

test("shouldAttachPreparedProxy rejects WebM self-paths and revalidations", () => {
  const asset = { path: "/a.mp4", proxy_path: "/cache/old.webm" };
  // WebM sources decode natively; the job returns the source path itself.
  assert.equal(shouldAttachPreparedProxy(asset, "/a.mp4"), false);
  // The prepared path matches what is already attached: no-op revalidate.
  assert.equal(shouldAttachPreparedProxy(asset, "/cache/old.webm"), false);
  assert.equal(shouldAttachPreparedProxy(asset, null), false);
  assert.equal(shouldAttachPreparedProxy(asset, undefined), false);
  assert.equal(shouldAttachPreparedProxy(asset, ""), false);
});

test("shouldAttachPreparedProxy accepts a genuinely new proxy path", () => {
  assert.equal(
    shouldAttachPreparedProxy({ path: "/a.mp4" }, "/cache/a.webm"),
    true,
  );
  // An asset that already carries a proxy can still be re-pointed at a
  // freshly generated one.
  assert.equal(
    shouldAttachPreparedProxy({ path: "/a.mp4", proxy_path: "/cache/old.webm" }, "/cache/a.webm"),
    true,
  );
});
