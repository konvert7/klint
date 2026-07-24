import ts from "typescript";
import { walkAst } from "../core/ast";
import { relativeSlashPath } from "../core/paths";
import { buildNodeReplacementFix } from "../core/rule-helpers";
import type { KlintRule } from "../core/types";

/**
 * Metacharacters that lose their special meaning inside a character class, so `[.]`
 * is a valid shorthand for a literal dot and must not be unwrapped to a bare `.`.
 */
const METACHAR_EXCEPTIONS = new Set([".", "*", "+", "?", "{", "}", "(", ")", "|", "$"]);

interface CharClass {
  bracketStart: number;
  bracketEnd: number;
  innerToken: string;
}

function parseCharClasses(pattern: string): CharClass[] {
  const results: CharClass[] = [];
  let i = 0;
  while (i < pattern.length) {
    if (pattern[i] === "\\") {
      i += 2;
      continue;
    }
    if (pattern[i] !== "[") {
      i++;
      continue;
    }
    const start = i++;
    // Negated classes have different semantics — skip
    if (i < pattern.length && pattern[i] === "^") {
      while (i < pattern.length && pattern[i] !== "]") {
        if (pattern[i] === "\\") i++;
        i++;
      }
      i++;
      continue;
    }
    const tokens: string[] = [];
    while (i < pattern.length && pattern[i] !== "]") {
      if (pattern[i] === "\\") {
        const s = i++;
        if (i >= pattern.length) break;
        const c = pattern[i];
        if (c === "u" && i + 5 <= pattern.length) {
          i += 5; // \uXXXX
        } else if (c === "x" && i + 3 <= pattern.length) {
          i += 3; // \xXX
        } else if (c === "c" && i + 1 < pattern.length) {
          i += 2; // \cX
        } else {
          i++;
        }
        tokens.push(pattern.slice(s, i));
      } else {
        tokens.push(pattern[i++]);
      }
    }
    if (i >= pattern.length) break;
    const end = i++;
    if (tokens.length === 1) {
      results.push({ bracketStart: start, bracketEnd: end, innerToken: tokens[0] });
    }
  }
  return results;
}

export const noSingleCharClass: KlintRule = {
  meta: {
    description:
      "Flags single-character character classes in regex (e.g. `/[a]/`) — drop the brackets; the regex matches the same.",
    examples: ["sonar/no-single-char-class: warn"],
  },
  check({ files, root, fileContents }, violations) {
    for (const file of files) {
      const content = fileContents.get(file) ?? "";
      walkAst(file, content, (node, src) => {
        if (!ts.isRegularExpressionLiteral(node)) return;

        const regexSrc = node.getText(src);
        const lastSlash = regexSrc.lastIndexOf("/");
        const pattern = regexSrc.slice(1, lastSlash);
        const flags = regexSrc.slice(lastSlash + 1);

        const allClasses = parseCharClasses(pattern);
        const toFix = allClasses.filter((c) => !METACHAR_EXCEPTIONS.has(c.innerToken));
        if (toFix.length === 0) return;

        let fixedPattern = "";
        let prev = 0;
        for (const cls of allClasses) {
          if (METACHAR_EXCEPTIONS.has(cls.innerToken)) continue;
          fixedPattern += pattern.slice(prev, cls.bracketStart);
          fixedPattern += cls.innerToken;
          prev = cls.bracketEnd + 1;
        }
        fixedPattern += pattern.slice(prev);

        const fixedRegex = `/${fixedPattern}/${flags}`;
        const fix = buildNodeReplacementFix(src, node, fixedRegex);

        violations.push({
          file: relativeSlashPath(root, file),
          line: fix.startLine,
          message: `Character class [${toFix[0].innerToken}] contains a single element — remove the brackets.`,
          fix,
        });
      });
    }
  },
};
