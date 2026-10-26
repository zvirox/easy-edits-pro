import assert from "node:assert/strict";
import test from "node:test";
import {
  handleTrimTransportCommand,
  isTrimTransportCommand,
  type TrimTransportContext,
} from "./trimTransports.ts";

const TICKS_PER_SECOND = 254_016_000_000;

function baseCtx(overrides: Partial<TrimTransportContext> = {}): TrimTransportContext {
  return {
    trimActive: true,
    baseCut: TICKS_PER_SECOND * 3,
    handleShuttle: () => {},
    cancelDynamic: () => {},
    seek: () => {},
    playhead: TICKS_PER_SECOND * 4,
    frameRate: { numerator: 30, denominator: 1 },
    notice: () => {},
    ...overrides,
  };
}

function withPrompt(answer: string | null, run: () => void) {
  const previous = (globalThis as { window?: unknown }).window;
  (globalThis as { window?: unknown }).window = { prompt: () => answer };
  try {
    run();
  } finally {
    (globalThis as { window?: unknown }).window = previous;
  }
}

test("only the dynamic-trim transports are claimed", () => {
  assert.ok(isTrimTransportCommand("trim.shuttle"));
  assert.ok(isTrimTransportCommand("trim.tick"));
  assert.ok(!isTrimTransportCommand("trim.forward"));
  assert.ok(!isTrimTransportCommand("trim.enter"));
});

test("transports refuse to run without an active trim session", () => {
  const notices: string[] = [];
  const ctx = baseCtx({ trimActive: false, notice: (m) => notices.push(m) });
  assert.ok(handleTrimTransportCommand("trim.shuttle", ctx));
  assert.ok(handleTrimTransportCommand("trim.tick", ctx));
  assert.ok(handleTrimTransportCommand("trim.playAround", ctx));
  assert.equal(notices.length, 3);
  // Cancel is safe without a session: it is an idempotent dismiss.
  let cancelled = 0;
  assert.ok(
    handleTrimTransportCommand("trim.cancelDynamic", baseCtx({
      cancelDynamic: () => {
        cancelled += 1;
      },
    })),
  );
  assert.equal(cancelled, 1);
});

test("shuttle maps direction onto the J/K/L keys", () => {
  const keys: string[] = [];
  const ctx = baseCtx({ handleShuttle: (k) => keys.push(k) });
  withPrompt("-1", () => {
    handleTrimTransportCommand("trim.shuttle", ctx);
  });
  withPrompt("0", () => {
    handleTrimTransportCommand("trim.shuttle", ctx);
  });
  withPrompt("1", () => {
    handleTrimTransportCommand("trim.shuttle", ctx);
  });
  assert.deepEqual(keys, ["j", "k", "l"]);
});

test("tick advances the playhead by frames at the project frame rate", () => {
  const frame = TICKS_PER_SECOND / 30;
  let sought: number | null = null;
  const ctx = baseCtx({
    playhead: frame * 10,
    seek: (ticks) => {
      sought = ticks;
    },
  });
  withPrompt("-4", () => {
    handleTrimTransportCommand("trim.tick", ctx);
  });
  assert.equal(sought, frame * 6);
  // Non-integer input is refused without seeking.
  sought = null;
  withPrompt("1.5", () => {
    handleTrimTransportCommand("trim.tick", ctx);
  });
  assert.equal(sought, null);
});

test("playAround rolls back two seconds from the cut and plays", () => {
  const keys: string[] = [];
  let sought: number | null = null;
  const ctx = baseCtx({
    handleShuttle: (k) => keys.push(k),
    seek: (ticks) => {
      sought = ticks;
    },
  });
  handleTrimTransportCommand("trim.playAround", ctx);
  assert.equal(sought, TICKS_PER_SECOND);
  assert.deepEqual(keys, ["l"]);
});
