import { exitWith } from "./exit-with";
import { checkLf } from "./lib/check-lf";

exitWith(checkLf(process.cwd(), process.argv.includes("--fix")));
