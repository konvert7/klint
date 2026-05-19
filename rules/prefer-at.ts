import { relative } from "node:path";
import ts from "typescript";
import { walkAst } from "../core/ast";
import { buildNodeReplacementFix } from "../core/rule-helpers";
import type { KlintRule } from "../core/types";

export const preferAt: KlintRule = {
  meta: {
    description:
      "Flags negative-index access via `arr[arr.length - 1]` — `.at(-1)` reads cleaner for last-element access.",
    examples: ["sonar/prefer-at: warn"],
  },
  check({ files, root, fileContents }, violations) {
    for (const file of files) {
      const content = fileContents.get(file) ?? "";
      walkAst(file, content, (node, src) => {
        // Match: base[base.length - n]
        if (!ts.isElementAccessExpression(node)) return;

        const arg = node.argumentExpression;
        if (!ts.isBinaryExpression(arg)) return;
        if (arg.operatorToken.kind !== ts.SyntaxKind.MinusToken) return;
        if (!ts.isPropertyAccessExpression(arg.left)) return;
        if (arg.left.name.text !== "length") return;
        if (!ts.isNumericLiteral(arg.right)) return;

        const n = Number(arg.right.text);
        // n=0 changes semantics: arr[arr.length - 0] is undefined, arr.at(-0) is arr[0]
        if (!Number.isInteger(n) || n <= 0) return;

        const baseText = node.expression.getText(src);
        if (baseText !== arg.left.expression.getText(src)) return;

        const fixedCall = `${baseText}.at(-${n})`;
        const fix = buildNodeReplacementFix(src, node, fixedCall);

        violations.push({
          file: relative(root, file),
          line: fix.startLine,
          message: `Prefer ${baseText}.at(-${n}) over ${baseText}[${baseText}.length - ${n}] for cleaner negative indexing.`,
          fix,
        });
      });
    }
  },
};
