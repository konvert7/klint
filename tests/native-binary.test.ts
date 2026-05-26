import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  nativePackageForPlatform,
  nativePackages,
  resolveNativePackageBinary,
} from "../core/native-binary";

const root = new URL("..", import.meta.url);

interface PackageJson {
  name: string;
  version?: string;
  optionalDependencies?: Record<string, string>;
  private?: boolean;
  repository?: {
    type?: string;
    url?: string;
  };
}

function readPackageJson(path: URL): PackageJson {
  return JSON.parse(readFileSync(path, "utf-8")) as PackageJson;
}

describe("native binary package metadata", () => {
  test("maps supported platforms to package names", () => {
    expect(nativePackageForPlatform("darwin", "arm64")).toMatchObject({
      packageName: "@konvert7/klint-darwin-arm64",
      binaryPath: "bin/klint-rs",
    });
    expect(nativePackageForPlatform("darwin", "x64")).toMatchObject({
      packageName: "@konvert7/klint-darwin-x64",
      binaryPath: "bin/klint-rs",
    });
    expect(nativePackageForPlatform("linux", "x64")).toMatchObject({
      packageName: "@konvert7/klint-linux-x64",
      binaryPath: "bin/klint-rs",
    });
    expect(nativePackageForPlatform("win32", "x64")).toMatchObject({
      packageName: "@konvert7/klint-win32-x64",
      binaryPath: "bin/klint-rs.exe",
    });
  });

  test("does not invent unsupported platform packages", () => {
    expect(nativePackageForPlatform("linux", "arm64")).toBeUndefined();
    expect(nativePackageForPlatform("freebsd", "x64")).toBeUndefined();
  });

  test("keeps package names unique", () => {
    const names = nativePackages().map((pkg) => pkg.packageName);
    expect(new Set(names).size).toBe(names.length);
  });

  test("root optional dependencies list every native package", () => {
    const packageJson = readPackageJson(new URL("package.json", root));
    const optionalDependencies = packageJson.optionalDependencies ?? {};
    const expectedNames = nativePackages()
      .map((pkg) => pkg.packageName)
      .sort();

    expect(Object.keys(optionalDependencies).sort()).toEqual(expectedNames);
    expect(packageJson.version).toBeString();
    for (const packageName of expectedNames) {
      expect(optionalDependencies[packageName]).toBe(packageJson.version as string);
    }
  });

  test("native packages remain private in source before release prep", () => {
    for (const nativePackage of nativePackages()) {
      const packageJson = readPackageJson(
        new URL(
          `npm/native/${nativePackage.platform}-${nativePackage.arch}/package.json`,
          root
        )
      );

      expect(packageJson.name).toBe(nativePackage.packageName);
      expect(packageJson.private).toBe(true);
      expect(packageJson.repository).toEqual({
        type: "git",
        url: "git+https://github.com/konvert7/klint.git",
      });
    }
  });

  test("resolves an installed optional package binary", () => {
    const packageRoot = "/repo/node_modules/@konvert7/klint";
    const packageJsonPath =
      "/repo/node_modules/@konvert7/klint-darwin-arm64/package.json";
    const expectedBinary = join(
      "/repo/node_modules/@konvert7/klint-darwin-arm64",
      "bin",
      "klint-rs"
    );

    const binaryPath = resolveNativePackageBinary({
      packageRoot,
      platform: "darwin",
      arch: "arm64",
      resolvePackageJson(packageName) {
        expect(packageName).toBe("@konvert7/klint-darwin-arm64");
        return packageJsonPath;
      },
      exists(path) {
        return path === expectedBinary;
      },
      readPackageJson(path) {
        expect([join(packageRoot, "package.json"), packageJsonPath]).toContain(path);
        return { version: "1.2.3" };
      },
    });

    expect(binaryPath).toBe(expectedBinary);
  });

  test("skips stale optional package binaries from older root versions", () => {
    const packageRoot = "/repo/node_modules/@konvert7/klint";
    const packageJsonPath =
      "/repo/node_modules/@konvert7/klint-darwin-arm64/package.json";

    const binaryPath = resolveNativePackageBinary({
      packageRoot,
      platform: "darwin",
      arch: "arm64",
      resolvePackageJson() {
        return packageJsonPath;
      },
      exists() {
        return true;
      },
      readPackageJson(path) {
        return { version: path === packageJsonPath ? "1.2.2" : "1.2.3" };
      },
    });

    expect(binaryPath).toBeUndefined();
  });

  test("resolves a package for an explicit platform and arch", () => {
    const packageRoot = "/repo/node_modules/@konvert7/klint";
    const packageJsonPath = "/repo/node_modules/@konvert7/klint-darwin-x64/package.json";
    const expectedBinary = join(
      "/repo/node_modules/@konvert7/klint-darwin-x64",
      "bin",
      "klint-rs"
    );

    const binaryPath = resolveNativePackageBinary({
      packageRoot,
      platform: "darwin",
      arch: "x64",
      resolvePackageJson(packageName) {
        expect(packageName).toBe("@konvert7/klint-darwin-x64");
        return packageJsonPath;
      },
      exists(path) {
        return path === expectedBinary;
      },
      readPackageJson() {
        return { version: "1.2.3" };
      },
    });

    expect(binaryPath).toBe(expectedBinary);
  });

  test("skips optional package when binary is missing", () => {
    const binaryPath = resolveNativePackageBinary({
      packageRoot: "/repo/node_modules/@konvert7/klint",
      platform: "darwin",
      arch: "arm64",
      resolvePackageJson() {
        return "/repo/node_modules/@konvert7/klint-darwin-arm64/package.json";
      },
      exists() {
        return false;
      },
    });

    expect(binaryPath).toBeUndefined();
  });
});
