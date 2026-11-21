# klint

Architecture-as-Code checks for Python projects.

`klint` enforces architecture rules from a small `klint.yaml` file. Use it to
keep module boundaries explicit, block risky patterns in specific layers, pin
important symbols to their intended owner file, and hold files to size and
comment budgets.

It installs as a Python package and runs as a native executable:

```bash
pip install klint
klint
```

For machine-readable output:

```bash
klint --reporter=json
python -m klint --reporter=json
```

By default, `klint` looks for `klint.yaml` or `klint.config.json` in the current
working directory. Use `--config` when the config lives somewhere else:

```bash
klint --config path/to/project
```

## CLI

The Python package exposes both a console command and a module entrypoint:

```bash
klint
python -m klint
```

Supported options:

| Option | Description |
| --- | --- |
| `--config <dir>` | Directory containing `klint.yaml` or `klint.config.json`. Defaults to the current working directory. |
| `--fix` | Apply auto-fixes in place, then report what is left. Works with the `text` and `json` reporters; `--reporter=github` is refused with exit 1. |
| `--fix=safe` | Apply only safe fixes, the ones that change layout and never behaviour (today: the blank-line rules). |
| `--fix[=safe] <path>...` | Fix only these files, or the files under these directories. Paths work only with `--fix`. |
| `--reporter <name>` | Output format: `text` (default), `json` for CI and agentic lifecycle hooks, `github` for pull request annotations in GitHub Actions. `--json` is a short alias for `--reporter=json`. |
| `--version`, `-V`, `version` | Print the klint version. |
| `--help`, `-h`, `help`, `h` | Print CLI usage. |

Arguments are passed straight through to the bundled native binary.

## Configuration

Create `klint.yaml` at the root of your project:

```yaml
include: ["src"]
rules: {}
arch:
  layers:
    api: ["src/app/api/**"]
    db: ["src/app/db/**"]
    jobs: ["src/app/jobs/**"]
```

`include` selects which paths are scanned. Prefix an entry with `!` to exclude,
either a whole directory or individual files:

```yaml
include: ["src", "!**/*_test.py", "!src/jobs/legacy.py"]
```

Inside a git repository, klint skips every file git ignores (`.gitignore` at every level
and `.git/info/exclude`), so a `.venv` or build folder listed there needs no `!` pattern.
An ignored file stays skipped even when `include` names it. Outside a git repository
nothing is skipped. Set `respectGitignore: false` to lint ignored files too.

Set `$schema` to a version-tagged URL to pin the schema your editor validates against:

```yaml
$schema: https://raw.githubusercontent.com/konvert7/klint/refs/tags/v0.29.0/klint.schema.json
```

When the tagged version differs from the installed klint, the run reports a
`klint/schema-version` warning naming both versions and the URL to move to. It is a
warning, so the exit code stays `0`. Local paths carry no version and are never checked.

`arch.layers` gives names to file groups so rules can talk about
architecture instead of repeating globs. `root` optionally sets the directory
that `include` paths and reported file names resolve against.

