import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

const manifest = readFileSync(
  resolve(import.meta.dirname, "..", "..", "src-tauri", "Cargo.toml"),
  "utf8",
);
const tauriConfiguration = JSON.parse(
  readFileSync(
    resolve(import.meta.dirname, "..", "..", "src-tauri", "tauri.conf.json"),
    "utf8",
  ),
);
const releaseBundleConfiguration = JSON.parse(
  readFileSync(
    resolve(
      import.meta.dirname,
      "..",
      "..",
      "src-tauri",
      "tauri.release-bundle.conf.json",
    ),
    "utf8",
  ),
);

function binBlock(name) {
  const blocks = manifest.split("[[bin]]").slice(1);
  const block = blocks.find((candidate) =>
    new RegExp(`^\\s*name\\s*=\\s*"${name}"\\s*$`, "m").test(candidate),
  );
  assert.ok(block, `missing explicit ${name} binary target`);
  return block;
}

test("production Cargo builds expose only the main executable by default", () => {
  assert.match(manifest, /^autobins\s*=\s*false$/m);
  assert.doesNotMatch(binBlock("formation-lap"), /required-features/);
  assert.match(
    binBlock("formation-lap-elevated-helper"),
    /required-features\s*=\s*\["elevated-helper"\]/,
  );
  for (const tool of ["generate-bindings", "validate-catalog"]) {
    assert.match(
      binBlock(tool),
      /required-features\s*=\s*\["development-tools"\]/,
    );
  }
});

test("test fixtures remain feature-gated outside production builds", () => {
  for (const fixture of [
    "formation-lap-process-fixture",
    "formation-lap-launcher-fixture",
    "formation-lap-stop-fixture",
  ]) {
    assert.match(
      binBlock(fixture),
      /required-features\s*=\s*\["process-fixtures"\]/,
    );
  }
});

test("the native updater plugin always receives a concrete fail-closed config", () => {
  assert.deepEqual(tauriConfiguration.plugins.updater, {
    endpoints: [],
    pubkey: "",
    windows: {
      installMode: "passive",
    },
  });
});

test("release bundles install the helper authorization manifest as a native resource", () => {
  assert.deepEqual(releaseBundleConfiguration.bundle.resources, {
    "release-identity/formation-lap-release-identity.json": "",
  });
});
