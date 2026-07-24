import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const generator = join(
  repositoryRoot,
  "scripts",
  "release",
  "generate-release-metadata.mjs",
);
const verifier = join(
  repositoryRoot,
  "scripts",
  "release",
  "verify-release-artifacts.mjs",
);

test("generates official updater metadata and checksums for a prepared release", () => {
  const root = mkdtempSync(join(tmpdir(), "formation-lap-metadata-"));
  const directory = join(root, "release");
  mkdirSync(directory);
  const version = "1.0.0-beta.1";
  const tag = `v${version}`;
  const installer = `Formation-Lap_${version}_x64-setup.exe`;
  try {
    writeFileSync(join(directory, installer), "signed installer");
    writeFileSync(join(directory, `${installer}.sig`), "Zml4dHVyZQ==");
    writeFileSync(
      join(directory, `Formation-Lap_${version}.spdx.json`),
      JSON.stringify({
        spdxVersion: "SPDX-2.3",
        name: `Formation Lap ${version}`,
        packages: [{ name: "formation-lap", versionInfo: version }],
      }),
    );
    writeFileSync(
      join(directory, "THIRD-PARTY-LICENSES.json"),
      JSON.stringify({
        schemaVersion: 1,
        deniedLicenses: [],
        packages: [
          {
            ecosystem: "cargo",
            name: "serde",
            version: "1.0.228",
            license: "MIT OR Apache-2.0",
          },
        ],
      }),
    );
    writeFileSync(
      join(directory, "THIRD-PARTY-NOTICES.txt"),
      "Third-party notices",
    );
    writeFileSync(
      join(directory, "AUTHENTICODE.txt"),
      `formation-lap.exe: Valid\nformation-lap-elevated-helper.exe: Valid\n${installer}: Valid\n`,
    );
    const notesPath = join(root, "RELEASE_NOTES.md");
    writeFileSync(notesPath, "Signed Beta candidate.");

    const generated = spawnSync(
      process.execPath,
      [
        generator,
        "--directory",
        directory,
        "--version",
        version,
        "--tag",
        tag,
        "--notes",
        notesPath,
        "--published-at",
        "2026-07-24T12:00:00Z",
      ],
      { cwd: repositoryRoot, encoding: "utf8", shell: false },
    );
    assert.equal(generated.status, 0, generated.stderr || generated.stdout);
    const verified = spawnSync(
      process.execPath,
      [verifier, "--directory", directory, "--version", version, "--tag", tag],
      { cwd: repositoryRoot, encoding: "utf8", shell: false },
    );
    assert.equal(verified.status, 0, verified.stderr || verified.stdout);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
