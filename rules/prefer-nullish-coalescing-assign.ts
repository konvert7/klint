import ts from "typescript";
import { walkAst } from "../core/ast";
import { relativeSlashPath } from "../core/paths";
import type { KlintRule } from "../core/types";

export const preferNullishCoalescingAssign: KlintRule = {
  meta: {
    description:
      "Flags explicit nullish assignment guards — `x ??= y` is the dedicated nullish assignment operator.",
    examples: ["sonar/prefer-nullish-coalescing-assign: warn"],
  },
  check({ files, root, fileContents }, violations) {
    for (const file of files) {
      const content = fileContents.get(file) ?? "";
      walkAst(file, content, (node, src) => {
        if (!ts.isIfStatement(node) || node.elseStatement !== undefined) return;

        const assignExpr = extractAssignment(node.thenStatement);
        if (!assignExpr) return;

        const xText = nullishGuardTarget(node.expression, src);
        if (!xText) return;
        if (assignExpr.left.getText(src) !== xText) return;

        const yText = assignExpr.right.getText(src);
        const { line: s } = src.getLineAndCharacterOfPosition(node.getStart());
        const { line: e } = src.getLineAndCharacterOfPosition(node.getEnd());
        const indent = getIndent(src, node);

        violations.push({
          file: relativeSlashPath(root, file),
          line: s + 1,
          message: `Prefer \`${xText} ??= ${yText}\` over explicit nullish guard assignment — ??= only assigns when null or undefined.`,
          fix: {
            startLine: s + 1,
            endLine: e + 1,
            replacement: `${indent}${xText} ??= ${yText};`,
          },
        });
      });
    }
  },
};

function extractAssignment(stmt: ts.Statement): ts.BinaryExpression | undefined {
  if (ts.isExpressionStatement(stmt)) {
    return isAssign(stmt.expression) ? stmt.expression : undefined;
  }
  if (ts.isBlock(stmt) && stmt.statements.length === 1) {
    const inner = stmt.statements[0];
    if (ts.isExpressionStatement(inner) && isAssign(inner.expression)) {
      return inner.expression;
    }
  }
  return undefined;
}

function isAssign(node: ts.Expression): node is ts.BinaryExpression {
  return (
    ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.EqualsToken
  );
}

function nullishGuardTarget(
  expression: ts.Expression,
  src: ts.SourceFile
): string | undefined {
  if (!ts.isBinaryExpression(expression)) return undefined;

  const direct = equalityNullishTarget(expression, src);
  if (direct) return direct.target;

  if (expression.operatorToken.kind !== ts.SyntaxKind.BarBarToken) return undefined;

  const left = equalityNullishTarget(expression.left, src);
  const right = equalityNullishTarget(expression.right, src);
  if (!left || !right || left.target !== right.target) return undefined;

  const values = new Set([left.value, right.value]);
  if (values.has("null") && values.has("undefined")) return left.target;
  return undefined;
}

function equalityNullishTarget(
  expression: ts.Expression,
  src: ts.SourceFile
): { target: string; value: "null" | "undefined" } | undefined {
  if (!ts.isBinaryExpression(expression)) return undefined;

  const operator = expression.operatorToken.kind;
  if (
    operator !== ts.SyntaxKind.EqualsEqualsToken &&
    operator !== ts.SyntaxKind.EqualsEqualsEqualsToken
  )
    return undefined;

  const left = expression.left.getText(src);
  const right = expression.right.getText(src);

  if (right === "null") return { target: left, value: "null" };
  if (right === "undefined") return { target: left, value: "undefined" };
  if (left === "null") return { target: right, value: "null" };
  if (left === "undefined") return { target: right, value: "undefined" };
  return undefined;
}

function getIndent(src: ts.SourceFile, node: ts.Node): string {
  const { line } = src.getLineAndCharacterOfPosition(node.getStart());
  const lineStart = src.getPositionOfLineAndCharacter(line, 0);
  return new RegExp(/^(\s*)/).exec(src.text.slice(lineStart, node.getStart()))?.[1] ?? "";
}
