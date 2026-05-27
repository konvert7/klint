import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = new URL("..", import.meta.url);
const rootPath = fileURLToPath(root);

describe("Python wheel package scaffold", () => {
  test("declares the klint package and console command", () => {
    const pyproject = readFileSync(new URL("python/pyproject.toml", root), "utf-8");

    expect(pyproject).toContain('name = "klint"');
    expect(pyproject).toContain('version = "0.0.0"');
    expect(pyproject).toContain('klint = "klint.__main__:main"');
  });

  test("wrapper looks for the bundled platform binary", () => {
    const wrapper = readFileSync(new URL("python/klint/__main__.py", root), "utf-8");

    expect(wrapper).toContain('"klint-rs.exe" if os.name == "nt" else "klint-rs"');
    expect(wrapper).toContain('Path(__file__).resolve().parent / "_bin" / name');
    expect(wrapper).toContain("subprocess.run([str(binary), *sys.argv[1:]]");
  });

  test("staging and smoke scripts exist", () => {
    expect(existsSync(join(import.meta.dir, "stage-python-wheel.ts"))).toBe(true);
    expect(existsSync(join(import.meta.dir, "smoke-python-wheel.ts"))).toBe(true);
  });

  test("release workflow uses publishable platform wheel tags", () => {
    const workflow = readFileSync(
      new URL(".github/workflows/python-release.yml", root),
      "utf-8"
    );

    expect(workflow).toContain("python-platform-tag: manylinux_2_28_x86_64");
    expect(workflow).toContain("python-platform-tag: macosx_11_0_arm64");
    expect(workflow).toContain("python-platform-tag: macosx_10_13_x86_64");
    expect(workflow).toContain("python-platform-tag: win_amd64");
    expect(workflow).not.toContain("linux_x86_64");
  });

  test("wheel setup emits Python-version-agnostic platform wheels", () => {
    const setup = readFileSync(new URL("python/setup.py", root), "utf-8");

    expect(setup).toContain('os.environ.get("KLINT_PYTHON_PLAT_NAME")');
    expect(setup).toContain("self.plat_name = plat_name");
    expect(setup).toContain("def get_tag(self)");
    expect(setup).toContain("if not plat_name:");
    expect(setup).toContain('return "py3", "none", plat_name');
    expect(setup).not.toContain("cp313");
  });

  test("release prep stages TestPyPI package name and release version", () => {
    const fixture = mkdtempSync(join(tmpdir(), "klint-python-release-"));
    const pythonRoot = join(fixture, "python");
    mkdirSync(join(pythonRoot, "klint"), { recursive: true });
    writeFileSync(
      join(pythonRoot, "pyproject.toml"),
      '[project]\nname = "klint"\nversion = "0.0.0"\n'
    );
    writeFileSync(
      join(pythonRoot, "klint", "__init__.py"),
      '__all__ = ["__version__"]\n\n__version__ = "0.0.0"\n'
    );

    const result = spawnSync(
      "bun",
      [
        "tools/prepare-python-release.ts",
        "--root",
        fixture,
        "--name",
        "konvert7-klint",
        "--version",
        "1.2.3",
      ],
      { cwd: rootPath, encoding: "utf-8" }
    );

    expect(result.status, result.stderr || result.stdout).toBe(0);
    expect(readFileSync(join(pythonRoot, "pyproject.toml"), "utf-8")).toContain(
      'name = "konvert7-klint"\nversion = "1.2.3"'
    );
    expect(readFileSync(join(pythonRoot, "klint", "__init__.py"), "utf-8")).toContain(
      '__version__ = "1.2.3"'
    );
  });
});
