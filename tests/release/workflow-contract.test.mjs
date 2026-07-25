import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import test from "node:test";

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const ci = readFileSync(
  join(repositoryRoot, ".github", "workflows", "ci.yml"),
  "utf8",
);
const release = readFileSync(
  join(repositoryRoot, ".github", "workflows", "release.yml"),
  "utf8",
);
const previewPath = join(repositoryRoot, ".github", "workflows", "preview.yml");

function assertActionsPinned(workflow) {
  const uses = [
    ...workflow.matchAll(/^\s*(?:-\s+)?uses:\s*(\S+)(?:\s+#.*)?$/gm),
  ].map((match) => match[1]);
  assert.ok(uses.length > 0);
  for (const action of uses) {
    assert.match(
      action,
      /^[^@\s]+@[0-9a-f]{40}$/,
      `action is not pinned to a full commit: ${action}`,
    );
  }
}

test("CI pins actions and gates the complete Windows and dependency surface", () => {
  assertActionsPinned(ci);
  assert.match(ci, /pnpm verify/);
  assert.match(ci, /pnpm audit --audit-level high --prod/);
  assert.match(ci, /cargo-audit --version 0\.22\.2 --locked/);
  assert.match(ci, /cargo audit --file src-tauri\/Cargo\.lock/);
  assert.match(ci, /--all-targets --all-features/);
  assert.match(ci, /generate-license-report\.mjs/);
  assert.match(ci, /dependency-review-action@[0-9a-f]{40}/);
});

test("release workflow signs every shipped executable before updater metadata", () => {
  assertActionsPinned(release);
  assert.match(release, /tags:\s*\r?\n\s+- "v\*"\s*\r?\n\s+- "!v0\.\*"/);
  assert.match(release, /\^v\(\?:\[1-9\]\\d\*\)\\\./);
  assert.match(release, /environment: release/);
  assert.match(release, /id-token: write/);
  assert.match(release, /attestations: write/);
  assert.match(release, /artifact-metadata: write/);
  assert.match(release, /verify-release-version\.mjs/);
  assert.match(release, /tauri build --no-bundle/);
  assert.match(release, /Azure\/artifact-signing-action@[0-9a-f]{40}/);
  assert.match(release, /Get-AuthenticodeSignature/);
  assert.match(release, /tauri bundle --bundles nsis/);
  assert.match(release, /tauri signer sign/);
  assert.match(release, /generate-license-report\.mjs/);
  assert.match(release, /generate-release-metadata\.mjs/);
  assert.match(release, /verify-release-artifacts\.mjs/);
  assert.match(release, /actions\/attest@[0-9a-f]{40}/);
  assert.match(release, /gh release create/);
  assert.doesNotMatch(
    release,
    /generate-preview-metadata|verify-preview-artifacts|UNSIGNED-PREVIEW/,
  );

  const build = release.indexOf("tauri build --no-bundle");
  const firstSigning = release.indexOf("Azure/artifact-signing-action");
  const bundle = release.indexOf("tauri bundle --bundles nsis");
  const secondSigning = release.indexOf(
    "Azure/artifact-signing-action",
    firstSigning + 1,
  );
  const updaterSigning = release.indexOf("tauri signer sign");
  const metadata = release.indexOf("generate-release-metadata.mjs");
  assert.ok(
    build < firstSigning &&
      firstSigning < bundle &&
      bundle < secondSigning &&
      secondSigning < updaterSigning &&
      updaterSigning < metadata,
    "signing and metadata stages are in the wrong order",
  );
});

test("preview workflow publishes only explicit unsigned v0.x prereleases", () => {
  assert.ok(
    existsSync(previewPath),
    "the separately gated technical-preview workflow is missing",
  );
  const preview = readFileSync(previewPath, "utf8");
  assertActionsPinned(preview);
  assert.doesNotMatch(preview, /^\s*push:/m);
  assert.match(preview, /workflow_dispatch:/);
  assert.match(preview, /environment: preview/);
  assert.match(preview, /\^v0\\\./);
  assert.match(preview, /verify-release-version\.mjs/);
  assert.match(preview, /tauri bundle --bundles nsis/);
  assert.doesNotMatch(preview, /Azure\/artifact-signing-action/);
  assert.doesNotMatch(preview, /Get-AuthenticodeSignature/);
  assert.doesNotMatch(preview, /AUTHENTICODE\.txt/);
  assert.match(preview, /UNSIGNED-PREVIEW\.txt/);
  assert.match(preview, /tauri signer sign/);
  assert.match(preview, /generate-preview-metadata\.mjs/);
  assert.match(preview, /verify-preview-artifacts\.mjs/);
  assert.match(preview, /actions\/attest@[0-9a-f]{40}/);
  assert.match(preview, /gh release create/);
  assert.match(preview, /--prerelease/);
  assert.doesNotMatch(preview, /--latest/);
});
