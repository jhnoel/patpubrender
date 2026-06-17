#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="$ROOT/e2e/fixtures/US12207575-20250128.XML"
WORKDIR="${TMPDIR:-/tmp}/patpubrender-e2e-python-$$"
trap 'rm -rf "$WORKDIR"' EXIT
mkdir -p "$WORKDIR"

python3 - "$FIXTURE" <<'PY'
import sys
try:
    import patpubrender
except ImportError as exc:
    raise SystemExit(
        "patpubrender is not importable. Install the Python package first, for example:\n"
        "  cd python && python -m pip install .\n"
        "or build a development wheel with maturin."
    ) from exc

fixture = sys.argv[1]
xml = open(fixture, encoding="utf-8").read()
doc = patpubrender.parse(xml)
md = doc.to_markdown()
assert "Apparatus for combining planting implements" in md, md
assert "## Claims" in md, md
assert doc.title == "Apparatus for combining planting implements", doc.title
print("Python package e2e passed")
PY
