import ts from "typescript";
import { defineAstRule } from "../core/rule-helpers";

// Builtins with a meaningful toString() that isn't [object Object]
const SAFE_SYMBOL_NAMES = new Set([
  "Date",
  "RegExp",
  "Error",
  "TypeError",
  "RangeError",
  "SyntaxError",
  "ReferenceError",
  "Array",
  "Map",
  "Set",
  "URL",
  "URLSearchParams",
  "Symbol",
]);

export const noObjectInTemplate = defineAstRule({
  meta: {
    description:
      "Flags non-primitive objects interpolated into template literals — produces `[object Object]` unless `toString()` is overridden.",
    examples: ["no-object-in-template: error"],
  },
  visit(node, ctx) {
    if (!ts.isTemplateExpression(node)) return;
    for (const span of node.templateSpans) {
      const type = ctx.checker.getTypeAtLocation(span.expression);
      if (wouldRenderAsObjectObject(type, ctx.checker)) {
        ctx.report(
          span.expression,
          "Object interpolated in template literal has no custom toString() — it will render as [object Object]. Access a specific property or implement toString()."
        );
      }
    }
  },
});

function wouldRenderAsObjectObject(type: ts.Type, checker: ts.TypeChecker): boolean {
  const primitiveFlags =
    ts.TypeFlags.String |
    ts.TypeFlags.Number |
    ts.TypeFlags.Boolean |
    ts.TypeFlags.BigInt |
    ts.TypeFlags.Null |
    ts.TypeFlags.Undefined |
    ts.TypeFlags.StringLiteral |
    ts.TypeFlags.NumberLiteral |
    ts.TypeFlags.BooleanLiteral |
    ts.TypeFlags.BigIntLiteral |
    ts.TypeFlags.Any |
    ts.TypeFlags.Unknown |
    ts.TypeFlags.Never |
    ts.TypeFlags.Void |
    ts.TypeFlags.ESSymbol |
    ts.TypeFlags.UniqueESSymbol |
    ts.TypeFlags.Enum |
    ts.TypeFlags.EnumLiteral;

  if (type.flags & primitiveFlags) return false;

  if (type.isUnion()) {
    const meaningful = type.types.filter(
      (t) => !(t.flags & (ts.TypeFlags.Null | ts.TypeFlags.Undefined))
    );
    return (
      meaningful.length > 0 &&
      meaningful.every((t) => wouldRenderAsObjectObject(t, checker))
    );
  }

  // Functions coerce to their source string — unusual but not [object Object]
  if (type.getCallSignatures().length > 0) return false;

  // Known builtins with meaningful toString
  const sym = type.getSymbol();
  if (sym && SAFE_SYMBOL_NAMES.has(sym.name)) return false;

  // Check for a custom toString not originating from TypeScript's built-in lib
  const toStringProp = checker.getPropertyOfType(type, "toString");
  if (!toStringProp) return true;

  const decls = toStringProp.getDeclarations() ?? [];
  const hasCustomToString = decls.some((d) => {
    const fileName = d.getSourceFile().fileName;
    return (
      !fileName.includes("/typescript/lib/lib") &&
      !fileName.includes("\\typescript\\lib\\lib")
    );
  });

  return !hasCustomToString;
}
