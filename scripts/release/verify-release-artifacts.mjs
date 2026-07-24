import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { basename, join, resolve } from "node:path";

function parseArguments(arguments_) {
  const values = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const name = arguments_[index];
    const value = arguments_[index + 1];
    if (!name?.startsWith("--") || !value) {
      throw new Error("Expected --directory, --version, and --tag arguments.");
    }
    values[name.slice(2)] = value;
  }
  for (const name of ["directory", "version", "tag"]) {
    if (!values[name]) {
      throw new Error(`Missing required --${name} argument.`);
    }
  }
  return values;
}

function requireVersion(value) {
  if (
    !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?$/.test(
      value,
    )
  ) {
    throw new Error(`Release version is not valid SemVer: ${value}`);
  }
}

function readJson(path, label) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    throw new Error(`${label} is not valid JSON.`);
  }
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function verifyChecksums(directory, expectedFiles) {
  const manifestPath = join(directory, "SHA256SUMS.txt");
  const entries = new Map();
  for (const line of readFileSync(manifestPath, "utf8")
    .split(/\r?\n/)
    .filter(Boolean)) {
    const match = /^([a-f0-9]{64}) {2}([^/\\]+)$/.exec(line);
    if (!match) {
      throw new Error(`Checksum manifest contains an invalid line: ${line}`);
    }
    const [, digest, name] = match;
    if (entries.has(name)) {
      throw new Error(`Checksum manifest contains duplicate file: ${name}`);
    }
    entries.set(name, digest);
  }
  assertSameNames(entries.keys(), expectedFiles, "checksum manifest");
  for (const [name, digest] of entries) {
    if (sha256(join(directory, name)) !== digest) {
      throw new Error(`Checksum mismatch for ${name}.`);
    }
  }
}

function assertSameNames(actualNames, expectedNames, label) {
  const actual = [...actualNames].sort();
  const expected = [...expectedNames].sort();
  if (
    actual.length !== expected.length ||
    actual.some((name, index) => name !== expected[index])
  ) {
    throw new Error(
      `${label} must contain exactly: ${expected.join(", ")}; found: ${actual.join(", ")}`,
    );
  }
}

function verifyUpdaterMetadata(
  metadata,
  version,
  tag,
  installerName,
  signature,
) {
  if (metadata.version !== version) {
    throw new Error("Updater metadata version does not match the release.");
  }
  if (
    typeof metadata.notes !== "string" ||
    metadata.notes.trim().length === 0
  ) {
    throw new Error("Updater metadata must include release notes.");
  }
  if (
    typeof metadata.pub_date !== "string" ||
    Number.isNaN(Date.parse(metadata.pub_date))
  ) {
    throw new Error("Updater metadata must include a valid publication date.");
  }
  const windows = metadata.platforms?.["windows-x86_64"];
  if (!windows || Object.keys(metadata.platforms).length !== 1) {
    throw new Error("Updater metadata must contain only windows-x86_64.");
  }
  const expectedUrl =
    `https://github.com/Williem3/FormationLap/releases/download/` +
    `${tag}/${installerName}`;
  if (windows.url !== expectedUrl) {
    throw new Error("Updater metadata left the official GitHub release.");
  }
  if (windows.signature !== signature) {
    throw new Error("Updater metadata signature does not match the .sig file.");
  }
}

function verifySbom(sbom, version) {
  if (
    sbom.spdxVersion !== "SPDX-2.3" ||
    !Array.isArray(sbom.packages) ||
    sbom.packages.length === 0
  ) {
    throw new Error("SPDX SBOM is missing package inventory.");
  }
  if (
    !sbom.packages.some(
      (package_) =>
        package_.name === "formation-lap" && package_.versionInfo === version,
    )
  ) {
    throw new Error("SPDX SBOM does not identify this Formation Lap version.");
  }
}

function verifyLicenses(report) {
  if (
    report.schemaVersion !== 1 ||
    !Array.isArray(report.deniedLicenses) ||
    report.deniedLicenses.length !== 0 ||
    !Array.isArray(report.packages) ||
    report.packages.length === 0
  ) {
    throw new Error("Dependency-license report did not pass policy.");
  }
  for (const package_ of report.packages) {
    if (
      !["cargo", "pnpm"].includes(package_.ecosystem) ||
      !package_.name ||
      !package_.version ||
      !package_.license
    ) {
      throw new Error(
        "Dependency-license report contains an incomplete package.",
      );
    }
  }
}

function verify() {
  const {
    directory: directoryArgument,
    version,
    tag,
  } = parseArguments(process.argv.slice(2));
  requireVersion(version);
  if (tag !== `v${version}`) {
    throw new Error("Release tag must equal v plus the synchronized version.");
  }
  const directory = resolve(directoryArgument);
  if (!existsSync(directory) || !statSync(directory).isDirectory()) {
    throw new Error("Release artifact directory does not exist.");
  }

  const installerName = `Formation-Lap_${version}_x64-setup.exe`;
  const sbomName = `Formation-Lap_${version}.spdx.json`;
  const artifactNames = [
    "AUTHENTICODE.txt",
    installerName,
    `${installerName}.sig`,
    sbomName,
    "THIRD-PARTY-LICENSES.json",
    "THIRD-PARTY-NOTICES.txt",
    "latest.json",
  ];
  const allNames = [...artifactNames, "SHA256SUMS.txt"];
  assertSameNames(readdirSync(directory), allNames, "release directory");

  const signature = readFileSync(
    join(directory, `${installerName}.sig`),
    "utf8",
  ).trim();
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(signature)) {
    throw new Error("Tauri updater signature is missing or malformed.");
  }
  verifyUpdaterMetadata(
    readJson(join(directory, "latest.json"), "Updater metadata"),
    version,
    tag,
    installerName,
    signature,
  );
  verifySbom(readJson(join(directory, sbomName), "SPDX SBOM"), version);
  verifyLicenses(
    readJson(
      join(directory, "THIRD-PARTY-LICENSES.json"),
      "Dependency-license report",
    ),
  );
  if (
    readFileSync(join(directory, "THIRD-PARTY-NOTICES.txt"), "utf8").trim()
      .length === 0
  ) {
    throw new Error("Third-party notices are empty.");
  }
  const authenticode = readFileSync(
    join(directory, "AUTHENTICODE.txt"),
    "utf8",
  );
  for (const name of [
    "formation-lap.exe",
    "formation-lap-elevated-helper.exe",
    installerName,
  ]) {
    if (!authenticode.includes(`${name}: Valid`)) {
      throw new Error(`Authenticode verification is missing for ${name}.`);
    }
  }
  verifyChecksums(directory, artifactNames);
  console.log(
    `Release artifact contract passed: ${basename(directory)} ${version}`,
  );
}

try {
  verify();
} catch (error) {
  console.error(
    `Release artifact contract failed: ${
      error instanceof Error ? error.message : String(error)
    }`,
  );
  process.exitCode = 1;
}
