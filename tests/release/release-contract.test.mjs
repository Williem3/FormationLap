import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const verifier = join(
  repositoryRoot,
  "scripts",
  "release",
  "verify-release-artifacts.mjs",
);
const version = "1.0.0";
const tag = `v${version}`;
const installerName = `Formation-Lap_${version}_x64-setup.exe`;

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function createFixture() {
  const directory = mkdtempSync(join(tmpdir(), "formation-lap-release-"));
  const installer = join(directory, installerName);
  const signature = "Zml4dHVyZS10YXVyaS1zaWduYXR1cmU=";
  writeFileSync(installer, "signed installer fixture");
  writeFileSync(`${installer}.sig`, signature);
  writeFileSync(
    join(directory, "latest.json"),
    JSON.stringify(
      {
        version,
        notes: "Formation Lap release fixture",
        pub_date: "2026-07-24T12:00:00Z",
        platforms: {
          "windows-x86_64": {
            url: `https://github.com/Williem3/FormationLap/releases/download/${tag}/${installerName}`,
            signature,
          },
        },
      },
      null,
      2,
    ),
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
    join(directory, "AUTHENTICODE.txt"),
    "formation-lap.exe: Valid\nformation-lap-elevated-helper.exe: Valid\nFormation-Lap_1.0.0_x64-setup.exe: Valid\n",
  );
  const files = [
    "AUTHENTICODE.txt",
    installerName,
    `${installerName}.sig`,
    `Formation-Lap_${version}.spdx.json`,
    "THIRD-PARTY-LICENSES.json",
    "THIRD-PARTY-NOTICES.txt",
    "latest.json",
  ];
  writeFileSync(
    join(directory, "SHA256SUMS.txt"),
    `${files
      .sort()
      .map((name) => `${sha256(join(directory, name))}  ${name}`)
      .join("\n")}\n`,
  );
  return directory;
}

function verify(directory) {
  return spawnSync(
    process.execPath,
    [verifier, "--directory", directory, "--version", version, "--tag", tag],
    { cwd: repositoryRoot, encoding: "utf8", shell: false },
  );
}

test("accepts one complete official Windows release set", () => {
  const directory = createFixture();
  try {
    const result = verify(directory);
    assert.equal(result.status, 0, result.stderr || result.stdout);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("rejects a release whose signed installer no longer matches its checksum", () => {
  const directory = createFixture();
  try {
    writeFileSync(join(directory, installerName), "tampered installer");
    const result = verify(directory);
    assert.notEqual(result.status, 0);
    assert.match(`${result.stderr}${result.stdout}`, /checksum/i);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("rejects updater metadata outside the official GitHub release", () => {
  const directory = createFixture();
  try {
    const metadataPath = join(directory, "latest.json");
    const metadata = JSON.parse(readFileSync(metadataPath, "utf8"));
    metadata.platforms["windows-x86_64"].url =
      "https://example.com/Formation-Lap_1.0.0_x64-setup.exe";
    writeFileSync(metadataPath, JSON.stringify(metadata));
    const checksumsPath = join(directory, "SHA256SUMS.txt");
    const checksums = readFileSync(checksumsPath, "utf8").replace(
      /^[a-f0-9]{64} {2}latest\.json$/m,
      `${sha256(metadataPath)}  latest.json`,
    );
    writeFileSync(checksumsPath, checksums);
    const result = verify(directory);
    assert.notEqual(result.status, 0);
    assert.match(`${result.stderr}${result.stdout}`, /official GitHub/i);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
