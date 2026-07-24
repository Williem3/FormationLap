import { createHash } from "node:crypto";
import {
  existsSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";

function parseArguments(arguments_) {
  const values = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const name = arguments_[index];
    const value = arguments_[index + 1];
    if (!name?.startsWith("--") || !value) {
      throw new Error(
        "Expected --directory, --version, --tag, --notes, and --published-at arguments.",
      );
    }
    values[name.slice(2)] = value;
  }
  for (const name of ["directory", "version", "tag", "notes", "published-at"]) {
    if (!values[name]) {
      throw new Error(`Missing required --${name} argument.`);
    }
  }
  return values;
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function sameNames(actual, expected) {
  const sortedActual = [...actual].sort();
  const sortedExpected = [...expected].sort();
  return (
    sortedActual.length === sortedExpected.length &&
    sortedActual.every((name, index) => name === sortedExpected[index])
  );
}

function generate() {
  const arguments_ = parseArguments(process.argv.slice(2));
  const directory = resolve(arguments_.directory);
  const version = arguments_.version;
  const tag = arguments_.tag;
  const notesPath = resolve(arguments_.notes);
  const publishedAt = arguments_["published-at"];
  if (tag !== `v${version}`) {
    throw new Error("Release tag must equal v plus the release version.");
  }
  if (Number.isNaN(Date.parse(publishedAt))) {
    throw new Error("Publication timestamp is invalid.");
  }
  if (!existsSync(directory) || !statSync(directory).isDirectory()) {
    throw new Error("Prepared release directory does not exist.");
  }
  const installerName = `Formation-Lap_${version}_x64-setup.exe`;
  const preparedNames = [
    "AUTHENTICODE.txt",
    installerName,
    `${installerName}.sig`,
    `Formation-Lap_${version}.spdx.json`,
    "THIRD-PARTY-LICENSES.json",
    "THIRD-PARTY-NOTICES.txt",
  ];
  if (!sameNames(readdirSync(directory), preparedNames)) {
    throw new Error(
      `Prepared release must contain exactly: ${preparedNames.sort().join(", ")}`,
    );
  }
  const notes = readFileSync(notesPath, "utf8").trim();
  if (!notes) {
    throw new Error("Release notes are empty.");
  }
  const signature = readFileSync(
    join(directory, `${installerName}.sig`),
    "utf8",
  ).trim();
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(signature)) {
    throw new Error("Tauri updater signature is missing or malformed.");
  }
  const metadata = {
    version,
    notes,
    pub_date: new Date(publishedAt).toISOString(),
    platforms: {
      "windows-x86_64": {
        url:
          `https://github.com/Williem3/FormationLap/releases/download/` +
          `${tag}/${installerName}`,
        signature,
      },
    },
  };
  writeFileSync(
    join(directory, "latest.json"),
    `${JSON.stringify(metadata, null, 2)}\n`,
  );
  const artifactNames = [...preparedNames, "latest.json"].sort();
  writeFileSync(
    join(directory, "SHA256SUMS.txt"),
    `${artifactNames
      .map((name) => `${sha256(join(directory, name))}  ${name}`)
      .join("\n")}\n`,
  );
  console.log(`Release metadata generated: ${tag}`);
}

try {
  generate();
} catch (error) {
  console.error(
    `Release metadata generation failed: ${
      error instanceof Error ? error.message : String(error)
    }`,
  );
  process.exitCode = 1;
}
