import { runHook } from "./run-hook";

const exitCode = runHook([
  "cargo",
  "clippy",
  "-p",
  "klint-rs",
  "--all-targets",
  "--",
  "-D",
  "warnings",
]);
process.exit(exitCode);
