import { exitWith } from "./exit-with";
import { checkLock } from "./lib/check-lock";

exitWith(checkLock(process.cwd(), process.argv.includes("--changed")));
