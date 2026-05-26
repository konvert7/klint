import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = new URL("..", import.meta.url);

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
});
