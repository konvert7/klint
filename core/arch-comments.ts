import ts from "typescript";

interface CommentSpan {
  startLine: number;
  endLine: number;
  isDoc: boolean;
}

/** JS-like sources are lexed for comments; Python/Swift comment rules run in the native engine. */
function collectComments(file: string, content: string): CommentSpan[] {
  if (!/\.(tsx?|jsx?|mts|cts)$/.test(file)) return [];
  const variant = /\.(tsx|jsx)$/.test(file)
    ? ts.LanguageVariant.JSX
    : ts.LanguageVariant.Standard;
  const scanner = ts.createScanner(ts.ScriptTarget.Latest, false, variant, content);
  const starts = lineStarts(content);
  const spans: CommentSpan[] = [];
  let token = scanner.scan();
  while (token !== ts.SyntaxKind.EndOfFileToken) {
    if (
      token === ts.SyntaxKind.SingleLineCommentTrivia ||
      token === ts.SyntaxKind.MultiLineCommentTrivia
    ) {
      const start = scanner.getTokenStart();
      const end = scanner.getTokenEnd();
      spans.push({
        startLine: lineAt(starts, start),
        endLine: lineAt(starts, Math.max(start, end - 1)),
        isDoc: isDocComment(content.slice(start, end)),
      });
    }
    token = scanner.scan();
  }
  return spans;
}

/** A `/** *​/` JSDoc block, but not the empty `/**​/` comment. Mirrors the Rust engine. */
function isDocComment(text: string): boolean {
  return text.startsWith("/**") && text !== "/**/";
}

export function commentLineSet(
  file: string,
  content: string,
  countDocComments: boolean
): Set<number> {
  const set = new Set<number>();
  for (const span of collectComments(file, content)) {
    if (!countDocComments && span.isDoc) continue;
    for (let line = span.startLine; line <= span.endLine; line++) set.add(line);
  }
  return set;
}

/** Returns the first line at which a run of consecutive comment lines exceeds `limit`. */
export function firstCommentBlockOverrun(
  sortedLines: number[],
  limit: number
): number | undefined {
  let runStart = sortedLines[0];
  let prev = sortedLines[0];
  for (let i = 1; i <= sortedLines.length; i++) {
    const line = sortedLines[i];
    if (line === prev + 1) {
      prev = line;
      continue;
    }
    if (prev - runStart + 1 > limit) return runStart + limit;
    runStart = line;
    prev = line;
  }
  return undefined;
}

function lineStarts(content: string): number[] {
  const starts = [0];
  for (let i = 0; i < content.length; i++) {
    if (content[i] === "\n") starts.push(i + 1);
  }
  return starts;
}

/** 1-based line for a character offset, via binary search over line starts. */
function lineAt(starts: number[], offset: number): number {
  let lo = 0;
  let hi = starts.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (starts[mid] <= offset) lo = mid;
    else hi = mid - 1;
  }
  return lo + 1;
}
