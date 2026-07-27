import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";

function parseArguments(arguments_) {
  const values = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const name = arguments_[index];
    const value = arguments_[index + 1];
    if (!name?.startsWith("--") || !value) {
      throw new Error("Expected --main, --helper, and --manifest arguments.");
    }
    values[name.slice(2)] = value;
  }
  for (const name of ["main", "helper", "manifest"]) {
    if (!values[name]) {
      throw new Error(`Missing required --${name} argument.`);
    }
  }
  return values;
}

function requireFile(path, expectedName) {
  const resolved = resolve(path);
  if (
    !existsSync(resolved) ||
    !statSync(resolved).isFile() ||
    basename(resolved).toLowerCase() !== expectedName
  ) {
    throw new Error(`Expected installed ${expectedName}.`);
  }
  return resolved;
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function verify() {
  const values = parseArguments(process.argv.slice(2));
  const main = requireFile(values.main, "formation-lap.exe");
  const helper = requireFile(
    values.helper,
    "formation-lap-elevated-helper.exe",
  );
  const manifestPath = requireFile(
    values.manifest,
    "formation-lap-release-identity.json",
  );
  if (
    dirname(main).toLowerCase() !== dirname(helper).toLowerCase() ||
    dirname(main).toLowerCase() !== dirname(manifestPath).toLowerCase()
  ) {
    throw new Error("The installed release identity files are not siblings.");
  }
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  if (
    !/^[a-f0-9]{64}$/.test(manifest.mainExecutableSha256 ?? "") ||
    !/^[a-f0-9]{64}$/.test(manifest.helperSha256 ?? "")
  ) {
    throw new Error(
      "The installed release identity has invalid executable hashes.",
    );
  }
  if (manifest.mainExecutableSha256 !== sha256(main)) {
    throw new Error(
      "The installed Formation Lap executable hash does not match the release identity.",
    );
  }
  if (manifest.helperSha256 !== sha256(helper)) {
    throw new Error(
      "The installed elevated helper hash does not match the release identity.",
    );
  }
  console.log("Installed release identity matches both executable hashes.");
}

try {
  verify();
} catch (error) {
  console.error(
    `Release identity layout verification failed: ${
      error instanceof Error ? error.message : String(error)
    }`,
  );
  process.exitCode = 1;
}
