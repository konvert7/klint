import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { fail, replaceRequired, valueAfter } from "./release-cli";

interface Options {
  name: string;
  root: string;
  version: string;
}

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const options = parseArgs();
const pythonRoot = join(options.root, "python");
const pyprojectPath = join(pythonRoot, "pyproject.toml");
const initPath = join(pythonRoot, "klint", "__init__.py");

const pyproject = readFileSync(pyprojectPath, "utf-8");
const nextPyproject = replaceRequired(
  replaceRequired(pyproject, /^name = ".+"$/m, `name = "${options.name}"`, pyprojectPath),
  /^version = ".+"$/m,
  `version = "${options.version}"`,
  pyprojectPath
);
writeFileSync(pyprojectPath, nextPyproject);

const init = readFileSync(initPath, "utf-8");
const nextInit = replaceRequired(
  init,
  /^__version__ = ".+"$/m,
  `__version__ = "${options.version}"`,
  initPath
);
writeFileSync(initPath, nextInit);

process.stdout.write(
  `prepared Python release metadata: ${options.name}@${options.version}\n`
);

function parseArgs(): Options {
  const args = process.argv.slice(2);
  const name = valueAfter(args, "--name");
  const version = valueAfter(args, "--version");
  if (!name || !version) {
    fail(
      "Usage: bun tools/prepare-python-release.ts --name <package-name> --version <version> [--root <repo-root>]"
    );
  }

  return {
    name,
    version,
    root: resolve(valueAfter(args, "--root") ?? repoRoot),
  };
}
