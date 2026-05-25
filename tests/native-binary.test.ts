import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import {
  nativePackageForPlatform,
  nativePackages,
  resolveNativePackageBinary,
} from "../core/native-binary";

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
