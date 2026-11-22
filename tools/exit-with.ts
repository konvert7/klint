import type { CheckResult } from "./lib/check-result";

export function exitWith(result: CheckResult): never {
  process.stdout.write(result.stdout);
  process.stderr.write(result.stderr);
  process.exit(result.exitCode);
}

export function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
