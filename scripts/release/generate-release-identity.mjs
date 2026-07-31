import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, resolve } from "node:path";

const schemaVersion = 1;
const protocolVersion = 3;

function parseArguments(arguments_) {
  const values = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const name = arguments_[index];
    const value = arguments_[index + 1];
    if (!name?.startsWith("--") || !value) {
      throw new Error(
        "Expected named main, helper, version, channel, and payload arguments.",
      );
    }
    values[name.slice(2)] = value;
  }
  for (const name of ["main", "helper", "version", "channel", "payload"]) {
    if (!values[name]) {
      throw new Error(`Missing required --${name} argument.`);
    }
  }
  if (Boolean(values.signature) !== Boolean(values.output)) {
    throw new Error("--signature and --output must be supplied together.");
  }
  return values;
}

function requireExecutable(path, expectedName) {
  const resolved = resolve(path);
  if (
    !existsSync(resolved) ||
    !statSync(resolved).isFile() ||
    basename(resolved).toLowerCase() !== expectedName
  ) {
    throw new Error(`Expected final ${expectedName} executable bytes.`);
  }
  return resolved;
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function identityFor(values) {
  if (
    !/^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?$/.test(
      values.version,
    )
  ) {
    throw new Error("Release identity version is not valid SemVer.");
  }
  if (!["preview", "beta", "stable"].includes(values.channel)) {
    throw new Error("Release identity channel is not recognized.");
  }
  if (values.channel === "preview" && values["authenticode-signer-sha256"]) {
    throw new Error(
      "A preview release identity cannot claim an Authenticode signer.",
    );
  }
  if (
    values.channel !== "preview" &&
    !/^[a-f0-9]{64}$/.test(values["authenticode-signer-sha256"] ?? "")
  ) {
    throw new Error(
      "A signed release identity requires the lowercase SHA-256 of its approved Authenticode signer certificate.",
    );
  }
  const main = requireExecutable(values.main, "formation-lap.exe");
  const helper = requireExecutable(
    values.helper,
    "formation-lap-elevated-helper.exe",
  );
  if (dirname(main).toLowerCase() !== dirname(helper).toLowerCase()) {
    throw new Error("Final Formation Lap executables are not siblings.");
  }
  const identity = {
    schemaVersion,
    mainExecutableSha256: sha256(main),
    helperSha256: sha256(helper),
    version: values.version,
    protocolVersion,
    releaseChannel: values.channel,
  };
  if (values.channel !== "preview") {
    identity.authenticodeSignerSha256 = values["authenticode-signer-sha256"];
  }
  return identity;
}

function signingPayload(identity) {
  let payload =
    "formation-lap-release-identity-v1\n" +
    `mainExecutableSha256=${identity.mainExecutableSha256}\n` +
    `helperSha256=${identity.helperSha256}\n` +
    `version=${identity.version}\n` +
    `protocolVersion=${identity.protocolVersion}\n` +
    `releaseChannel=${identity.releaseChannel}\n`;
  if (identity.authenticodeSignerSha256) {
    payload += `authenticodeSignerSha256=${identity.authenticodeSignerSha256}\n`;
  }
  return payload;
}

function generate() {
  const values = parseArguments(process.argv.slice(2));
  const identity = identityFor(values);
  const payload = signingPayload(identity);
  const payloadPath = resolve(values.payload);

  if (!values.signature) {
    mkdirSync(dirname(payloadPath), { recursive: true });
    writeFileSync(payloadPath, payload, { encoding: "utf8", flag: "wx" });
    console.log(`Release identity payload prepared: ${payloadPath}`);
    return;
  }

  if (
    !existsSync(payloadPath) ||
    readFileSync(payloadPath, "utf8") !== payload
  ) {
    throw new Error(
      "Release identity payload no longer matches the final executable bytes.",
    );
  }
  const signature = readFileSync(resolve(values.signature), "utf8").trim();
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(signature)) {
    throw new Error("Release identity signature is missing or malformed.");
  }
  const output = resolve(values.output);
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(
    output,
    `${JSON.stringify({ ...identity, signature }, null, 2)}\n`,
    { encoding: "utf8", flag: "wx" },
  );
  console.log(`Release identity manifest sealed: ${output}`);
}

try {
  generate();
} catch (error) {
  console.error(
    `Release identity generation failed: ${
      error instanceof Error ? error.message : String(error)
    }`,
  );
  process.exitCode = 1;
}
