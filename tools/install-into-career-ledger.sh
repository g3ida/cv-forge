#!/usr/bin/env bash
# Build cv-forge's Python wheel and install it into career-ledger's venv, where auto-apply's
# render.py picks it up (RUST_ANYTHING_MODE: shadow by default, new, old). pdf-raster is a
# career-ledger workspace project now, so `uv sync` installs that one; it removes cvforge again:
# run this after it.
#   CAREER_LEDGER=/path/to/career-ledger   (default: ~/Documents/projects/personal/career/career-ledger)
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
ledger="${CAREER_LEDGER:-$HOME/Documents/projects/personal/career/career-ledger}"

(cd "$here/crates/cvforge-py" && uvx --from 'maturin>=1.9,<2' maturin build --out "$here/dist")

uv pip install --python "$ledger/.venv/bin/python" --reinstall \
  "$(ls -t "$here"/dist/cvforge-*.whl | head -1)"
"$ledger/.venv/bin/python" -c 'import cvforge, pdfraster; print("installed:", cvforge.ENGINE, "next to pdfraster", pdfraster.__version__)'
