import ts from "typescript";
import { walkAst } from "../core/ast";
import { relativeSlashPath } from "../core/paths";
import { buildNodeReplacementFix } from "../core/rule-helpers";
import type { KlintRule } from "../core/types";

export const preferStringReplaceall: KlintRule = {
  meta: {
    description:
      "Flags `.replace(/x/g, y)` on strings — `.replaceAll('x', y)` is clearer and faster for all-occurrence replacements.",
    examples: ["sonar/prefer-string-replaceall: error"],
  },
  check({ files, root, fileContents }, violations) {
    for (const file of files) {
      const content = fileContents.get(file) ?? "";
      walkAst(file, content, (node, src) => {
        if (
          !ts.isCallExpression(node) ||
          !ts.isPropertyAccessExpression(node.expression) ||
          node.expression.name.text !== "replace" ||
          node.arguments.length !== 2 ||
          !ts.isRegularExpressionLiteral(node.arguments[0])
        )
          return;

        const regexSrc = node.arguments[0].getText(src);
        const flags = extractFlags(regexSrc);
        const pattern = extractPattern(regexSrc);

        if (flags !== "g") return;
        if (!isPlainLiteral(pattern)) return;

        const strText = (
          node.expression as ts.PropertyAccessExpression
        ).expression.getText(src);
        const replacementText = node.arguments[1].getText(src);
        const patternLit = pattern.includes('"')
          ? `'${pattern.replaceAll("'", String.raw`\'`)}'`
          : `"${pattern}"`;
        const fixedCall = `${strText}.replaceAll(${patternLit}, ${replacementText})`;
        const fix = buildNodeReplacementFix(src, node, fixedCall);

        violations.push({
          file: relativeSlashPath(root, file),
          line: fix.startLine,
          message: `Prefer \`${strText}.replaceAll(${patternLit}, ...)\` over \`.replace(/${pattern}/g, ...)\` — replaceAll() with a string is clearer and avoids regex escaping pitfalls.`,
          fix,
        });
      });
    }
  },
};

function extractFlags(literal: string): string {
  const last = literal.lastIndexOf("/");
  return last > 0 ? literal.slice(last + 1) : "";
}

function extractPattern(literal: string): string {
  return literal.slice(1, literal.lastIndexOf("/"));
}

function isPlainLiteral(pattern: string): boolean {
  // Strip escaped-backslash pairs (\\) before checking — \\ is a plain literal, not a metachar
  const stripped = pattern.replaceAll("\\\\", "");
  return !/[.*+?[\]{}()|^$\\]/.test(stripped);
}