`rules` holds klint's source rules. Most are TypeScript/JavaScript only; the ones
that also check Python are listed under [Supported Python Rules](#supported-python-rules).

Every `arch` rule accepts `severity`. Use `error` (exit code `2`) or `warn`
(reported, exit code `0`). Arch rules cannot be `off`; remove the entry instead.

## Import Boundaries

Use `arch.imports` to block dependencies between layers.

```yaml
include: ["src"]
rules: {}
arch:
  layers:
    api: ["src/app/api/**"]
    db: ["src/app/db/**"]
  imports:
    - from: api
      deny: db
      message: "API code must not import database internals directly"
```

This flags Python imports such as:

```python
from app.db.session import get_session
```

from files under `src/app/api/**`.

Use `allow` instead of `deny` to invert the check — anything not matching
`allow` is denied.

**What resolves.** Relative imports (`from ..lib.auth import load_key`) and
absolute project imports (`from app.lib.auth import load_key`) both resolve.
Absolute imports are matched against the project root and its direct child
directories containing Python files, checking `<module>.py`,
`<module>/__init__.py`, and — for PEP 420 namespace packages — a `<module>/`
directory the scan found `.py` files under.

Every target of a multi-target statement is checked separately, so
`import json, app.lib.auth` and `from . import helper, sibling` each produce one
record per target. Dynamic imports are read off the AST too:
`importlib.import_module("…")` and `__import__("…")`, including aliased bindings
such as `import importlib as il` and
`from importlib import import_module as load`. A call to a same-named method
that is not bound to `importlib` is not treated as an import.

Imports that do not resolve to a project file — third-party packages such as
`import requests` — are ignored by `deny`/`allow`. Use `deny-packages` to reach
those.

### Third-party and stdlib packages

`deny-packages` matches pip packages and standard-library modules, which
`deny`/`allow` cannot see because they resolve to no project file. Matching is
per dotted segment, so denying `os` also catches `os.path`.

```yaml
arch:
  imports:
    - from: jobs
      deny-packages: ["os", "requests"]
      message: "Jobs must go through the platform adapter"
```

### Type-only imports

Set `type-only: allow` to exempt imports inside an `if TYPE_CHECKING:` block,
mirroring how `import type` is exempted in TypeScript. The `else` and `elif`
branches of that guard remain runtime imports.

```yaml
arch:
  imports:
    - from: api
      deny: db
      type-only: allow
```

## Forbidden Patterns

Use `arch.forbidden` to block text patterns inside a layer.

```yaml
include: ["src"]
rules: {}
arch:
  layers:
    jobs: ["src/app/jobs/**"]
  forbidden:
    - in: jobs
      pattern: "print("
      message: "Jobs must not print directly"
```

`pattern` is a literal substring scanned per line. Prefix it with `re:` to match
a regular expression instead:

```yaml
    - in: jobs
      pattern: "re:^\\s*os\\.environ\\["
      message: "Read configuration through settings, not os.environ"
```

Regexes must stay inside the common regex subset — no lookaround or
backreferences. A literal pattern that itself begins with `re:` cannot be
expressed; such a value is always read as a regex.

This is useful for project-specific policies such as blocking direct logging,
environment access, framework shortcuts, or unsafe helpers in the wrong layer.

## Singleton Ownership

Use `arch.singleton` when a symbol or pattern must only appear in one file.

```yaml
include: ["src"]
rules: {}
arch:
  singleton:
    - only: "src/app/config/settings.py"
      pattern: "API_KEY"
      message: "API_KEY must only live in settings.py"
```

This allows `API_KEY` in `src/app/config/settings.py` and flags the same pattern
anywhere else in scanned files. `pattern` takes the same literal-or-`re:` form
as `arch.forbidden`.

## File Size

Use `arch.maxLines` to cap how long a file may get. The limit counts physical
lines, and the violation is reported at the first line past the limit.

```yaml
arch:
  layers:
    jobs: ["src/app/jobs/**"]
  maxLines:
    - limit: 500
      in: jobs
      message: "Split this job into smaller modules"
```

## Comment Budgets

Use `arch.maxCommentDensity` to cap what share of a file may be comments, and
`arch.maxCommentBlock` to cap how tall a single run of comment lines may get.

```yaml
arch:
  layers:
    jobs: ["src/app/jobs/**"]
  maxCommentDensity:
    - limit: 10
      in: jobs
  maxCommentBlock:
    - limit: 3
      in: jobs
```

Density is measured against total physical lines — code, comments, and blanks —
the same denominator `maxLines` uses. A comment block violation is reported at
the first line past the limit.

**What counts as a comment in Python.** `#` comments count toward both limits.
Docstrings are string expressions rather than comments, so they never count
toward either limit — a module that is nothing but docstrings measures 0%
density. The `countDocComments` option therefore has no effect on Python files;
it exists for languages whose doc-comments are real comment nodes.

### Ignoring structural comments

Some comment lines are machinery rather than prose — tool directives a linter or
codegen step reads. Use `ignore` to keep them out of both budgets:

```yaml
arch:
  maxCommentDensity:
    - limit: 10
      in: jobs
      ignore: ["re:^\\s*# (noqa|type:|pragma:)"]
```

`ignore` takes the same literal-or-`re:` form as `arch.forbidden` and tests the
physical source line. Ignored lines still count in the density denominator, and
for `maxCommentBlock` they connect a run without adding to its height — so a
directive sitting inside a comment block does not split it in two.

## Supported Python Rules

The Python package supports:

- `arch/imports`
- `arch/forbidden`
- `arch/singleton`
- `arch/max-lines`
- `arch/max-comment-density`
- `arch/max-comment-block`

These rules are intentionally configuration-driven. They are for enforcing your
project's architecture, not for replacing formatters or style linters.

These opt-in source rules under `rules.core` also check `.py` files:

- `blank-line-before-control-flow` and `blank-line-after-block` (fixed by `klint --fix`)
- `cognitive-complexity`
- `function-length`
- `naming-convention`: snake_case functions and PascalCase classes; unittest hooks
  such as `setUp` are skipped. There is no fix for Python.

```yaml
rules:
  core:
    function-length: error
    naming-convention: error
```

The other source rules and the `sonar` plugin are TypeScript/JavaScript only and do
not apply to `.py` files.

## CI

Run klint in CI after installing your Python dependencies:

```bash
pip install klint
klint --reporter=json
```

`klint` exits with:

- `0` when no errors are found
- `2` when rule violations are found
- `1` for configuration or runtime errors
