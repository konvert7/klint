from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def main() -> int:
    binary = _binary_path()
    if not binary.exists():
        sys.stderr.write(f"klint: bundled native binary is missing: {binary}\n")
        return 1

    result = subprocess.run([str(binary), *sys.argv[1:]], check=False)
    return result.returncode


def _binary_path() -> Path:
    name = "klint-rs.exe" if os.name == "nt" else "klint-rs"
    return Path(__file__).resolve().parent / "_bin" / name


if __name__ == "__main__":
    raise SystemExit(main())
