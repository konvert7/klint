import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { formulaFromArgs } from "./lib/generate-homebrew-formula";

const { output, formula } = formulaFromArgs(process.argv.slice(2));
const path = resolve(output);
mkdirSync(dirname(path), { recursive: true });
writeFileSync(path, formula);
