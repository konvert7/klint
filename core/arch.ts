import { dirname, isAbsolute, resolve } from "node:path";
import ts from "typescript";
import { walkAst } from "./ast";
import { relativeSlashPath, toSlashPath } from "./paths";
import type { ArchConfig, Severity, Violation } from "./types";

interface AliasEntry {
  /** The prefix to match (pattern with `/*` stripped, e.g. `"@"` from `"@/*"`). */
  prefix: string;
  /** Resolved absolute base directory (target with `/*` stripped). */
  base: string;
  isWildcard: boolean;
}

function loadPathAliases(root: string): AliasEntry[] {
  const tsconfigPath = resolve(root, "tsconfig.json");
  const parsed = ts.getParsedCommandLineOfConfigFile(tsconfigPath, undefined, {
    ...ts.sys,
    onUnRecoverableConfigFileDiagnostic: () => {},
  });
  if (!parsed) return [];
  const { paths, baseUrl } = parsed.options;
  if (!paths) return [];
  // baseUrl is absolute when set; fall back to tsconfig dir for TS 5+ pathless baseUrl
  const base = baseUrl ?? root;
  const entries: AliasEntry[] = [];
  for (const [pattern, targets] of Object.entries(paths)) {
    if (targets.length === 0) continue;
    const isWildcard = pattern.endsWith("/*");
    const prefix = isWildcard ? pattern.slice(0, -2) : pattern;
    const targetStr = targets[0];
    const targetBase = targetStr.endsWith("/*") ? targetStr.slice(0, -2) : targetStr;
    entries.push({ prefix, base: toSlashPath(resolve(base, targetBase)), isWildcard });
  }
  return entries;
}

function resolveAlias(importPath: string, aliases: AliasEntry[]): string | undefined {
  for (const alias of aliases) {
    if (alias.isWildcard) {
      const matchPrefix = `${alias.prefix}/`;
      if (importPath.startsWith(matchPrefix)) {
        return toSlashPath(resolve(alias.base, importPath.slice(matchPrefix.length)));
      }
    } else if (importPath === alias.prefix) {
      return alias.base;
    }
  }
  return undefined;
}

interface ImportRecord {
  path: string;
  resolved: string;
  isTypeOnly: boolean;
  line: number;
}

function isBareSpecifier(path: string): boolean {
  return !path.startsWith(".") && !isAbsolute(path);
}

function globToPrefix(glob: string, root: string): string {
  return toSlashPath(resolve(root, glob.split("/**")[0].split("/*")[0].split("*")[0]));
}

function resolveGlobs(
  ref: string | string[],
  layers: Record<string, string[]> | undefined
): string[] {
  const items = Array.isArray(ref) ? ref : [ref];
  return items.flatMap((item) => layers?.[item] ?? [item]);
}

function resolveLayerPrefixes(
  ref: string | string[],
  layers: Record<string, string[]> | undefined,
  root: string
): string[] {
  return resolveGlobs(ref, layers)
    .filter((g) => !g.startsWith("!"))
    .map((g) => globToPrefix(g, root));
}

function resolveLayerFiles(
  ref: string | string[],
  layers: Record<string, string[]> | undefined,
  root: string,
  allFiles: string[]
): string[] {
  const globs = resolveGlobs(ref, layers);
  const includePrefixes = globs
    .filter((g) => !g.startsWith("!"))
    .map((g) => globToPrefix(g, root));
  const excludePrefixes = globs
    .filter((g) => g.startsWith("!"))
    .map((g) => globToPrefix(g.slice(1), root));
  return allFiles.filter(
    (f) =>
      includePrefixes.some((p) => f === p || f.startsWith(`${p}/`)) &&
      !excludePrefixes.some((p) => f === p || f.startsWith(`${p}/`))
  );
}

function inPrefixes(absPath: string, prefixes: string[]): boolean {
  return prefixes.some((p) => {
    if (absPath === p || absPath.startsWith(`${p}/`)) return true;
    // Extension-less imports ("../cli") won't match a prefix like "{root}/cli.ts"
    const bare = p.replace(/\.(tsx?|jsx?|mts|cts)$/, "");
    return bare !== p && (absPath === bare || absPath.startsWith(`${bare}/`));
  });
}

function scanImports(
  file: string,
  content: string,
  aliases: AliasEntry[]
): ImportRecord[] {
  const records: ImportRecord[] = [];
  const fileDir = dirname(file);

  walkAst(file, content, (node, src) => {
    let specifierNode: ts.StringLiteral | undefined;
    let isTypeOnly = false;

    if (ts.isImportDeclaration(node)) {
      if (ts.isStringLiteral(node.moduleSpecifier)) {
        specifierNode = node.moduleSpecifier;
        isTypeOnly = node.importClause?.isTypeOnly ?? false;
      }
    } else if (
      ts.isCallExpression(node) &&
      node.expression.kind === ts.SyntaxKind.ImportKeyword &&
      node.arguments.length >= 1 &&
      ts.isStringLiteral(node.arguments[0])
    ) {
      specifierNode = node.arguments[0] as ts.StringLiteral;
    }

    if (!specifierNode) return;

    const path = specifierNode.text;
    let resolved: string;
    if (isBareSpecifier(path)) {
      resolved = resolveAlias(path, aliases) ?? path;
    } else {
      resolved = toSlashPath(resolve(fileDir, path));
    }
    const { line } = src.getLineAndCharacterOfPosition(specifierNode.getStart());
    records.push({ path, resolved, isTypeOnly, line: line + 1 });
  });

  return records;
}

