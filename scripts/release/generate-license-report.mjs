import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

function parseArguments(arguments_) {
  const values = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const name = arguments_[index];
    const value = arguments_[index + 1];
    if (!name?.startsWith("--") || !value) {
      throw new Error("Expected --root, --output, and --notices arguments.");
    }
    values[name.slice(2)] = value;
  }
  for (const name of ["root", "output", "notices"]) {
    if (!values[name]) {
      throw new Error(`Missing required --${name} argument.`);
    }
  }
  return values;
}

function run(command, arguments_, cwd) {
  const result = spawnSync(command, arguments_, {
    cwd,
    encoding: "utf8",
    shell: false,
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(
      `${command} exited with ${result.status}: ${result.stderr || result.stdout}`,
    );
  }
  return result.stdout;
}

function cargoPackages(root) {
  const cargo = process.env.CARGO ?? "cargo";
  const metadata = JSON.parse(
    run(
      cargo,
      [
        "metadata",
        "--locked",
        "--format-version",
        "1",
        "--filter-platform",
        "x86_64-pc-windows-msvc",
        "--manifest-path",
        join(root, "src-tauri", "Cargo.toml"),
      ],
      root,
    ),
  );
  const resolved = new Set(metadata.resolve.nodes.map((node) => node.id));
  return metadata.packages
    .filter(
      (package_) =>
        resolved.has(package_.id) && package_.name !== "formation-lap",
    )
    .map((package_) => ({
      ecosystem: "cargo",
      name: package_.name,
      version: package_.version,
      license: package_.license ?? "<missing>",
      source:
        package_.repository ?? package_.homepage ?? package_.source ?? null,
    }));
}

export function resolvePnpmEntrypoint(
  environment = process.env,
  exists = existsSync,
) {
  const candidates = [
    environment.npm_execpath,
    environment.PNPM_HOME
      ? resolve(environment.PNPM_HOME, "..", "pnpm", "bin", "pnpm.cjs")
      : null,
    environment.PNPM_HOME ? join(environment.PNPM_HOME, "pnpm.cjs") : null,
    environment.APPDATA
      ? join(
          environment.APPDATA,
          "npm",
          "node_modules",
          "pnpm",
          "bin",
          "pnpm.cjs",
        )
      : null,
  ].filter(Boolean);
  return candidates.find((candidate) => exists(candidate));
}

function pnpmPackages(root) {
  const pnpm = resolvePnpmEntrypoint();
  if (!pnpm) {
    throw new Error(
      "The pinned pnpm JavaScript entrypoint could not be found.",
    );
  }
  const groups = JSON.parse(
    run(process.execPath, [pnpm, "licenses", "list", "--prod", "--json"], root),
  );
  const packages = [];
  for (const [groupLicense, entries] of Object.entries(groups)) {
    for (const entry of entries) {
      for (const version of entry.versions) {
        packages.push({
          ecosystem: "pnpm",
          name: entry.name,
          version,
          license: entry.license ?? groupLicense,
          source: entry.homepage ?? null,
        });
      }
    }
  }
  return packages;
}

function packageKey(package_) {
  return `${package_.ecosystem}\0${package_.name}\0${package_.version}`;
}

function generate() {
  const arguments_ = parseArguments(process.argv.slice(2));
  const root = resolve(arguments_.root);
  const output = resolve(arguments_.output);
  const notices = resolve(arguments_.notices);
  const policy = JSON.parse(
    readFileSync(join(import.meta.dirname, "allowed-licenses.json"), "utf8"),
  );
  if (policy.schemaVersion !== 1 || !Array.isArray(policy.allowed)) {
    throw new Error("License policy is invalid.");
  }
  const allowed = new Set(policy.allowed);
  const unique = new Map();
  for (const package_ of [...cargoPackages(root), ...pnpmPackages(root)]) {
    unique.set(packageKey(package_), package_);
  }
  const packages = [...unique.values()].sort(
    (left, right) =>
      left.ecosystem.localeCompare(right.ecosystem) ||
      left.name.localeCompare(right.name) ||
      left.version.localeCompare(right.version),
  );
  const deniedLicenses = [
    ...new Set(
      packages
        .map((package_) => package_.license)
        .filter((license) => !allowed.has(license)),
    ),
  ].sort();
  const report = {
    schemaVersion: 1,
    target: "x86_64-pc-windows-msvc",
    policyReviewedAt: policy.reviewedAt,
    allowedLicenses: [...allowed].sort(),
    deniedLicenses,
    packages,
  };
  mkdirSync(dirname(output), { recursive: true });
  mkdirSync(dirname(notices), { recursive: true });
  writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
  writeFileSync(
    notices,
    [
      "Formation Lap third-party dependency notices",
      "",
      "The following locked production dependencies are distributed under",
      "their respective licenses. Source links are included when declared by",
      "the package metadata.",
      "",
      ...packages.map(
        (package_) =>
          `- ${package_.ecosystem}: ${package_.name}@${package_.version} — ` +
          `${package_.license}${package_.source ? ` — ${package_.source}` : ""}`,
      ),
      "",
    ].join("\n"),
  );
  if (deniedLicenses.length > 0) {
    throw new Error(
      `Unreviewed dependency licenses: ${deniedLicenses.join(", ")}`,
    );
  }
  console.log(
    `Dependency license policy passed: ${packages.length} locked packages.`,
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    generate();
  } catch (error) {
    console.error(
      `Dependency license report failed: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
    process.exitCode = 1;
  }
}
