import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const generator = join(
  repositoryRoot,
  "scripts",
  "release",
  "generate-release-identity.mjs",
);
const protocolSource = readFileSync(
  join(repositoryRoot, "src-tauri", "src", "privilege_protocol.rs"),
  "utf8",
);
const protocolVersion = Number(
  /ELEVATED_HELPER_PROTOCOL_VERSION:\s*u16\s*=\s*(\d+)/.exec(
    protocolSource,
  )?.[1],
);

function fixture() {
  const directory = mkdtempSync(join(tmpdir(), "formation-lap-identity-"));
  const main = join(directory, "formation-lap.exe");
  const helper = join(directory, "formation-lap-elevated-helper.exe");
  const payload = join(directory, "formation-lap-release-identity.payload");
  const signature = `${payload}.sig`;
  const output = join(directory, "formation-lap-release-identity.json");
  writeFileSync(main, "main release bytes");
  writeFileSync(helper, "helper release bytes");
  return { directory, main, helper, payload, signature, output };
}

function runGenerator(
  paths,
  extraArguments = [],
  { version = "0.9.0-preview.1", channel = "preview" } = {},
) {
  return spawnSync(
    process.execPath,
    [
      generator,
      "--main",
      paths.main,
      "--helper",
      paths.helper,
      "--version",
      version,
      "--channel",
      channel,
      "--payload",
      paths.payload,
      ...extraArguments,
    ],
    {
      cwd: repositoryRoot,
      encoding: "utf8",
      shell: false,
    },
  );
}

test("release identity binds the final main and helper bytes before sealing", () => {
  const paths = fixture();
  try {
    const prepared = runGenerator(paths);
    assert.equal(prepared.status, 0, prepared.stderr);
    const payload = readFileSync(paths.payload, "utf8");
    assert.match(payload, /^formation-lap-release-identity-v1$/m);
    assert.match(payload, /^mainExecutableSha256=[a-f0-9]{64}$/m);
    assert.match(payload, /^helperSha256=[a-f0-9]{64}$/m);
    assert.match(payload, /^version=0\.9\.0-preview\.1$/m);
    assert.match(
      payload,
      new RegExp(`^protocolVersion=${protocolVersion}$`, "m"),
    );
    assert.match(payload, /^releaseChannel=preview$/m);

    writeFileSync(
      paths.signature,
      Buffer.from("fixture minisign signature").toString("base64"),
    );
    const sealed = runGenerator(paths, [
      "--signature",
      paths.signature,
      "--output",
      paths.output,
    ]);
    assert.equal(sealed.status, 0, sealed.stderr);
    const manifest = JSON.parse(readFileSync(paths.output, "utf8"));
    assert.equal(manifest.schemaVersion, 1);
    assert.match(manifest.mainExecutableSha256, /^[a-f0-9]{64}$/);
    assert.match(manifest.helperSha256, /^[a-f0-9]{64}$/);
    assert.equal(manifest.version, "0.9.0-preview.1");
    assert.equal(manifest.protocolVersion, protocolVersion);
    assert.equal(manifest.releaseChannel, "preview");
    assert.equal(
      manifest.signature,
      Buffer.from("fixture minisign signature").toString("base64"),
    );
  } finally {
    rmSync(paths.directory, { recursive: true, force: true });
  }
});

test("release identity refuses to seal after either executable changes", () => {
  const paths = fixture();
  try {
    const prepared = runGenerator(paths);
    assert.equal(prepared.status, 0, prepared.stderr);
    writeFileSync(
      paths.signature,
      Buffer.from("fixture minisign signature").toString("base64"),
    );
    writeFileSync(paths.helper, "tampered helper bytes");

    const sealed = runGenerator(paths, [
      "--signature",
      paths.signature,
      "--output",
      paths.output,
    ]);
    assert.notEqual(sealed.status, 0);
    assert.match(`${sealed.stderr}${sealed.stdout}`, /final executable bytes/i);
  } finally {
    rmSync(paths.directory, { recursive: true, force: true });
  }
});

test("signed identity requires and seals one approved signer certificate", () => {
  const paths = fixture();
  const signed = {
    version: "1.0.0-beta.1",
    channel: "beta",
  };
  const signer = "a".repeat(64);
  try {
    const missingSigner = runGenerator(paths, [], signed);
    assert.notEqual(missingSigner.status, 0);
    assert.match(
      `${missingSigner.stderr}${missingSigner.stdout}`,
      /approved Authenticode signer certificate/i,
    );

    const prepared = runGenerator(
      paths,
      ["--authenticode-signer-sha256", signer],
      signed,
    );
    assert.equal(prepared.status, 0, prepared.stderr);
    assert.match(
      readFileSync(paths.payload, "utf8"),
      new RegExp(`^authenticodeSignerSha256=${signer}$`, "m"),
    );
    writeFileSync(
      paths.signature,
      Buffer.from("fixture minisign signature").toString("base64"),
    );
    const sealed = runGenerator(
      paths,
      [
        "--authenticode-signer-sha256",
        signer,
        "--signature",
        paths.signature,
        "--output",
        paths.output,
      ],
      signed,
    );
    assert.equal(sealed.status, 0, sealed.stderr);
    assert.equal(
      JSON.parse(readFileSync(paths.output, "utf8")).authenticodeSignerSha256,
      signer,
    );
  } finally {
    rmSync(paths.directory, { recursive: true, force: true });
  }
});

test("preview identity rejects an Authenticode signer claim", () => {
  const paths = fixture();
  try {
    const result = runGenerator(paths, [
      "--authenticode-signer-sha256",
      "a".repeat(64),
    ]);
    assert.notEqual(result.status, 0);
    assert.match(
      `${result.stderr}${result.stdout}`,
      /preview release identity cannot claim/i,
    );
  } finally {
    rmSync(paths.directory, { recursive: true, force: true });
  }
});
