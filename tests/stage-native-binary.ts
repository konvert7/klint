import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  nativePackageForPlatform,
  resolveNativePackageBinary,
} from "../core/native-binary";

const root = fileURLToPath(new URL("..", import.meta.url));
const nativePackage = nativePackageForPlatform();

if (!nativePackage) {
  fail(`No native package mapping for ${process.platform}-${process.arch}`);
}

const source = join(
  root,
  "target",
  "release",
  process.platform === "win32" ? "klint-rs.exe" : "klint-rs"
);
const packageRoot = join(
  root,
  "npm",
  "native",
  `${nativePackage.platform}-${nativePackage.arch}`
);
const destination = join(packageRoot, nativePackage.binaryPath);

if (!existsSync(source)) {
  fail(`Missing release binary: ${source}`);
}

mkdirSync(join(packageRoot, "bin"), { recursive: true });
copyFileSync(source, destination);

const resolved = resolveNativePackageBinary({
  packageRoot: root,
  resolvePackageJson: (packageName) => {
    if (packageName !== nativePackage.packageName) {
      throw new Error(`Unexpected package resolution: ${packageName}`);
    }
    return join(packageRoot, "package.json");
  },
});

if (resolved !== destination) {
  fail(
    `Native binary resolver returned ${resolved ?? "(none)"}, expected ${destination}`
  );
}

process.stdout.write(`staged ${nativePackage.packageName} binary at ${destination}\n`);

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
