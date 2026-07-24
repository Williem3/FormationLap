import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

function parseArguments(arguments_) {
  const values = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const name = arguments_[index];
    const value = arguments_[index + 1];
    if (!name?.startsWith("--") || !value) {
      throw new Error("Expected --root and optional --tag arguments.");
    }
    values[name.slice(2)] = value;
  }
  return values;
}

function matchVersion(document, pattern, label) {
  const version = pattern.exec(document)?.[1];
  if (!version) {
    throw new Error(`${label} does not declare the Formation Lap version.`);
  }
  return version;
}

function verify() {
  const arguments_ = parseArguments(process.argv.slice(2));
  const root = resolve(arguments_.root ?? ".");
  const packageVersion = JSON.parse(
    readFileSync(join(root, "package.json"), "utf8"),
  ).version;
  const cargoManifest = readFileSync(
    join(root, "src-tauri", "Cargo.toml"),
    "utf8",
  );
  const cargoLock = readFileSync(join(root, "src-tauri", "Cargo.lock"), "utf8");
  const tauriVersion = JSON.parse(
    readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"),
  ).version;
  const cargoVersion = matchVersion(
    cargoManifest,
    /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
    "Cargo.toml",
  );
  const lockVersion = matchVersion(
    cargoLock,
    /^\[\[package\]\]\r?\nname = "formation-lap"\r?\nversion = "([^"]+)"/m,
    "Cargo.lock",
  );
  const versions = {
    "package.json": packageVersion,
    "Cargo.toml": cargoVersion,
    "Cargo.lock": lockVersion,
    "tauri.conf.json": tauriVersion,
  };
  if (Object.values(versions).some((version) => version !== packageVersion)) {
    throw new Error(
      `Version drift detected: ${Object.entries(versions)
        .map(([name, version]) => `${name}=${version}`)
        .join(", ")}`,
    );
  }
  if (
    typeof packageVersion !== "string" ||
    !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?$/.test(
      packageVersion,
    )
  ) {
    throw new Error(`Version is not valid SemVer: ${packageVersion}`);
  }
  if (arguments_.tag && arguments_.tag !== `v${packageVersion}`) {
    throw new Error(
      `Release tag ${arguments_.tag} does not identify v${packageVersion}.`,
    );
  }
  console.log(`Release version contract passed: ${packageVersion}`);
}

try {
  verify();
} catch (error) {
  console.error(
    `Release version contract failed: ${
      error instanceof Error ? error.message : String(error)
    }`,
  );
  process.exitCode = 1;
}
