# klint (.NET tool)

Architecture-as-Code linting for TypeScript, Python, Swift, Rust, and C# projects.

This package delivers the native `klint` engine as a .NET global (or local) tool.
It bundles the platform-native binary and forwards every argument to it.

## Install

```bash
# global
dotnet tool install --global Konvert7.Klint

# or as a repo-local tool
dotnet new tool-manifest
dotnet tool install Konvert7.Klint
```

## Use

```bash
klint --help
klint --json
```

Configuration lives in `klint.yaml` at your project root — the same file the npm
and PyPI distributions read. See https://github.com/konvert7/klint for the full
rule reference.

## Supported platforms

`linux-x64`, `osx-x64`, `osx-arm64`, `win-x64`.
