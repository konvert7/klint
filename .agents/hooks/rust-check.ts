import { runHook } from "./run-hook";

const exitCode = runHook(["cargo", "check", "-p", "klint-rs"]);
process.exit(exitCode);
