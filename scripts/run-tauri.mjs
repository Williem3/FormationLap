import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const repositoryRoot = resolve(import.meta.dirname, "..");
const tauriArguments = process.argv.slice(2);
const command = tauriArguments[0];
const isBuild = command === "build";
const isDev = command === "dev";

if (isBuild || isDev) {
  const prepare = spawnSync(
    process.execPath,
    [
      resolve(import.meta.dirname, "prepare-elevated-helper.mjs"),
      ...(isDev || tauriArguments.includes("--debug") ? ["--debug"] : []),
    ],
    {
      cwd: repositoryRoot,
      stdio: "inherit",
      shell: false,
    },
  );
  if (prepare.error) {
    throw prepare.error;
  }
  if (prepare.status !== 0) {
    process.exit(prepare.status ?? 1);
  }
  if (!tauriArguments.includes("--config")) {
    tauriArguments.push("--config", "src-tauri/tauri.sidecar.conf.json");
  }
}

const tauri = spawnSync(
  process.execPath,
  [
    resolve(repositoryRoot, "node_modules", "@tauri-apps", "cli", "tauri.js"),
    ...tauriArguments,
  ],
  {
    cwd: repositoryRoot,
    stdio: "inherit",
    shell: false,
  },
);
if (tauri.error) {
  throw tauri.error;
}
process.exit(tauri.status ?? 1);
