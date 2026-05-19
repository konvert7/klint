import ts from "typescript";
import { defineAstRule } from "../core/rule-helpers";

export const noOptionalChainOnNonNullable = defineAstRule({
  meta: {
    description:
      "Flags `?.` on receivers whose static type cannot be `null` or `undefined`. The optional chain is dead code and obscures the real shape.",
    examples: ["no-optional-chain-on-non-nullable: warn"],
  },
  shouldRun(program) {
    return program.getCompilerOptions().strictNullChecks === true;
  },
  visit(node, ctx) {
    const receiver = getOptionalChainReceiver(node);
    if (!receiver) return;
    const type = ctx.checker.getTypeAtLocation(receiver);
    if (!isNullable(type)) {
      ctx.report(
        node,
        "Optional chain (?.) on a non-nullable type — the receiver can never be null or undefined here. Use . to remove misleading dead code."
      );
    }
  },
});

function getOptionalChainReceiver(node: ts.Node): ts.Expression | undefined {
  if (
    (ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node)) &&
    node.questionDotToken
  ) {
    return node.expression;
  }
  if (ts.isCallExpression(node) && node.questionDotToken) {
    return node.expression;
  }
  return undefined;
}

function isNullable(type: ts.Type): boolean {
  if (
    type.flags &
    (ts.TypeFlags.Null | ts.TypeFlags.Undefined | ts.TypeFlags.Any | ts.TypeFlags.Unknown)
  )
    return true;
  if (type.isUnion()) return type.types.some(isNullable);
  return false;
}
