import { runHook } from "./run-hook";

const exitCode = runHook(["bun", "run", "test:rust-engine"]);
process.exit(exitCode);
