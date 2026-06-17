#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="$ROOT/e2e/fixtures/US12207575-20250128.XML"
WORKDIR="${TMPDIR:-/tmp}/patpubrender-e2e-rust-$$"
trap 'rm -rf "$WORKDIR"' EXIT
mkdir -p "$WORKDIR/xml" "$WORKDIR/out"

cp "$FIXTURE" "$WORKDIR/xml/US12207575-20250128.XML"

cargo run --manifest-path "$ROOT/Cargo.toml" -- render "$FIXTURE" --output "$WORKDIR/rendered.md"
grep -q "Apparatus for combining planting implements" "$WORKDIR/rendered.md"
grep -q "## Claims" "$WORKDIR/rendered.md"

# Simulate a USPTO weekly XML bulk data ZIP and exercise shard ingest/read.
python3 - "$WORKDIR/ipg250128.zip" "$WORKDIR/xml/US12207575-20250128.XML" <<'PY'
import sys, zipfile
zip_path, xml_path = sys.argv[1:]
with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
    zf.write(xml_path, "US12207575-20250128.XML")
PY

cargo run --manifest-path "$ROOT/Cargo.toml" --features shard -- shard write \
  --zip "$WORKDIR/ipg250128.zip" --output "$WORKDIR/out"

cargo run --manifest-path "$ROOT/Cargo.toml" --features shard -- shard read \
  --shard "$WORKDIR/out/ipg250128.zst" --index "$WORKDIR/out/ipg250128.idx" \
  --key US12207575-20250128 > "$WORKDIR/read.md"

grep -q "Apparatus for combining planting implements" "$WORKDIR/read.md"
echo "Rust CLI e2e passed"
