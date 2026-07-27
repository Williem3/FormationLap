import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const bootstrapper = join(
  repositoryRoot,
  "scripts",
  "release",
  "manage-release-identity-resource.mjs",
);

function manage(manifest, action) {
  return spawnSync(
    process.execPath,
    [bootstrapper, "--manifest", manifest, "--action", action],
    { cwd: repositoryRoot, encoding: "utf8", shell: false },
  );
}

test("prepares and clears one exact non-runtime release identity resource", () => {
  const root = mkdtempSync(join(tmpdir(), "formation-lap-identity-bootstrap-"));
  const manifest = join(
    root,
    "release-identity",
    "formation-lap-release-identity.json",
  );
  try {
    const prepared = manage(manifest, "prepare");
    assert.equal(prepared.status, 0, prepared.stderr || prepared.stdout);
    assert.deepEqual(JSON.parse(readFileSync(manifest, "utf8")), {
      schemaVersion: 0,
      bootstrapOnly: true,
    });

    const duplicate = manage(manifest, "prepare");
    assert.notEqual(duplicate.status, 0);
    assert.match(`${duplicate.stderr}${duplicate.stdout}`, /already exists/i);

    const cleared = manage(manifest, "clear");
    assert.equal(cleared.status, 0, cleared.stderr || cleared.stdout);
    assert.equal(existsSync(manifest), false);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("refuses to clear a changed or real release identity manifest", () => {
  const root = mkdtempSync(join(tmpdir(), "formation-lap-identity-bootstrap-"));
  const manifest = join(
    root,
    "release-identity",
    "formation-lap-release-identity.json",
  );
  try {
    mkdirSync(join(root, "release-identity"));
    writeFileSync(manifest, '{"schemaVersion":1}\n');

    const cleared = manage(manifest, "clear");
    assert.notEqual(cleared.status, 0);
    assert.match(`${cleared.stderr}${cleared.stdout}`, /does not match/i);
    assert.equal(existsSync(manifest), true);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
