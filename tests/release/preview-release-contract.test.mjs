import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const generator = join(
  repositoryRoot,
  "scripts",
  "release",
  "generate-preview-metadata.mjs",
);
const verifier = join(
  repositoryRoot,
  "scripts",
  "release",
  "verify-preview-artifacts.mjs",
);
const version = "0.9.0-preview.8";
const tag = `v${version}`;
const installerName = `Formation-Lap_${version}_x64-setup.exe`;

function createPreparedPreview() {
  const root = mkdtempSync(join(tmpdir(), "formation-lap-preview-"));
  const directory = join(root, "release");
  mkdirSync(directory);
  writeFileSync(join(directory, installerName), "unsigned installer fixture");
  writeFileSync(
    join(directory, `${installerName}.sig`),
    "Zml4dHVyZS10YXVyaS1zaWduYXR1cmU=",
  );
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
    "Formation Lap third-party dependency notices\n",
  );
  writeFileSync(
    join(directory, "UNSIGNED-PREVIEW.txt"),
    [
      "Formation Lap unsigned technical preview",
      "This v0.x installer, application, and one-shot elevated helper do not have Windows Authenticode publisher signatures.",
      "Windows may show SmartScreen and Unknown publisher warnings.",
      "This preview is not a Stable Formation Lap release.",
      "Verify the installer against SHA256SUMS.txt.",
      "In-app updates still require the separate Tauri updater signature.",
    ].join("\n"),
  );
  const notes = join(root, "RELEASE_NOTES.md");
  writeFileSync(
    notes,
    "# Unsigned technical preview\n\nThis build is not Authenticode-signed.",
  );
  return { directory, notes, root };
}

function generate(directory, notes, overrideTag = tag) {
  return spawnSync(
    process.execPath,
    [
      generator,
      "--directory",
      directory,
      "--version",
      version,
      "--tag",
      overrideTag,
      "--notes",
      notes,
      "--published-at",
      "2026-07-24T12:00:00Z",
    ],
    { cwd: repositoryRoot, encoding: "utf8", shell: false },
  );
}

function verify(directory) {
  return spawnSync(
    process.execPath,
    [verifier, "--directory", directory, "--version", version, "--tag", tag],
    { cwd: repositoryRoot, encoding: "utf8", shell: false },
  );
}

test("generates and accepts one exact unsigned v0.x preview set", () => {
  const fixture = createPreparedPreview();
  try {
    const generated = generate(fixture.directory, fixture.notes);
    assert.equal(generated.status, 0, generated.stderr || generated.stdout);
    const verified = verify(fixture.directory);
    assert.equal(verified.status, 0, verified.stderr || verified.stdout);
    const metadata = JSON.parse(
      readFileSync(join(fixture.directory, "latest.json"), "utf8"),
    );
    assert.equal(metadata.version, version);
    assert.match(metadata.notes, /not Authenticode-signed/);
    assert.equal(
      metadata.platforms["windows-x86_64"].url,
      `https://github.com/Williem3/FormationLap/releases/download/${tag}/${installerName}`,
    );
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test("rejects a preview without its unsigned-binary disclosure", () => {
  const fixture = createPreparedPreview();
  try {
    const generated = generate(fixture.directory, fixture.notes);
    assert.equal(generated.status, 0, generated.stderr || generated.stdout);
    unlinkSync(join(fixture.directory, "UNSIGNED-PREVIEW.txt"));
    const verified = verify(fixture.directory);
    assert.notEqual(verified.status, 0);
    assert.match(`${verified.stderr}${verified.stdout}`, /UNSIGNED-PREVIEW/i);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test("rejects Authenticode evidence that could misrepresent a preview", () => {
  const fixture = createPreparedPreview();
  try {
    const generated = generate(fixture.directory, fixture.notes);
    assert.equal(generated.status, 0, generated.stderr || generated.stdout);
    writeFileSync(
      join(fixture.directory, "AUTHENTICODE.txt"),
      "installer: Valid",
    );
    const verified = verify(fixture.directory);
    assert.notEqual(verified.status, 0);
    assert.match(`${verified.stderr}${verified.stdout}`, /preview directory/i);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test("rejects a preview installer that no longer matches its checksum", () => {
  const fixture = createPreparedPreview();
  try {
    const generated = generate(fixture.directory, fixture.notes);
    assert.equal(generated.status, 0, generated.stderr || generated.stdout);
    writeFileSync(join(fixture.directory, installerName), "tampered preview");
    const verified = verify(fixture.directory);
    assert.notEqual(verified.status, 0);
    assert.match(`${verified.stderr}${verified.stdout}`, /checksum/i);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test("rejects a Stable or v1 tag in the unsigned preview contract", () => {
  const fixture = createPreparedPreview();
  try {
    const generated = generate(fixture.directory, fixture.notes, "v1.0.0");
    assert.notEqual(generated.status, 0);
    assert.match(`${generated.stderr}${generated.stdout}`, /v0\.x/i);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});
