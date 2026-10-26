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
  assert.equal(PROJECT_PANEL_COMMANDS.length, 35);
  assert.equal(new Set(PROJECT_PANEL_COMMANDS).size, 35);
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
  assert.deepEqual(PANEL_DOC_COMMANDS, ["project.setNotes", "project.setMarks", "project.remove"]);
  for (const id of PANEL_DOC_COMMANDS) {
    assert.ok((PROJECT_PANEL_COMMANDS as readonly ProjectPanelCommand[]).includes(id));
  }
});

test("freeform builders validate the selection and prompt-driven inputs", () => {
  const withPrompt = (answer: string | null, run: () => void) => {
    const previous = (globalThis as { window?: unknown }).window;
    (globalThis as { window?: unknown }).window = { prompt: () => answer };
    try {
      run();
    } finally {
      (globalThis as { window?: unknown }).window = previous;
    }
  };
  // Selection-gated builders refuse to run without panel items.
  assert.equal(
    buildProjectPanelParams("project.remove", ctx({ selectedMediaIds: [] }))?.cancel,
    true
  );
  assert.deepEqual(buildProjectPanelParams("project.remove", ctx({ selectedMediaIds: [3, 5] })), {
    params: { media_ids: [3, 5] },
  });
  assert.equal(
    buildProjectPanelParams("project.freeform.stack", ctx({ selectedMediaIds: [] }))?.cancel,
    true
  );
  assert.deepEqual(
    buildProjectPanelParams("project.freeform.unstack", ctx({ selectedMediaIds: [7] })),
    { params: { media_ids: [7] } }
  );
  // Prompt-driven freeform commands parse freeform movement and scale.
  withPrompt("40 -8", () => {
    assert.deepEqual(
      buildProjectPanelParams("project.freeform.move", ctx({ selectedMediaIds: [1] })),
      { params: { media_ids: [1], dx: 40, dy: -8 } }
    );
  });
  withPrompt(null, () => {
    assert.equal(
      buildProjectPanelParams("project.freeform.move", ctx({ selectedMediaIds: [1] }))?.cancel,
      true
    );
  });
  withPrompt("0.5", () => {
    assert.deepEqual(
      buildProjectPanelParams("project.freeform.resize", ctx({ selectedMediaIds: [1, 2] })),
      { params: { media_ids: [1, 2], scale: 0.5 } }
    );
  });
  withPrompt("80", () => {
    assert.deepEqual(buildProjectPanelParams("project.freeform.options", ctx()), {
      params: { grid: 80 },
    });
  });
  withPrompt("", () => {
    assert.equal(
      buildProjectPanelParams("project.freeform.saveArrangement", ctx())?.cancel,
      true
    );
  });
  withPrompt("Wide board", () => {
    assert.deepEqual(buildProjectPanelParams("project.freeform.saveArrangement", ctx()), {
      params: { name: "Wide board" },
    });
  });
  withPrompt("Wide board", () => {
    assert.deepEqual(buildProjectPanelParams("project.freeform.deleteArrangement", ctx()), {
      params: { name: "Wide board" },
    });
  });
});

test("search-bin builders split the name from the query text", () => {
  const withPrompt = (answer: string | null, run: () => void) => {
    const previous = (globalThis as { window?: unknown }).window;
    (globalThis as { window?: unknown }).window = { prompt: () => answer };
    try {
      run();
    } finally {
      (globalThis as { window?: unknown }).window = previous;
    }
  };
  withPrompt("INT ship", () => {
    assert.deepEqual(buildProjectPanelParams("project.searchBin.set", ctx()), {
      params: { name: "INT", query: "ship" },
    });
  });
  withPrompt("greedy-name-only", () => {
    assert.equal(
      buildProjectPanelParams("project.searchBin.set", ctx())?.cancel,
      true
    );
  });
  withPrompt("INT", () => {
    assert.deepEqual(buildProjectPanelParams("project.searchBin.items", ctx()), {
      params: { name: "INT" },
    });
  });
  withPrompt("INT", () => {
    assert.deepEqual(buildProjectPanelParams("project.searchBin.delete", ctx()), {
      params: { name: "INT" },
    });
  });
});

test("formatters summarize the freeform and search-bin results", () => {
  assert.equal(formatPanelResult("project.freeform.layout", { freeform: true }), "Freeform layout on");
  assert.equal(formatPanelResult("project.freeform.layout", { freeform: false }), "Freeform layout off (list view)");
  assert.equal(formatPanelResult("project.freeform.move", { moved: 3 }), "3 clip card(s) moved");
  assert.equal(formatPanelResult("project.freeform.reset", { cleared: 4 }), "Freeform layout reset (4 card position(s) cleared)");
  assert.equal(formatPanelResult("project.freeform.unstack", { updated: 9 }), "9 clip card(s) unstacked onto the grid");
  assert.equal(
    formatPanelResult("project.freeform.arrangements", { arrangements: ["A", "B"] }),
    "2 arrangement(s): A, B"
  );
  assert.equal(
    formatPanelResult("project.searchBin.items", { bin: "INT", matches: [1, 2, 3] }),
    'Search bin "INT" matched 3 media item(s)'
  );
});
