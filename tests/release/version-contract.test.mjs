import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
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
const verifier = join(
  repositoryRoot,
  "scripts",
  "release",
  "verify-release-version.mjs",
);

function createVersionFixture(versions) {
  const directory = mkdtempSync(join(tmpdir(), "formation-lap-version-"));
  mkdirSync(join(directory, "src-tauri"));
  writeFileSync(
    join(directory, "package.json"),
    JSON.stringify({ name: "formation-lap", version: versions.package }),
  );
  writeFileSync(
    join(directory, "src-tauri", "Cargo.toml"),
    `[package]\nname = "formation-lap"\nversion = "${versions.cargo}"\n`,
  );
  writeFileSync(
    join(directory, "src-tauri", "Cargo.lock"),
    `version = 4\n\n[[package]]\nname = "formation-lap"\nversion = "${versions.lock}"\n`,
  );
  writeFileSync(
    join(directory, "src-tauri", "tauri.conf.json"),
    JSON.stringify({ version: versions.tauri }),
  );
  return directory;
}

function verify(directory, tag = "v1.0.0") {
  return spawnSync(
    process.execPath,
    [verifier, "--root", directory, "--tag", tag],
    { cwd: repositoryRoot, encoding: "utf8", shell: false },
  );
}

test("accepts one synchronized SemVer and matching release tag", () => {
  const directory = createVersionFixture({
    package: "1.0.0",
    cargo: "1.0.0",
    lock: "1.0.0",
    tauri: "1.0.0",
  });
  try {
    const result = verify(directory);
    assert.equal(result.status, 0, result.stderr || result.stdout);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("rejects drift between package, Cargo, lockfile, and Tauri versions", () => {
  const directory = createVersionFixture({
    package: "1.0.0",
    cargo: "1.0.0",
    lock: "0.9.0",
    tauri: "1.0.0",
  });
  try {
    const result = verify(directory);
    assert.notEqual(result.status, 0);
    assert.match(`${result.stderr}${result.stdout}`, /version drift/i);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("rejects a tag that does not identify the synchronized version", () => {
  const directory = createVersionFixture({
    package: "1.0.0-beta.1",
    cargo: "1.0.0-beta.1",
    lock: "1.0.0-beta.1",
    tauri: "1.0.0-beta.1",
  });
  try {
    const result = verify(directory, "v1.0.0");
    assert.notEqual(result.status, 0);
    assert.match(`${result.stderr}${result.stdout}`, /tag/i);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("repository is prepared as the disclosed 0.9.0-preview.6 candidate", () => {
  const version = "0.9.0-preview.6";
  const result = verify(repositoryRoot, `v${version}`);
  assert.equal(result.status, 0, result.stderr || result.stdout);

  const notes = readFileSync(
    join(repositoryRoot, "docs", "releases", `${version}.md`),
    "utf8",
  );
  const [heading, firstParagraph] = notes.trim().split(/\r?\n\r?\n/, 2);
  assert.equal(
    heading,
    "# Formation Lap 0.9.0-preview.6 — unsigned technical preview",
  );
  for (const disclosure of [
    /unsigned/i,
    /Authenticode/i,
    /SmartScreen/i,
    /unknown publisher/i,
    /not a Stable/i,
  ]) {
    assert.match(firstParagraph, disclosure);
  }

  const readme = readFileSync(join(repositoryRoot, "README.md"), "utf8");
  assert.match(readme, /\$version = "0\.9\.0-preview\.6"/);
});
