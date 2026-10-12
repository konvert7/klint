import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { parse } from "yaml";

const RELEASE_WORKFLOWS = ["native-artifacts", "nuget-release", "python-release"];
const SEMANTIC_RELEASE_TAGS_ONLY =
  "github.event_name != 'release' || startsWith(github.event.release.tag_name, 'v')";

interface Workflow {
  on: { release?: { types: string[] }; workflow_dispatch?: unknown };
  jobs: { metadata: { if?: string; steps: Array<{ run?: string }> } };
}

function workflow(name: string): Workflow {
  const url = new URL(`../.github/workflows/${name}.yml`, import.meta.url);
  return parse(readFileSync(url, "utf-8")) as Workflow;
}

describe.each(RELEASE_WORKFLOWS)("%s workflow", (name) => {
  test("runs when semantic-release publishes a release", () => {
    expect(workflow(name).on.release?.types).toEqual(["published"]);
  });

  test("can still be run by hand", () => {
    expect(workflow(name).on.workflow_dispatch).toBeDefined();
  });

  test("ignores releases that are not semantic-release vX tags", () => {
    expect(workflow(name).jobs.metadata.if).toBe(SEMANTIC_RELEASE_TAGS_ONLY);
  });

  test("reads event values from env, not by interpolating them into the script", () => {
    const scripts = workflow(name).jobs.metadata.steps.map((step) => step.run ?? "");
    for (const script of scripts) expect(script).not.toContain("${{");
  });
});

test("native artifacts attach to the semantic-release vX release", () => {
  const source = readFileSync(
    new URL("../.github/workflows/native-artifacts.yml", import.meta.url),
    "utf-8"
  );
  expect(source).toContain('tag="v$version"');
  expect(source).not.toContain("native-v");
  expect(source).not.toContain("gh release create");
});
