import type { CheckResult } from "./lib/check-result";

export function exitWith(result: CheckResult): never {
  process.stdout.write(result.stdout);
  process.stderr.write(result.stderr);
  process.exit(result.exitCode);
}