export function runArchRules(
  arch: ArchConfig,
  allFiles: string[],
  fileContents: Map<string, string>,
  root: string
): Violation[] {
  const violations: Violation[] = [];
  const layers = arch.layers;
  const aliases = loadPathAliases(root);

  for (const rule of arch.imports ?? []) {
    const severity: Severity = rule.severity ?? "error";
    const fromFiles = resolveLayerFiles(rule.from, layers, root, allFiles);

    for (const file of fromFiles) {
      const content = fileContents.get(file);
      if (!content) continue;

      for (const imp of scanImports(file, content, aliases)) {
        if (isBareSpecifier(imp.resolved)) continue;
        if (rule["type-only"] === "allow" && imp.isTypeOnly) continue;

        const relFile = relativeSlashPath(root, file);

        if (rule.deny !== undefined) {
          const denyPrefixes = resolveLayerPrefixes(rule.deny, layers, root);
          if (inPrefixes(imp.resolved, denyPrefixes)) {
            violations.push({
              file: relFile,
              line: imp.line,
              message: rule.message ?? "Import crosses a denied boundary",
              rule: "arch/imports",
              severity,
            });
          }
        } else if (rule.allow !== undefined) {
          const allowPrefixes = resolveLayerPrefixes(rule.allow, layers, root);
          if (!inPrefixes(imp.resolved, allowPrefixes)) {
            violations.push({
              file: relFile,
              line: imp.line,
              message: rule.message ?? "Import is not in the allowed list",
              rule: "arch/imports",
              severity,
            });
          }
        }
      }
    }
  }

  for (const rule of arch.forbidden ?? []) {
    const severity: Severity = rule.severity ?? "error";
    const inFiles = resolveLayerFiles(rule.in, layers, root, allFiles);
    if ("jsx-element" in rule) {
      scanJsxElements(
        inFiles,
        toArray(rule["jsx-element"]),
        fileContents,
        root,
        "arch/forbidden",
        rule.message,
        severity,
        violations
      );
    } else {
      scanLinesForPattern(
        inFiles,
        rule.pattern,
        fileContents,
        root,
        "arch/forbidden",
        rule.message,
        severity,
        violations
      );
    }
  }

  for (const rule of arch.singleton ?? []) {
    const severity: Severity = rule.severity ?? "error";
    const onlyFile = toSlashPath(resolve(root, rule.only));
    const inFiles = rule.in
      ? resolveLayerFiles(rule.in, layers, root, allFiles)
      : allFiles;
    const scope = inFiles.filter((f) => f !== onlyFile);
    if ("jsx-element" in rule) {
      scanJsxElements(
        scope,
        toArray(rule["jsx-element"]),
        fileContents,
        root,
        "arch/singleton",
        rule.message,
        severity,
        violations
      );
    } else {
      scanLinesForPattern(
        scope,
        rule.pattern,
        fileContents,
        root,
        "arch/singleton",
        rule.message,
        severity,
        violations
      );
    }
  }

  return violations;
}

function toArray<T>(v: T | T[]): T[] {
  return Array.isArray(v) ? v : [v];
}

function scanJsxElements(
  files: string[],
  tagNames: string[],
  fileContents: Map<string, string>,
  root: string,
  ruleName: string,
  message: string,
  severity: Severity,
  violations: Violation[]
): void {
  const targets = new Set(tagNames);
  for (const file of files) {
    if (!/\.(tsx|jsx)$/.test(file)) continue;
    const content = fileContents.get(file);
    if (!content) continue;
    walkAst(file, content, (node, src) => {
      if (!ts.isJsxOpeningElement(node) && !ts.isJsxSelfClosingElement(node)) return;
      const tagName = node.tagName;
      if (!ts.isIdentifier(tagName)) return;
      if (!targets.has(tagName.text)) return;
      const { line } = src.getLineAndCharacterOfPosition(tagName.getStart());
      violations.push({
        file: relativeSlashPath(root, file),
        line: line + 1,
        message,
        rule: ruleName,
        severity,
      });
    });
  }
}

const REGEX_PREFIX = "re:";

function buildLineMatcher(pattern: string): (line: string) => boolean {
  if (!pattern.startsWith(REGEX_PREFIX)) {
    return (line) => line.includes(pattern);
  }
  const source = pattern.slice(REGEX_PREFIX.length);
  let regex: RegExp;
  try {
    regex = new RegExp(source);
  } catch (cause) {
    throw new Error(
      `klint: invalid regex in arch pattern "${pattern}": ${(cause as Error).message}`
    );
  }
  return (line) => regex.test(line);
}

function scanLinesForPattern(
  files: string[],
  pattern: string,
  fileContents: Map<string, string>,
  root: string,
  ruleName: string,
  message: string,
  severity: Severity,
  violations: Violation[]
): void {
  const matchesLine = buildLineMatcher(pattern);
  for (const file of files) {
    const content = fileContents.get(file);
    if (!content) continue;
    const lines = content.split("\n");
    for (let i = 0; i < lines.length; i++) {
      if (matchesLine(lines[i])) {
        violations.push({
          file: relativeSlashPath(root, file),
          line: i + 1,
          message,
          rule: ruleName,
          severity,
        });
      }
    }
  }
}
