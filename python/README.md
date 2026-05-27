# klint

Architecture-as-Code checks for Python projects.

`klint` enforces architecture rules from a small `klint.yaml` file. Use it to
keep module boundaries explicit, block risky patterns in specific layers, and
pin important symbols to their intended owner file.

It installs as a Python package and runs as a native executable:

```bash
pip install klint
klint
```

For machine-readable output:

```bash
klint --json
python -m klint --json
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
| `--json` | Emit structured JSON to stdout. Useful in CI and agentic lifecycle hooks. |
| `--help`, `-h`, `help`, `h` | Print CLI usage. |

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

`include` controls which files are scanned. `arch.layers` gives names to file
groups so rules can talk about architecture instead of repeating globs.

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
anywhere else in scanned files.

## Supported Python Rules

The Python package currently supports:

- `arch/imports`
- `arch/forbidden`
- `arch/singleton`

These rules are intentionally configuration-driven. They are for enforcing your
project's architecture, not for replacing formatters or style linters.

## CI

Run klint in CI after installing your Python dependencies:

```bash
pip install klint
klint --json
```

`klint` exits with:

- `0` when no errors are found
- `2` when rule violations are found
- `1` for configuration or runtime errors
