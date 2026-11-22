import { afterAll, describe, expect, test } from "bun:test";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import {
  currentLibc,
  nativePackageForPlatform,
  nativePackages,
  resolveNativePackageBinary,
} from "../tools/native-binary";

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
    expect(nativePackageForPlatform("linux", "x64", "glibc")).toMatchObject({
      packageName: "@konvert7/klint-linux-x64",
      binaryPath: "bin/klint-rs",
    });
    expect(nativePackageForPlatform("linux", "x64", "musl")).toMatchObject({
      packageName: "@konvert7/klint-linux-x64-musl",
      binaryPath: "bin/klint-rs",
    });
    expect(nativePackageForPlatform("linux", "arm64", "glibc")).toMatchObject({
      packageName: "@konvert7/klint-linux-arm64",
      binaryPath: "bin/klint-rs",
    });
    expect(nativePackageForPlatform("win32", "x64")).toMatchObject({
      packageName: "@konvert7/klint-win32-x64",
      binaryPath: "bin/klint-rs.exe",
    });
  });

  test("serves the x64 package to Windows on Arm, which emulates x64", () => {
    expect(nativePackageForPlatform("win32", "arm64")).toMatchObject({
      packageName: "@konvert7/klint-win32-x64",
      binaryPath: "bin/klint-rs.exe",
    });
  });

  test("does not invent unsupported platform packages", () => {
    expect(nativePackageForPlatform("linux", "arm64", "musl")).toBeUndefined();
    expect(nativePackageForPlatform("freebsd", "x64")).toBeUndefined();
  });

  test("declares libc on every linux package so npm installs only the matching one", () => {
    for (const nativePackage of nativePackages()) {
      const packageJson = JSON.parse(
        readFileSync(
          new URL(`npm/native/${nativePackage.dir}/package.json`, root),
          "utf-8"
        )
      ) as { libc?: unknown };
      const expected =
        nativePackage.platform === "linux" ? [nativePackage.libc] : undefined;
      expect(packageJson.libc).toEqual(expected);
    }
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

    for (const packageName of expectedNames) {
      expect(optionalDependencies[packageName]).toBe("*");
    }
  });

  test("native packages remain private in source before release prep", () => {
    for (const nativePackage of nativePackages()) {
      const packageJson = readPackageJson(
        new URL(`npm/native/${nativePackage.dir}/package.json`, root)
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

describe("currentLibc", () => {
  test("is undefined off Linux and never reads the report there", () => {
    expect(
      currentLibc("darwin", () => {
        throw new Error("read");
      })
    ).toBeUndefined();
  });

  test("tells glibc from musl by the report's glibc runtime", () => {
    expect(
      currentLibc("linux", () => ({ header: { glibcVersionRuntime: "2.39" } }))
    ).toBe("glibc");
    expect(currentLibc("linux", () => ({ header: {} }))).toBe("musl");
    expect(currentLibc("linux", () => ({}))).toBe("musl");
  });
});

describe("nativePackageForPlatform, edge cases", () => {
  test("every package's folder names its platform, arch and libc", () => {
    for (const pkg of nativePackages()) {
      const musl = pkg.libc === "musl" ? "-musl" : "";
      expect(pkg.dir).toBe(`${pkg.platform}-${pkg.arch}${musl}`);
    }
  });

  test("adds the musl suffix only on Linux", () => {
    expect(nativePackageForPlatform("darwin", "arm64", "musl")?.dir).toBe("darwin-arm64");
  });

  test("only a package for the same platform can serve as an emulator", () => {
    expect(nativePackageForPlatform("freebsd", "arm64", undefined)).toBeUndefined();
  });
});

describe("resolveNativePackageBinary, edge cases", () => {
  const scratch = realpathSync(mkdtempSync(join(tmpdir(), "klint-native-binary-")));

  afterAll(() => {
    rmSync(scratch, { recursive: true, force: true });
  });

  function writeJson(path: string, value: unknown): void {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, typeof value === "string" ? value : JSON.stringify(value));
  }

  function installed(rootVersion: unknown, nativeJson: unknown): string {
    const project = mkdtempSync(join(scratch, "project-"));
    const packageRoot = join(project, "node_modules", "@konvert7", "klint");
    const nativeRoot = join(project, "node_modules", "@konvert7", "klint-linux-x64");
    writeJson(join(packageRoot, "package.json"), {
      name: "@konvert7/klint",
      ...(rootVersion as object),
    });
    writeJson(join(nativeRoot, "package.json"), nativeJson);
    writeJson(join(nativeRoot, "bin", "klint-rs"), "");

    return packageRoot;
  }

  const linux = { platform: "linux", arch: "x64", libc: "glibc" };
  const nativeBinaryOf = (packageRoot: string) =>
    join(dirname(packageRoot), "klint-linux-x64", "bin", "klint-rs");

  test("finds the installed package through node resolution and reads real package.json files", () => {
    const packageRoot = installed(
      { version: "1.2.3" },
      { name: "@konvert7/klint-linux-x64", version: "1.2.3" }
    );

    expect(resolveNativePackageBinary({ packageRoot, ...linux })).toBe(
      nativeBinaryOf(packageRoot)
    );
  });

  test("rejects an installed package from another release", () => {
    const packageRoot = installed(
      { version: "1.2.3" },
      { name: "@konvert7/klint-linux-x64", version: "1.2.2" }
    );

    expect(resolveNativePackageBinary({ packageRoot, ...linux })).toBeUndefined();
  });

  test("accepts a package whose version cannot be read", () => {
    const packageRoot = installed({ version: "1.2.3" }, "{ not json");

    expect(resolveNativePackageBinary({ packageRoot, ...linux })).toBe(
      nativeBinaryOf(packageRoot)
    );
  });

  test("accepts versions missing on either side", () => {
    const packageRoot = installed(
      {},
      { name: "@konvert7/klint-linux-x64", version: "1.2.3" }
    );
    const options = { packageRoot, ...linux };

    expect(resolveNativePackageBinary(options)).toBe(nativeBinaryOf(packageRoot));
    expect(
      resolveNativePackageBinary({
        ...options,
        readPackageJson: (path) =>
          path === join(packageRoot, "package.json") ? { version: "1.2.3" } : {},
      })
    ).toBe(nativeBinaryOf(packageRoot));
  });

  test("accepts the package when reading a version throws", () => {
    const packageRoot = installed({ version: "1.2.3" }, { version: "1.2.2" });

    expect(
      resolveNativePackageBinary({
        packageRoot,
        ...linux,
        readPackageJson: () => {
          throw new Error("unreadable");
        },
      })
    ).toBe(nativeBinaryOf(packageRoot));
  });

  test("is undefined when the package is not installed or the platform has none", () => {
    const packageRoot = join(scratch, "empty", "node_modules", "@konvert7", "klint");

    expect(resolveNativePackageBinary({ packageRoot, ...linux })).toBeUndefined();
    expect(
      resolveNativePackageBinary({
        packageRoot,
        ...linux,
        resolvePackageJson: () => {
          throw new Error("not installed");
        },
      })
    ).toBeUndefined();
    expect(
      resolveNativePackageBinary({ packageRoot, platform: "aix", arch: "ppc64" })
    ).toBeUndefined();
  });
});
