import assert from "node:assert/strict";
import test from "node:test";
import {
  buildProjectPanelParams,
  formatPanelResult,
  PANEL_DOC_COMMANDS,
  PROJECT_PANEL_COMMANDS,
  type ProjectPanelCommand,
} from "./panelMenu.ts";

const TICKS_PER_SECOND = 254_016_000_000;

type PromptTarget = { prompt: (message?: string) => string | null };

const ctx = (overrides: Record<string, unknown> = {}) =>
  ({ selectedMediaId: 5, notes: "hello", ...overrides }) as Parameters<
    typeof buildProjectPanelParams
  >[1];

/** The builders use the native prompt idiom; stub `window` for node runs. */
function withPrompt(answer: string | null, run: () => void) {
  const target: PromptTarget = { prompt: () => answer };
  const previous = (globalThis as { window?: unknown }).window;
  (globalThis as { window?: unknown }).window = target;
  try {
    run();
  } finally {
    (globalThis as { window?: unknown }).window = previous;
  }
}

test("panel command list is unique and grouped under project.", () => {
  assert.equal(PROJECT_PANEL_COMMANDS.length, 18);
  assert.equal(new Set(PROJECT_PANEL_COMMANDS).size, 18);
  for (const id of PROJECT_PANEL_COMMANDS) assert.ok(id.startsWith("project."));
});

test("view and sort builders validate their input", () => {
  withPrompt("grid", () => {
    assert.deepEqual(buildProjectPanelParams("project.view.set", ctx()), {
      params: { view_mode: "grid" },
    });
  });
  withPrompt("spiral", () => {
    const bad = buildProjectPanelParams("project.view.set", ctx());
    assert.equal(bad?.cancel, true);
  });
  withPrompt("name desc", () => {
    assert.deepEqual(buildProjectPanelParams("project.sort", ctx()), {
      params: { key: "name", dir: "desc" },
    });
  });
  withPrompt("name sideways", () => {
    const bad = buildProjectPanelParams("project.sort", ctx());
    assert.equal(bad?.cancel, true);
  });
  // Prompt cancelled = abort with no params.
  withPrompt(null, () => {
    const result = buildProjectPanelParams("project.sort", ctx());
    assert.deepEqual(result, { cancel: true });
  });
});

test("column and marks builders parse engine parameter shapes", () => {
  withPrompt("name, duration", () => {
    assert.deepEqual(buildProjectPanelParams("project.columns.set", ctx()), {
      params: {
        columns: [
          { id: "name", visible: true },
          { id: "duration", visible: true },
        ],
      },
    });
  });
  withPrompt("name 240", () => {
    assert.deepEqual(buildProjectPanelParams("project.columns.resize", ctx()), {
      params: { id: "name", width: 240 },
    });
  });
  withPrompt("0 4.5", () => {
    const result = buildProjectPanelParams("project.setMarks", ctx());
    assert.deepEqual(result, {
      params: {
        media_id: 5,
        source_in: 0,
        source_out: Math.round(4.5 * TICKS_PER_SECOND),
      },
    });
  });
  const noSelection = buildProjectPanelParams("project.setMarks", ctx({ selectedMediaId: null }));
  assert.equal(noSelection?.cancel, true);
});

test("preset builders separate save (update) from saveAs (named)", () => {
  withPrompt("", () => {
    // Save with an empty prompt updates the current preset: params {}.
    assert.deepEqual(buildProjectPanelParams("project.viewPreset.save", ctx()), {
      params: {},
    });
    // Save As demands a name.
    const bad = buildProjectPanelParams("project.viewPreset.saveAs", ctx());
    assert.equal(bad?.cancel, true);
  });
  withPrompt("Night", () => {
    assert.deepEqual(buildProjectPanelParams("project.viewPreset.saveAs", ctx()), {
      params: { name: "Night" },
    });
  });
});

test("formatters summarize engine results for the pool notice", () => {
  assert.equal(
    formatPanelResult("project.view.get", {
      view: { view_mode: "list", sort: { key: "kind", dir: "desc" }, columns: [1, 2, 3] },
    }),
    "Panel view: list, sorted by kind desc, 3 columns"
  );
  assert.equal(
    formatPanelResult("project.items", { media: [{}, {}], bins: [{}] }),
    "2 media item(s), 1 bin(s) in the project panel"
  );
  assert.equal(formatPanelResult("project.notes", { notes: "" }), "Project notes are empty");
  assert.equal(formatPanelResult("project.notes", { notes: "Call sheet" }), "Project notes: Call sheet");
  assert.equal(
    formatPanelResult("project.viewPreset.delete", { deleted: "Night" }),
    'View preset "Night" deleted'
  );
  assert.equal(
    formatPanelResult("project.sort", { sort: { key: "kind", dir: "asc" } }),
    "Panel sorted by kind asc"
  );
  assert.equal(
    formatPanelResult("project.columns.resize", { column: { id: "name", width: 96 } }),
    'Column "name" resized'
  );
  assert.equal(formatPanelResult("project.zzz", {}), null);
});

test("document-mutating panel commands are a narrow, marked set", () => {
  assert.deepEqual(PANEL_DOC_COMMANDS, ["project.setNotes", "project.setMarks"]);
  for (const id of PANEL_DOC_COMMANDS) {
    assert.ok((PROJECT_PANEL_COMMANDS as readonly ProjectPanelCommand[]).includes(id));
  }
});
