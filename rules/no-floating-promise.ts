import ts from "typescript";
import { defineAstRule } from "../core/rule-helpers";

export const noFloatingPromise = defineAstRule({
  meta: {
    description:
      "Flags Promise-returning expressions that aren't awaited or `.catch()`-handled — silent failures and unpredictable execution order.",
    examples: ["no-floating-promise: error"],
  },
  visit(node, ctx) {
    if (
      ts.isExpressionStatement(node) &&
      ts.isCallExpression(node.expression) &&
      !isHandled(node.expression)
    ) {
      const type = ctx.checker.getTypeAtLocation(node.expression);
      if (isPromiseLike(type)) {
        ctx.report(
          node,
          "Promise-returning call is not awaited — errors will be silently discarded and execution order is unpredictable. Use await, void, or .catch()."
        );
      }
    }
  },
});

function isHandled(node: ts.CallExpression): boolean {
  if (!ts.isPropertyAccessExpression(node.expression)) return false;
  const name = node.expression.name.text;
  return name === "catch" || name === "finally";
}

function isPromiseLike(type: ts.Type): boolean {
  if (type.getSymbol()?.name === "Promise") return true;
  if (type.isUnion()) return type.types.some(isPromiseLike);
  return false;
}
