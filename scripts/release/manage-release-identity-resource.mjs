import { mkdirSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

const bootstrapManifest = `${JSON.stringify(
  {
    schemaVersion: 0,
    bootstrapOnly: true,
  },
  null,
  2,
)}\n`;

function parseArguments(arguments_) {
  const values = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const name = arguments_[index];
    const value = arguments_[index + 1];
    if (!name?.startsWith("--") || !value) {
      throw new Error("Expected --manifest and --action arguments.");
    }
    values[name.slice(2)] = value;
  }
  if (!values.manifest || !["prepare", "clear"].includes(values.action)) {
    throw new Error("Expected --manifest and --action prepare|clear.");
  }
  return values;
}

function manage() {
  const values = parseArguments(process.argv.slice(2));
  const manifest = resolve(values.manifest);
  if (values.action === "prepare") {
    mkdirSync(dirname(manifest), { recursive: true });
    writeFileSync(manifest, bootstrapManifest, {
      encoding: "utf8",
      flag: "wx",
    });
    console.log(`Release identity bootstrap prepared: ${manifest}`);
    return;
  }

  if (readFileSync(manifest, "utf8") !== bootstrapManifest) {
    throw new Error(
      "Release identity resource does not match the exact bootstrap manifest.",
    );
  }
  unlinkSync(manifest);
  console.log(`Release identity bootstrap cleared: ${manifest}`);
}

try {
  manage();
} catch (error) {
  console.error(
    `Release identity bootstrap failed: ${
      error instanceof Error ? error.message : String(error)
    }`,
  );
  process.exitCode = 1;
}
