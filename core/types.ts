interface ViolationFix {
  startLine: number;
  endLine: number;
  replacement: string;
}

export type Severity = "error" | "warn" | "off";

/** @lintignore */
export interface RuleOptions {
  severity?: Severity;
  include?: string[];
}

export type RuleConfigValue = Severity | RuleOptions;

export interface Violation {
  file: string;
  line: number;
  rule: string;
  message: string;
  severity: Severity;
  fix?: ViolationFix;
}

/** @lintignore */
export interface RuleContext {
  files: string[];
  root: string;
  fileContents: Map<string, string>;
}

/** Violation as emitted by a rule — rule name and severity are stamped by the runner. */
export type RawViolation = Omit<Violation, "rule" | "severity">;

export interface RuleMeta {
  /** One- or two-line description of what the rule catches. */
  description: string;
  /** Concrete YAML config snippets showing usage. Rendered as code blocks in docs. */
  examples?: string[];
}

export interface KlintRule {
  check: (ctx: RuleContext, violations: RawViolation[]) => void;
  meta?: RuleMeta;
}

/** A named bundle of rules with their default severities and implementations. */
export interface KlintPlugin {
  name: string;
  /** Default severity for each rule. Keys use the prefixed form e.g. "sonar/rule-name". */
  rules: Record<string, RuleConfigValue>;
  /** Rule implementations keyed by the same prefixed names. */
  implementations: Record<string, KlintRule>;
}

interface ArchImportRule {
  from: string | string[];
  deny?: string | string[];
  allow?: string | string[];
  "type-only"?: "allow";
  message?: string;
  severity?: Exclude<Severity, "off">;
}

interface ArchForbiddenRuleBase {
  in: string | string[];
  message: string;
  severity?: Exclude<Severity, "off">;
}

interface ArchForbiddenPatternRule extends ArchForbiddenRuleBase {
  pattern: string;
}

interface ArchForbiddenJsxRule extends ArchForbiddenRuleBase {
  "jsx-element": string | string[];
}

type ArchForbiddenRule = ArchForbiddenPatternRule | ArchForbiddenJsxRule;

interface ArchSingletonRuleBase {
  only: string;
  message: string;
  /** Limit scan to these files/layers. Defaults to all files. */
  in?: string | string[];
  severity?: Exclude<Severity, "off">;
}

interface ArchSingletonPatternRule extends ArchSingletonRuleBase {
  pattern: string;
}

interface ArchSingletonJsxRule extends ArchSingletonRuleBase {
  "jsx-element": string | string[];
}

type ArchSingletonRule = ArchSingletonPatternRule | ArchSingletonJsxRule;

interface ArchMaxLinesRule {
  limit: number;
  in: string | string[];
  message?: string;
  severity?: Exclude<Severity, "off">;
}

export interface ArchConfig {
  layers?: Record<string, string[]>;
  imports?: ArchImportRule[];
  forbidden?: ArchForbiddenRule[];
  singleton?: ArchSingletonRule[];
  maxLines?: ArchMaxLinesRule[];
}

export interface KlintConfig {
  root: string;
  include: string[];
  plugins?: string[];
  rules: Record<string, RuleConfigValue>;
  arch?: ArchConfig;
}

export const defineRule = (r: KlintRule): KlintRule => r;

/** @lintignore */
export const defineConfig = (c: KlintConfig): KlintConfig => c;
