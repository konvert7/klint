import { relative } from "node:path";
import ts from "typescript";
import { walkAst } from "../core/ast";
import { buildNodeReplacementFix } from "../core/rule-helpers";
import type { KlintRule } from "../core/types";

export const preferStringRaw: KlintRule = {
  meta: {
    description:
      "Flags string literals with multiple backslash escapes — `String.raw` is more readable (Windows paths, regex source, etc.).",
    examples: ["sonar/prefer-string-raw: warn"],
  },
  check({ files, root, fileContents }, violations) {
    for (const file of files) {
      const content = fileContents.get(file) ?? "";
      walkAst(file, content, (node, src) => {
        if (!ts.isStringLiteral(node)) return;

        const sourceText = node.getText(src);
        if (!sourceText.includes("\\\\")) return;

        const value = node.text;
        // Can't embed backtick, ${ (would start interpolation), or trailing \ (would escape closing backtick)
        if (value.includes("`") || value.includes("${") || value.endsWith("\\")) return;

        const fixedNode = `String.raw\`${value}\``;
        const fix = buildNodeReplacementFix(src, node, fixedNode);

        violations.push({
          file: relative(root, file),
          line: fix.startLine,
          message:
            "String literal with escaped backslashes — use String.raw`...` for clarity (Sonar S6535).",
          fix,
        });
      });
    }
  },
};
