import ts from "typescript";
import { createProgram } from "./ast";
import { relativeSlashPath } from "./paths";
import type { KlintRule, RawViolation, RuleMeta } from "./types";
import { defineRule } from "./types";

export interface AstRuleContext {
  sourceFile: ts.SourceFile;
  checker: ts.TypeChecker;
  root: string;
  report(node: ts.Node, message: string): void;
}

export function defineAstRule(opts: {
  meta: RuleMeta;
  shouldRun?: (program: ts.Program) => boolean;
  visit: (node: ts.Node, ctx: AstRuleContext) => void;
}): KlintRule {
  return defineRule({
    meta: opts.meta,
    check({ files, root }, violations) {
      const program = createProgram(files, root);
      if (opts.shouldRun && !opts.shouldRun(program)) return;
      const checker = program.getTypeChecker();
      const fileSet = new Set(files);

      for (const sourceFile of program.getSourceFiles()) {
        if (!fileSet.has(sourceFile.fileName) || sourceFile.isDeclarationFile) continue;

        const ctx: AstRuleContext = {
          sourceFile,
          checker,
          root,
          report(node, message) {
            const { line } = sourceFile.getLineAndCharacterOfPosition(node.getStart());
            violations.push({
              file: relativeSlashPath(root, sourceFile.fileName),
              line: line + 1,
              message,
            });
          },
        };

        const walk = (node: ts.Node): void => {
          opts.visit(node, ctx);
          ts.forEachChild(node, walk);
        };
        walk(sourceFile);
      }
    },
  });
}

/** Builds a single-node replacement fix preserving surrounding line text. */
export function buildNodeReplacementFix(
  src: ts.SourceFile,
  node: ts.Node,
  replacement: string
): NonNullable<RawViolation["fix"]> {
  const { line: startLine } = src.getLineAndCharacterOfPosition(node.getStart());
  const { line: endLine } = src.getLineAndCharacterOfPosition(node.getEnd());
  const lineStarts = src.getLineStarts();
  const lineStart = lineStarts[startLine];
  const lineEnd =
    endLine + 1 < lineStarts.length ? lineStarts[endLine + 1] - 1 : src.text.length;
  const linesText = src.text.slice(lineStart, lineEnd);
  const nodeOffset = node.getStart() - lineStart;
  const nodeEndOffset = node.getEnd() - lineStart;
  const fixedLines =
    linesText.slice(0, nodeOffset) + replacement + linesText.slice(nodeEndOffset);
  return {
    startLine: startLine + 1,
    endLine: endLine + 1,
    replacement: fixedLines,
  };
}
