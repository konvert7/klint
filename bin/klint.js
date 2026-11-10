#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

function isMuslLinux() {
  if (process.platform !== "linux") return false;
  process.report.excludeNetwork = true;
  return !process.report.getReport().header.glibcVersionRuntime;
}

const target = `${process.platform}-${process.arch}${isMuslLinux() ? "-musl" : ""}`;
const nativePackage = `@konvert7/klint-${target}`;
const binaryName = process.platform === "win32" ? "klint-rs.exe" : "klint-rs";

function nativeBinary() {
  try {
    const manifest = createRequire(import.meta.url).resolve(`${nativePackage}/package.json`);
    return join(dirname(manifest), "bin", binaryName);
  } catch {
    return undefined;
  }
}

function fail(message) {
  process.stderr.write(`klint: ${message}\n`);
  process.exit(1);
}

const binary = nativeBinary();
if (!binary) {
  fail(
    `no native binary for ${target}: ${nativePackage} is not installed. ` +
      "Reinstall without --omit=optional, or use a supported platform " +
      "(darwin-arm64, darwin-x64, linux-arm64, linux-x64, linux-x64-musl, win32-x64)."
  );
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) fail(`could not run ${binary}: ${result.error.message}`);
if (result.signal) process.kill(process.pid, result.signal);
process.exit(result.status ?? 1);
