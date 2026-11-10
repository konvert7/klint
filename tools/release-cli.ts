export function replaceRequired(
  input: string,
  pattern: RegExp,
  replacement: string,
  path: string
): string {
  if (!pattern.test(input)) {
    fail(`Expected release metadata pattern ${pattern} in ${path}`);
  }
  return input.replace(pattern, replacement);
}

export function valueAfter(args: string[], name: string): string | undefined {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

export function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
