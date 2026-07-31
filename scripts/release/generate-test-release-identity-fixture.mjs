import { createPrivateKey, createPublicKey, sign } from "node:crypto";
import { readFileSync } from "node:fs";

// This seed is a test fixture only. It is deliberately not a release signing
// credential and is used solely to keep the checked-in Rust verification
// fixture reproducible when the helper protocol changes.
const testSeed = Buffer.from(
  "4f2d6f2b8bb9a1d9c6f07e4fbaf9e2d3c8105c8d6b3a2c170f9a5e6d4c2b1a09",
  "hex",
);
const keyId = Buffer.from("f1a2b3c4d5e6f708", "hex");
const trustedComment =
  "timestamp:1700000000\tfile:release_identity_payload.txt";

const payloadPath = process.argv[2];
if (!payloadPath) {
  throw new Error(
    "Usage: node generate-test-release-identity-fixture.mjs <payload>",
  );
}

const privateKey = createPrivateKey({
  key: Buffer.concat([
    Buffer.from("302e020100300506032b657004220420", "hex"),
    testSeed,
  ]),
  format: "der",
  type: "pkcs8",
});
const publicDer = createPublicKey(privateKey).export({
  format: "der",
  type: "spki",
});
const publicKey = Buffer.concat([
  Buffer.from("4564", "hex"),
  keyId,
  publicDer.subarray(-32),
]);
const payload = readFileSync(payloadPath);
const signature = sign(null, payload, privateKey);
const globalSignature = sign(
  null,
  Buffer.concat([signature, Buffer.from(trustedComment)]),
  privateKey,
);
const publicKeyText = [
  "untrusted comment: Formation Lap deterministic test release identity key",
  publicKey.toString("base64"),
].join("\n");
const signatureText = [
  "untrusted comment: Formation Lap deterministic test release identity signature",
  Buffer.concat([Buffer.from("4564", "hex"), keyId, signature]).toString(
    "base64",
  ),
  `trusted comment: ${trustedComment}`,
  globalSignature.toString("base64"),
].join("\n");

console.log(
  JSON.stringify({
    publicKey: Buffer.from(publicKeyText).toString("base64"),
    signature: Buffer.from(signatureText).toString("base64"),
  }),
);
