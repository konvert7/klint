import { describe, expect, test } from "bun:test";
import packageJson from "../package.json";

const SONARJS_PIN_THAT_NEEDS_THE_MINIMATCH_OVERRIDE = "3.0.5";

function sonarjsOverride(): unknown {
  const overrides: Record<string, unknown> = packageJson.overrides ?? {};

  return overrides["eslint-plugin-sonarjs"];
}

describe("dependency overrides", () => {
  test("the sonarjs minimatch override goes away with the sonarjs 3.0.5 pin", () => {
    const sonarjs = packageJson.devDependencies["eslint-plugin-sonarjs"];
    const pinned = sonarjs === SONARJS_PIN_THAT_NEEDS_THE_MINIMATCH_OVERRIDE;

    expect(
      Boolean(sonarjsOverride()),
      `eslint-plugin-sonarjs is ${sonarjs}: delete overrides["eslint-plugin-sonarjs"] from package.json, then check bun audit --audit-level=high`
    ).toBe(pinned);
  });
});
