import ts from "typescript";
import { nearestFunctionIsAsync, walkAst } from "../core/ast";
import { relativeSlashPath } from "../core/paths";
import type { KlintRule } from "../core/types";

export const noSyncInAsync: KlintRule = {
  meta: {
    description:
      "Flags synchronous I/O (`readFileSync`, `execSync`, etc.) inside async functions — blocks the event loop when an async equivalent is available.",
    examples: [
      "no-sync-in-async: error",
      'no-sync-in-async:\n  severity: warn\n  include: ["src/server/**"]',
    ],
  },
  check({ files, root, fileContents }, violations) {
    for (const file of files) {
      const content = fileContents.get(file) ?? "";
      walkAst(file, content, (node, src) => {
        if (ts.isCallExpression(node)) {
          const callee = node.expression;
          let name: string | null = null;
          if (ts.isIdentifier(callee)) {
            name = callee.text;
          } else if (ts.isPropertyAccessExpression(callee)) {
            name = callee.name.text;
          }
          if (
            name?.endsWith("Sync") &&
            name !== "existsSync" &&
            nearestFunctionIsAsync(node)
          ) {
            const { line } = src.getLineAndCharacterOfPosition(node.getStart());
            violations.push({
              file: relativeSlashPath(root, file),
              line: line + 1,
              message: `${name}() blocks the event loop inside an async function — use the async equivalent from node:fs/promises.`,
            });
          }
        }
      });
    }
  },
};
