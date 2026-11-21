#!/usr/bin/env bash
# Build cv-forge's and pdf-raster's Python wheels and install them into career-ledger's venv,
# where auto-apply's render.py picks them up (RUST_ANYTHING_MODE: shadow by default, new, old).
# `uv sync` in career-ledger removes them again: run this after it.
#   CAREER_LEDGER=/path/to/career-ledger   (default: ~/Documents/projects/personal/career/career-ledger)
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
raster="$(cd "$here/../pdf-raster" && pwd)"
ledger="${CAREER_LEDGER:-$HOME/Documents/projects/personal/career/career-ledger}"
maturin=(uvx --from 'maturin>=1.9,<2' maturin)

(cd "$raster/crates/pdfraster-py" && "${maturin[@]}" build --out "$raster/dist")
(cd "$here/crates/cvforge-py" && "${maturin[@]}" build --out "$here/dist")

uv pip install --python "$ledger/.venv/bin/python" --reinstall \
  "$(ls -t "$here"/dist/cvforge-*.whl | head -1)" \
  "$(ls -t "$raster"/dist/pdfraster-*.whl | head -1)"
"$ledger/.venv/bin/python" -c 'import cvforge, pdfraster; print("installed:", cvforge.ENGINE, "and pdfraster", pdfraster.__version__)'
