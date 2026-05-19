import ts from "typescript";
import { defineAstRule } from "../core/rule-helpers";

export const noDateEquality = defineAstRule({
  meta: {
    description:
      "Flags `Date` comparisons with `==`/`===`/`!=`/`!==` — these compare object identity, not the time value. Use `.getTime()` instead.",
    examples: ["no-date-equality: error"],
  },
  visit(node, ctx) {
    if (!ts.isBinaryExpression(node)) return;
    const { kind } = node.operatorToken;
    if (
      kind !== ts.SyntaxKind.EqualsEqualsEqualsToken &&
      kind !== ts.SyntaxKind.ExclamationEqualsEqualsToken
    )
      return;

    const leftType = ctx.checker.getTypeAtLocation(node.left);
    const rightType = ctx.checker.getTypeAtLocation(node.right);
    if (isDateType(leftType) && isDateType(rightType)) {
      const op = node.operatorToken.getText(ctx.sourceFile);
      ctx.report(
        node,
        `Date values compared with ${op} — this compares object references, not time values. Use .getTime() or .valueOf() instead.`
      );
    }
  },
});

function isDateType(type: ts.Type): boolean {
  if (type.getSymbol()?.name === "Date") return true;
  if (type.isUnion()) return type.types.some(isDateType);
  return false;
}
