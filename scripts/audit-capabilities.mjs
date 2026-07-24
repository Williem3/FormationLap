import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const readJson = async (path) => JSON.parse(await readFile(path, "utf8"));

const [
  capability,
  tauriConfig,
  sidecarConfig,
  packageJson,
  nativeHost,
  windowsManifest,
] = await Promise.all([
  readJson("src-tauri/capabilities/main.json"),
  readJson("src-tauri/tauri.conf.json"),
  readJson("src-tauri/tauri.sidecar.conf.json"),
  readJson("package.json"),
  readFile("src-tauri/src/lib.rs", "utf8"),
  readFile("src-tauri/windows-app-manifest.xml", "utf8"),
]);

assert.equal(capability.identifier, "main-local");
assert.equal(capability.local, true);
assert.deepEqual(capability.windows, ["main"]);
assert.deepEqual(
  capability.permissions,
  [],
  "M1 must not grant built-in or plugin permissions to the WebView",
);

const windows = tauriConfig.app?.windows;
assert.equal(windows?.length, 1);
assert.equal(windows[0]?.label, "main");
assert.equal(windows[0]?.decorations, true);
assert.equal(
  Object.hasOwn(windows[0], "url"),
  false,
  "the main window must load frontendDist rather than a configured remote URL",
);
assert.deepEqual(sidecarConfig.bundle?.externalBin, [
  "binaries/formation-lap-elevated-helper",
]);
assert.match(
  windowsManifest,
  /requestedExecutionLevel level="asInvoker" uiAccess="false"/,
);
assert.doesNotMatch(windowsManifest, /requireAdministrator/);

const csp = tauriConfig.app?.security?.csp ?? "";
const devCsp = tauriConfig.app?.security?.devCsp ?? "";
assert.match(csp, /default-src 'self'/);
assert.doesNotMatch(csp, /https?:\/\/\*/);
assert.doesNotMatch(
  csp,
  /(?:https?|wss?):\/\/(?:localhost|127\.0\.0\.1)/,
  "production CSP must not grant browser network access to loopback services",
);
assert.match(devCsp, /http:\/\/localhost:1420/);
assert.match(devCsp, /http:\/\/127\.0\.0\.1:1420/);
assert.match(devCsp, /ws:\/\/localhost:1420/);
assert.match(devCsp, /ws:\/\/127\.0\.0\.1:1420/);
assert.equal(
  Object.hasOwn(
    tauriConfig.app?.security ?? {},
    "dangerousRemoteDomainIpcAccess",
  ),
  false,
);

const allPackages = {
  ...packageJson.dependencies,
  ...packageJson.devDependencies,
};
const forbiddenPackages = Object.keys(allPackages).filter((name) =>
  /^@tauri-apps\/plugin-(fs|http|shell|opener|upload|websocket)$/.test(name),
);
assert.deepEqual(
  forbiddenPackages,
  [],
  "generic system/network Tauri plugins are forbidden in M1",
);

const handlerMatch = nativeHost.match(
  /generate_handler!\[(?<commands>[^\]]+)\]/,
);
assert.ok(handlerMatch?.groups?.commands);
const commands = handlerMatch.groups.commands
  .split(",")
  .map((command) => command.trim())
  .filter(Boolean);
assert.deepEqual(commands, [
  "commands::get_app_snapshot",
  "commands::create_profile",
  "commands::save_profile",
  "commands::select_profile",
  "commands::duplicate_profile",
  "commands::delete_profile",
  "commands::export_profile",
  "commands::import_profile",
  "commands::start_application",
  "commands::refresh_processes",
  "commands::exit_application",
  "commands::force_stop_application",
  "commands::restart_application",
  "commands::start_session",
  "commands::test_game_launch",
  "commands::cancel_startup",
  "commands::close_session",
  "commands::accept_recovery",
  "commands::dismiss_recovery",
  "commands::discover_applications",
  "commands::recommend_applications",
]);
assert.match(nativeHost, /on_navigation/);

console.log(
  "Capability audit passed: local non-administrative main window, zero core/plugin permissions, twenty-one narrow app commands, one bundled helper, remote navigation guard, production IPC-only CSP, development-only Vite loopback.",
);
