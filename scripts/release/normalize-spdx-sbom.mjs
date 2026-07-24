import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

function parseArguments(arguments_) {
  const values = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const name = arguments_[index];
    const value = arguments_[index + 1];
    if (!name?.startsWith("--") || !value) {
      throw new Error("Expected --file and --version arguments.");
    }
    values[name.slice(2)] = value;
  }
  if (!values.file || !values.version) {
    throw new Error("Missing --file or --version argument.");
  }
  return values;
}

try {
  const arguments_ = parseArguments(process.argv.slice(2));
  const path = resolve(arguments_.file);
  const sbom = JSON.parse(readFileSync(path, "utf8"));
  if (sbom.spdxVersion !== "SPDX-2.3" || !Array.isArray(sbom.packages)) {
    throw new Error("Syft did not produce an SPDX 2.3 package inventory.");
  }
  const id = "SPDXRef-FormationLap";
  const existing = sbom.packages.find(
    (package_) => package_.name === "formation-lap",
  );
  if (existing) {
    existing.versionInfo = arguments_.version;
  } else {
    sbom.packages.unshift({
      SPDXID: id,
      name: "formation-lap",
      versionInfo: arguments_.version,
      downloadLocation: "NOASSERTION",
      filesAnalyzed: false,
      licenseConcluded: "MIT",
      licenseDeclared: "MIT",
      copyrightText: "NOASSERTION",
    });
  }
  sbom.name = `Formation Lap ${arguments_.version}`;
  sbom.documentDescribes = [
    existing?.SPDXID ?? id,
    ...(sbom.documentDescribes ?? []).filter(
      (candidate) => candidate !== (existing?.SPDXID ?? id),
    ),
  ];
  writeFileSync(path, `${JSON.stringify(sbom, null, 2)}\n`);
  console.log(`Normalized SPDX SBOM: Formation Lap ${arguments_.version}`);
} catch (error) {
  console.error(
    `SPDX SBOM normalization failed: ${
      error instanceof Error ? error.message : String(error)
    }`,
  );
  process.exitCode = 1;
}
