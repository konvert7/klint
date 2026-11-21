# klint for VS Code

Shows [klint](https://klint.dev) violations inline: errors underlined in red, warnings in yellow, and all of them in the Problems panel. Each one links to its rule's documentation.

## Requirements

klint itself, installed in the project or on your PATH:

```sh
npm install --save-dev @konvert7/klint   # or: pip install klint
```

The extension runs only in workspace folders that contain `klint.yaml` or `klint.config.json`.

## How it works

The extension runs `klint --json` in each workspace folder when the workspace opens and again after every save, then shows what klint reports. It uses the first klint it finds:

1. the `klint.path` setting
2. `node_modules/.bin/klint`
3. `.venv/bin/klint` (`.venv\Scripts\klint.exe` on Windows)
4. `klint` on your PATH

Errors from klint itself (a broken config, a missing binary) are written to the **klint** output channel.

Run **klint: Lint workspace** from the command palette to lint again without saving.

## Limits

- Diagnostics update on save, not as you type.
- The whole line is underlined, because klint's JSON output does not include the column yet.
