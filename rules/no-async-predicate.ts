import ts from "typescript";
import { defineAstRule } from "../core/rule-helpers";

const PREDICATE_METHODS = new Set(["filter", "some", "every", "find", "findIndex"]);

export const noAsyncPredicate = defineAstRule({
  meta: {
    description:
      "Flags `async` functions passed to `.filter`/`.some`/`.every`/`.find`/`.findIndex`. Array methods coerce the returned promise to truthy, so every element appears to match.",
    examples: ["no-async-predicate: error"],
  },
  visit(node, ctx) {
    if (
      !ts.isCallExpression(node) ||
      !ts.isPropertyAccessExpression(node.expression) ||
      !PREDICATE_METHODS.has(node.expression.name.text) ||
      node.arguments.length === 0
    )
      return;

    const receiverType = ctx.checker.getTypeAtLocation(node.expression.expression);
    if (isArrayLike(receiverType) && isAsyncFunction(node.arguments[0])) {
      const method = node.expression.name.text;
      ctx.report(
        node.arguments[0],
        `Async callback passed to .${method}() — the returned Promise is always truthy, so the predicate never filters correctly. The array method cannot await it.`
      );
    }
  },
});

function isArrayLike(type: ts.Type): boolean {
  if (type.isUnion()) return type.types.some((t) => isArrayLike(t));
  const sym = type.getSymbol();
  if (sym?.name === "Array" || sym?.name === "ReadonlyArray") return true;
  const ref = type as ts.TypeReference;
  if (ref.target) {
    const targetSym = ref.target.getSymbol();
    if (targetSym?.name === "Array" || targetSym?.name === "ReadonlyArray") return true;
  }
  return false;
}

function isAsyncFunction(
  node: ts.Node
): node is ts.ArrowFunction | ts.FunctionExpression {
  return (
    (ts.isArrowFunction(node) || ts.isFunctionExpression(node)) &&
    (node.modifiers?.some((m) => m.kind === ts.SyntaxKind.AsyncKeyword) ?? false)
  );
}
