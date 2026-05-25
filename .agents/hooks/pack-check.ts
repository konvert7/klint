import { runHook } from "./run-hook";

const exitCode = runHook(["bun", "run", "pack:check"]);
process.exit(exitCode);
