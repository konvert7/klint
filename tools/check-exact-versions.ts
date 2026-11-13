import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { nativePackages } from "./native-binary";

const DEPENDENCY_SECTIONS = [
  "dependencies",
  "devDependencies",
  "optionalDependencies",
  "peerDependencies",
  "overrides",
] as const;

const EXACT_SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;

type Manifest = Partial<Record<string, unknown>>;

interface NonExactVersion {
  section: (typeof DEPENDENCY_SECTIONS)[number];
  name: string;
  version: string;
}

export function versionsStampedAtRelease(): Set<string> {
  return new Set(nativePackages().map((nativePackage) => nativePackage.packageName));
}

type VersionTree = { [name: string]: string | VersionTree };

function qualifiedName(parent: string, key: string): string {
  if (key === ".") return parent;

  return parent ? `${parent} > ${key}` : key;
}

function flattenVersions(tree: VersionTree, parent = ""): Array<[string, string]> {
  return Object.entries(tree).flatMap(([key, value]): Array<[string, string]> => {
    const name = qualifiedName(parent, key);

    return typeof value === "string" ? [[name, value]] : flattenVersions(value, name);
  });
}

export function nonExactVersions(
  manifest: Manifest,
  stampedAtRelease: Set<string> = new Set()
): NonExactVersion[] {
  return DEPENDENCY_SECTIONS.flatMap((section) =>
    flattenVersions((manifest[section] ?? {}) as VersionTree)
      .filter(
        ([name, version]) => !stampedAtRelease.has(name) && !EXACT_SEMVER.test(version)
      )
      .map(([name, version]) => ({ section, name, version }))
  );
}

export function trackedManifests(root: string): string[] {
  const listed = spawnSync("git", ["ls-files", "package.json", "**/package.json"], {
    cwd: root,
    encoding: "utf8",
  });
  if (listed.status !== 0) throw new Error(listed.stderr || "git ls-files failed");

  return listed.stdout.split("\n").filter(Boolean);
}

function readManifest(path: string): Manifest {
  try {
    return JSON.parse(readFileSync(resolve(path), "utf8"));
  } catch (error) {
    throw new Error(`${path} is not valid JSON: ${(error as Error).message}`);
  }
}

if (import.meta.main) {
  const stamped = versionsStampedAtRelease();
  const offenders = trackedManifests(".").flatMap((path) =>
    nonExactVersions(readManifest(path), stamped).map(
      (o) => `  ${path} ${o.section}: ${o.name} ${o.version}`
    )
  );
  if (offenders.length === 0) {
    process.stdout.write("Every dependency in every package.json is pinned exactly.\n");
    process.exit(0);
  }

  process.stderr.write(
    `Dependencies without an exact version:\n${offenders.join("\n")}\n` +
      "Pin each to the version bun.lock resolves (bun add --exact <name>@<version>).\n"
  );
  process.exit(1);
}
