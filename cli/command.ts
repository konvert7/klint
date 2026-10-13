interface OptionSpec {
  description: string;
  value?: string;
  short?: string;
  aliases?: string[];
}

type OptionSpecs = Record<string, OptionSpec>;
export type OptionValues = Record<string, string | true | undefined>;

interface Leaf {
  kind: "leaf";
  summary: string;
  options: OptionSpecs;
  details?: () => string[];
  run(values: OptionValues): unknown;
}

export interface Group {
  kind: "group";
  summary: string;
  commands: Record<string, Leaf>;
  aliases?: Record<string, string>;
  defaultCommand?: Leaf;
  details?: () => string[];
}

export type Command = Leaf | Group;

const HELP_FLAGS = new Set(["-h", "--help"]);
const HELP_WORD = "help";
const HELP_OPTION: OptionSpec = { short: "h", description: "Show this help" };

interface ParsedOptions {
  values: OptionValues;
  positionals: string[];
  help: boolean;
  error?: string;
  rest: string[];
}

function findOption(
  options: OptionSpecs,
  token: string
): [string, OptionSpec] | undefined {
  return Object.entries(options).find(
    ([name, spec]) =>
      token === `--${name}` ||
      (spec.short !== undefined && token === `-${spec.short}`) ||
      (spec.aliases?.includes(token) ?? false)
  );
}

function parseOptions(
  args: string[],
  options: OptionSpecs,
  stopAtPositional: boolean
): ParsedOptions {
  const parsed: ParsedOptions = { values: {}, positionals: [], help: false, rest: [] };
  for (let i = 0; i < args.length; i++) {
    const token = args[i];
    const match = findOption(options, token);
    if (HELP_FLAGS.has(token)) {
      parsed.help = true;
    } else if (match?.[1].value !== undefined) {
      const [name, spec] = match;
      const value = args[i + 1];
      if (value === undefined) parsed.error ??= `option '--${name}' needs ${spec.value}`;
      else parsed.values[name] = value;
      i++;
    } else if (match) {
      parsed.values[match[0]] = true;
    } else if (token.startsWith("-")) {
      parsed.error ??= `unknown option '${token}'`;
    } else if (stopAtPositional) {
      parsed.positionals.push(token);
      parsed.rest = args.slice(i + 1);
      return parsed;
    } else {
      parsed.positionals.push(token);
    }
  }
  return parsed;
}

export async function runCommand(
  command: Command,
  args: string[],
  path: string[]
): Promise<number> {
  return command.kind === "group"
    ? runGroup(command, args, path)
    : runLeaf(command, args, helpText(command, path));
}

async function runGroup(group: Group, args: string[], path: string[]): Promise<number> {
  const help = helpText(group, path);
  const parsed = parseOptions(args, group.defaultCommand?.options ?? {}, true);
  const [name] = parsed.positionals;
  if (parsed.help || name === HELP_WORD || group.aliases?.[name] === HELP_WORD) {
    return showHelp(help);
  }
  if (name !== undefined) {
    const sub = group.commands[group.aliases?.[name] ?? name];
    if (!sub) return usageFailure(`unknown command '${name}'`, help);
    if (parsed.error) return usageFailure(parsed.error, help);
    return runCommand(sub, parsed.rest, [...path, name]);
  }
  if (!group.defaultCommand) return showHelp(help);
  return runLeaf(group.defaultCommand, args, help);
}

async function runLeaf(leaf: Leaf, args: string[], help: string): Promise<number> {
  const parsed = parseOptions(args, leaf.options, false);
  if (parsed.help || parsed.positionals.includes(HELP_WORD)) return showHelp(help);
  if (parsed.error) return usageFailure(parsed.error, help);
  const [extra] = parsed.positionals;
  if (extra !== undefined) return usageFailure(`unexpected argument '${extra}'`, help);
  const result = await leaf.run(parsed.values);
  return typeof result === "number" ? result : 0;
}

function showHelp(help: string): number {
  process.stdout.write(`${help}\n`);
  return 0;
}

function usageFailure(message: string, help: string): number {
  process.stderr.write(`error: ${message}\n\n${help}\n`);
  return 1;
}

function helpText(command: Command, path: string[]): string {
  return command.kind === "group" ? groupHelp(command, path) : leafHelp(command, path);
}

function optionLabel(name: string, spec: OptionSpec): string {
  const short = spec.short ? `-${spec.short}, ` : "";
  const value = spec.value ? ` ${spec.value}` : "";
  return `${short}--${name}${value}`;
}

function optionRows(options: OptionSpecs): string[] {
  return table(
    Object.entries({ ...options, help: HELP_OPTION }).map(([name, spec]) => [
      optionLabel(name, spec),
      spec.description,
    ])
  );
}

function leafHelp(leaf: Leaf, path: string[]): string {
  return [
    `Usage: ${path.join(" ")} [options]`,
    "",
    `  ${leaf.summary}`,
    "",
    "Options:",
    ...optionRows(leaf.options),
    ...detailLines(leaf.details),
  ].join("\n");
}

function groupHelp(group: Group, path: string[]): string {
  const fallback = group.defaultCommand;
  const synopsis = fallback ? `[<command>] [options]` : "<command>";
  return [
    `Usage: ${path.join(" ")} ${synopsis}`,
    "",
    `  ${group.summary}`,
    "",
    "Commands:",
    ...table(Object.entries(group.commands).map(([name, sub]) => [name, sub.summary])),
    ...(fallback ? ["", "Options:", ...optionRows(fallback.options)] : []),
    "",
    ...(fallback ? [`With no command, ${fallback.summary.toLowerCase()}.`] : []),
    `Run '${path.join(" ")} <command> --help' for its options.`,
    ...detailLines(group.details),
  ].join("\n");
}

function detailLines(details: (() => string[]) | undefined): string[] {
  return details ? ["", ...details()] : [];
}

function table(rows: string[][]): string[] {
  const width = Math.max(...rows.map(([label]) => label.length));
  return rows.map(([label, text]) => `  ${label.padEnd(width)}  ${text}`);
}
