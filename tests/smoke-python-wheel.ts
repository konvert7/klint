import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const pythonRoot = join(root, "python");
const distDir = join(pythonRoot, "dist");
const binName = process.platform === "win32" ? "klint-rs.exe" : "klint-rs";
const stagedBinary = join(pythonRoot, "klint", "_bin", binName);
const workDir = mkdtempSync(join(tmpdir(), "klint-python-wheel-"));

if (!existsSync(stagedBinary)) {
  fail(`Missing staged Python wheel binary: ${stagedBinary}`);
}

try {
  rmSync(distDir, { recursive: true, force: true });
  mkdirSync(distDir, { recursive: true });
  run("python3", ["-m", "pip", "wheel", pythonRoot, "--wheel-dir", distDir]);
  const wheel = readdirSync(distDir).find((file) => file.endsWith(".whl"));
  if (!wheel) fail(`No wheel built in ${distDir}`);

  const venv = join(workDir, "venv");
  run("python3", ["-m", "venv", venv]);
  const python = join(
    venv,
    process.platform === "win32" ? "Scripts/python.exe" : "bin/python"
  );
  run(python, ["-m", "pip", "install", join(distDir, wheel)]);

  const fixture = join(workDir, "fixture");
  mkdirSync(join(fixture, "src", "app", "jobs"), { recursive: true });
  mkdirSync(join(fixture, "src", "app", "lib"), { recursive: true });
  writeFileSync(
    join(fixture, "klint.yaml"),
    `
include: ["src"]
rules: {}
arch:
  layers:
    jobs: ["src/app/jobs/**"]
    lib: ["src/app/lib/**"]
  imports:
    - from: jobs
      deny: lib
      message: "Jobs must not import lib directly"
`
  );
  writeFileSync(
    join(fixture, "src", "app", "jobs", "worker.py"),
    "from app.lib.auth import load_key\n"
  );
  writeFileSync(
    join(fixture, "src", "app", "lib", "auth.py"),
    "def load_key():\n    return 'x'\n"
  );

  const result = spawnSync(python, ["-m", "klint", "--config", fixture, "--json"], {
    encoding: "utf-8",
  });
  if ((result.status ?? -1) !== 2) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
    fail(`Expected installed Python wheel klint to exit 2, received ${result.status}`);
  }
  const payload = JSON.parse(result.stdout);
  if (payload.summary?.errors !== 1 || payload.violations?.[0]?.rule !== "arch/imports") {
    fail(`Unexpected Python wheel smoke output: ${result.stdout}`);
  }

  process.stdout.write(`python wheel smoke ok (${wheel})\n`);
} finally {
  rmSync(workDir, { recursive: true, force: true });
}

function run(command: string, args: string[]): void {
  const result = spawnSync(command, args, { encoding: "utf-8" });
  if ((result.status ?? -1) !== 0) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
    process.exit(result.status ?? 1);
  }
}

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
