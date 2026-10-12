import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { parse } from "yaml";

const WORKFLOWS_DIR = new URL("../.github/workflows/", import.meta.url);
const SEMANTIC_RELEASE_TAGS_ONLY =
  "github.event_name != 'release' || startsWith(github.event.release.tag_name, 'v')";
const PUBLISH_JOBS = ["github-release", "nuget", "python"];

interface Step {
  run?: string;
  uses?: string;
  with?: Record<string, string>;
}

interface Workflow {
  on: { release?: { types: string[] }; workflow_dispatch?: unknown };
  jobs: Record<string, { if?: string; needs?: string[]; steps: Step[] }>;
}

function workflow(name: string): Workflow {
  return parse(readFileSync(new URL(`${name}.yml`, WORKFLOWS_DIR), "utf-8")) as Workflow;
}

function compilesKlint(step: Step): boolean {
  return step.run?.includes("cargo build -p klint-rs") ?? false;
}

const packages = workflow("release-packages");

test("release-packages is the only workflow semantic-release releases trigger", () => {
  const triggered = readdirSync(WORKFLOWS_DIR)
    .map((file) => file.replace(/\.yml$/, ""))
    .filter((name) => workflow(name).on.release);
  expect(triggered).toEqual(["release-packages"]);
  expect(packages.on.release?.types).toEqual(["published"]);
});

test("release-packages can still be run by hand", () => {
  expect(packages.on.workflow_dispatch).toBeDefined();
});

test("release-packages ignores releases that are not semantic-release vX tags", () => {
  expect(packages.jobs.metadata.if).toBe(SEMANTIC_RELEASE_TAGS_ONLY);
});

test("release-packages reads event values from env, not by interpolating them into the script", () => {
  const scripts = packages.jobs.metadata.steps.map((step) => step.run ?? "");
  for (const script of scripts) expect(script).not.toContain("${{");
});

test("only the build job compiles klint; publish jobs reuse its binaries", () => {
  const compiling = Object.entries(packages.jobs)
    .filter(([, job]) => job.steps.some(compilesKlint))
    .map(([name]) => name);
  expect(compiling).toEqual(["build"]);

  for (const name of PUBLISH_JOBS) {
    const job = packages.jobs[name];
    expect(job.needs).toContain("build");
    expect(
      job.steps.some((step) => step.uses?.startsWith("actions/download-artifact"))
    ).toBe(true);
  }
});

test("native archives attach to the semantic-release vX release", () => {
  const source = readFileSync(new URL("release-packages.yml", WORKFLOWS_DIR), "utf-8");
  expect(source).toContain('tag="v$version"');
  expect(source).not.toContain("native-v");
  expect(source).not.toContain("gh release create");
});

test("semantic-release publishes the crate with a crates.io trusted publishing token", () => {
  const ci = parse(
    readFileSync(new URL("../.github/workflows/ci.yml", import.meta.url), "utf-8")
  ) as {
    jobs: {
      release: {
        permissions: Record<string, string>;
        steps: Array<{
          id?: string;
          uses?: string;
          run?: string;
          env?: Record<string, string>;
        }>;
      };
    };
  };
  const { permissions, steps } = ci.jobs.release;
  const auth = steps.find((step) => step.uses === "rust-lang/crates-io-auth-action@v1");
  const release = steps.find((step) => step.run === "bunx semantic-release");

  expect(permissions["id-token"]).toBe("write");
  expect(auth?.id).toBe("crates-auth");
  expect(release?.env?.CARGO_REGISTRY_TOKEN).toContain("steps.crates-auth.outputs.token");
});
