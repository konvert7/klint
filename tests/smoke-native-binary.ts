import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { nativePackageForPlatform } from "../core/native-binary";

const root = fileURLToPath(new URL("..", import.meta.url));
const nativePackage = nativePackageForPlatform();

if (!nativePackage) {
  fail(`No native package mapping for ${process.platform}-${process.arch}`);
}

const binary = join(
  root,
  "npm",
  "native",
  `${nativePackage.platform}-${nativePackage.arch}`,
  nativePackage.binaryPath
);
const fixture = mkdtempSync(join(tmpdir(), "klint-native-smoke-"));

try {
  writeFileSync(join(fixture, "klint.yaml"), 'include: ["src"]\nrules: {}\n');
  const result = spawnSync(binary, ["--config", fixture, "--json"], {
    encoding: "utf-8",
  });

  if ((result.status ?? -1) !== 0) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
    process.exit(result.status ?? 1);
  }

  const output = JSON.parse(result.stdout);
  if (output.summary?.errors !== 0 || output.summary?.warnings !== 0) {
    fail(`Unexpected smoke output: ${result.stdout}`);
  }

  process.stdout.write(`native smoke ok (${binary})\n`);
} finally {
  rmSync(fixture, { recursive: true, force: true });
}

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
