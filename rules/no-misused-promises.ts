import ts from "typescript";
import { defineAstRule } from "../core/rule-helpers";

export const noMisusedPromises = defineAstRule({
  meta: {
    description:
      "Flags Promises passed where the type expects a non-thenable (e.g. an `if` condition, a `void` callback). The Promise's truthiness — not its resolved value — drives the branch.",
    examples: ["no-misused-promises: error"],
  },
  visit(node, ctx) {
    if (!ts.isCallExpression(node)) return;
    const sig = ctx.checker.getResolvedSignature(node);
    if (!sig) return;
    const params = sig.getParameters();
    for (let i = 0; i < node.arguments.length; i++) {
      const arg = node.arguments[i];
      if (!isAsyncFunction(arg)) continue;

      const param = params[Math.min(i, params.length - 1)];
      if (!param) continue;

      const paramType = ctx.checker.getTypeOfSymbolAtLocation(param, node);
      if (expectsSyncCallback(paramType, ctx.checker)) {
        ctx.report(
          arg,
          "Async function passed where a sync callback is expected — the caller cannot await it and errors will be silently lost."
        );
      }
    }
  },
});

function isAsyncFunction(
  node: ts.Node
): node is ts.ArrowFunction | ts.FunctionExpression {
  return (
    (ts.isArrowFunction(node) || ts.isFunctionExpression(node)) &&
    (node.modifiers?.some((m) => m.kind === ts.SyntaxKind.AsyncKeyword) ?? false)
  );
}

function expectsSyncCallback(type: ts.Type, checker: ts.TypeChecker): boolean {
  if (type.isUnion()) {
    const callable = type.types.filter(
      (t) => !(t.flags & ts.TypeFlags.Undefined) && !(t.flags & ts.TypeFlags.Null)
    );
    return callable.length > 0 && callable.every((t) => expectsSyncCallback(t, checker));
  }
  const sigs = type.getCallSignatures();
  if (sigs.length === 0) return false;
  return sigs.every((sig) => !returnsPromise(checker.getReturnTypeOfSignature(sig)));
}

function returnsPromise(type: ts.Type): boolean {
  const name = type.getSymbol()?.name;
  if (name === "Promise" || name === "PromiseLike") return true;
  if (type.isUnion()) return type.types.some(returnsPromise);
  return false;
}
