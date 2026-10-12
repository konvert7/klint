import { runHook } from "./run-hook";

const exitCode = runHook(["bun", "run", "klint", "--json", "--hook"]);
process.exit(exitCode);
