import { copyFileSync, mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const debug = process.argv.includes("--debug");
const repositoryRoot = resolve(import.meta.dirname, "..");
const manifestPath = join(repositoryRoot, "src-tauri", "Cargo.toml");
const cargo =
  process.env.CARGO ??
  (process.platform === "win32"
    ? join(homedir(), ".cargo", "bin", "cargo.exe")
    : "cargo");
const cargoArguments = [
  "build",
  "--manifest-path",
  manifestPath,
  "--bin",
  "formation-lap-elevated-helper",
];
if (!debug) {
  cargoArguments.push("--release");
}

const build = spawnSync(cargo, cargoArguments, {
  cwd: repositoryRoot,
  stdio: "inherit",
  shell: false,
});
if (build.error) {
  throw build.error;
}
if (build.status !== 0) {
  throw new Error(`elevated helper build exited with status ${build.status}`);
}

const profile = debug ? "debug" : "release";
const executableName =
  process.platform === "win32"
    ? "formation-lap-elevated-helper.exe"
    : "formation-lap-elevated-helper";
const source = join(
  repositoryRoot,
  "src-tauri",
  "target",
  profile,
  executableName,
);
const destinationDirectory = join(repositoryRoot, "src-tauri", "binaries");
const destination = join(
  destinationDirectory,
  `formation-lap-elevated-helper-x86_64-pc-windows-msvc${
    process.platform === "win32" ? ".exe" : ""
  }`,
);
mkdirSync(destinationDirectory, { recursive: true });
copyFileSync(source, destination);
console.log(`Prepared one-shot helper: ${destination}`);
