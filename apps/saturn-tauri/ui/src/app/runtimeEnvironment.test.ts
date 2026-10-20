import assert from "node:assert/strict";
import test from "node:test";
import {
  capabilityAttributes,
  isSlowEnvironment,
  type RuntimeEnvironment,
} from "./runtimeEnvironment.ts";

function environment(overrides: Partial<RuntimeEnvironment> = {}): RuntimeEnvironment {
  return {
    appVersion: "0.1.0",
    osName: "Linux",
    kernel: "6.0",
    sessionType: "wayland",
    cpuModel: "CPU",
    ramMb: 16_384,
    gpuVendor: "Intel",
    gpuModel: "Iris Xe",
    gpuClass: "integrated",
    gpuDriver: "i915",
    gpuDevices: [],
    dmabufRendererDisabled: false,
    dmabufPolicy: "enabled-default",
    webkitgtkVersion: "2.54.1",
    ...overrides,
  };
}

test("integrated GPUs with DMA-BUF are not marked slow", () => {
  const env = environment();
  assert.equal(isSlowEnvironment(env), false);
  assert.equal(capabilityAttributes(env)["data-webkit-gtk-slow"], "false");
  assert.equal(capabilityAttributes(env)["data-dmabuf"], "enabled");
});

test("a disabled DMA-BUF renderer forces the lite fallback", () => {
  const env = environment({ dmabufRendererDisabled: true, dmabufPolicy: "disabled-env" });
  assert.equal(isSlowEnvironment(env), true);
  assert.equal(capabilityAttributes(env)["data-webkit-gtk-slow"], "true");
  assert.equal(capabilityAttributes(env)["data-dmabuf-policy"], "disabled-env");
});

test("an unknown GPU forces the lite fallback", () => {
  const env = environment({ gpuClass: "unknown", gpuVendor: "unknown" });
  assert.equal(isSlowEnvironment(env), true);
});

test("a missing probe forces the lite fallback", () => {
  assert.equal(isSlowEnvironment(null), true);
  assert.deepEqual(capabilityAttributes(null), {
    "data-webkit-gtk-slow": "true",
    "data-runtime-probe": "unavailable",
  });
});
