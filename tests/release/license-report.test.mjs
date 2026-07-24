import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const generator = join(
  repositoryRoot,
  "scripts",
  "release",
  "generate-license-report.mjs",
);

test("resolves the pnpm entrypoint installed by pnpm/action-setup", async () => {
  const { resolvePnpmEntrypoint } = await import(pathToFileURL(generator).href);
  const pnpmHome = join(
    "C:",
    "Users",
    "runneradmin",
    "setup-pnpm",
    "node_modules",
    ".bin",
  );
  const expected = resolve(pnpmHome, "..", "pnpm", "bin", "pnpm.cjs");

  assert.equal(
    resolvePnpmEntrypoint(
      { PNPM_HOME: pnpmHome },
      (candidate) => candidate === expected,
    ),
    expected,
  );
});

test("generates a reviewed production dependency inventory and notices", () => {
  const directory = mkdtempSync(join(tmpdir(), "formation-lap-licenses-"));
  const reportPath = join(directory, "THIRD-PARTY-LICENSES.json");
  const noticesPath = join(directory, "THIRD-PARTY-NOTICES.txt");
  try {
    const result = spawnSync(
      process.execPath,
      [
        generator,
        "--root",
        repositoryRoot,
        "--output",
        reportPath,
        "--notices",
        noticesPath,
      ],
      {
        cwd: repositoryRoot,
        encoding: "utf8",
        shell: false,
        env: {
          ...process.env,
          PATH: `C:\\Users\\willi\\.cargo\\bin;${process.env.PATH ?? ""}`,
        },
      },
    );
    assert.equal(result.status, 0, result.stderr || result.stdout);
    const report = JSON.parse(readFileSync(reportPath, "utf8"));
    assert.deepEqual(report.deniedLicenses, []);
    assert.ok(
      report.packages.some(
        (package_) =>
          package_.ecosystem === "cargo" && package_.name === "serde",
      ),
    );
    assert.ok(
      report.packages.some(
        (package_) =>
          package_.ecosystem === "pnpm" && package_.name === "react",
      ),
    );
    assert.match(readFileSync(noticesPath, "utf8"), /react@/);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
