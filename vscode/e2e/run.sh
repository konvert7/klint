#!/bin/sh
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
klint=${KLINT_BINARY:-$here/../target/debug/klint-rs}
code=${VSCODE_BINARY:-/usr/share/code/code}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/project/src" "$work/project/.vscode"
cat > "$work/project/klint.yaml" <<'YAML'
include: ["src"]
rules:
  core:
    no-string-match: error
arch:
  forbidden:
    - {pattern: "console.log(", in: "src/**", message: "Use logger"}
    - {pattern: "debugger", in: "src/**", message: "No debugger", severity: warn}
YAML
printf 'console.log("x");\ndebugger;\nconst m = s.match(/x/);\n' > "$work/project/src/index.ts"
printf '{"klint.path": "%s"}\n' "$klint" > "$work/project/.vscode/settings.json"

(cd "$here" && bun run build >/dev/null)
xvfb-run -a "$code" --extensionDevelopmentPath="$here" \
  --extensionTestsPath="$here/e2e/suite.cjs" \
  --user-data-dir="$work/user" --extensions-dir="$work/extensions" \
  --disable-extensions --disable-gpu --no-sandbox --disable-workspace-trust \
  "$work/project" >"$work/vscode.log" 2>&1 || { cat "$work/vscode.log"; exit 1; }

node "$here/e2e/check.cjs" "$work/project/e2e-result.json"
