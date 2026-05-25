import { runHook } from "./run-hook";

const exitCode = runHook(["cargo", "fmt", "--check"]);
process.exit(exitCode);
