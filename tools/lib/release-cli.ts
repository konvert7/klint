export function replaceRequired(
  input: string,
  pattern: RegExp,
  replacement: string,
  path: string
): string {
  if (!pattern.test(input)) {
    throw new Error(`Expected release metadata pattern ${pattern} in ${path}`);
  }

  return input.replace(pattern, replacement);
}

export function valueAfter(args: string[], name: string): string | undefined {
  const index = args.indexOf(name);

  return index >= 0 ? args[index + 1] : undefined;
}
