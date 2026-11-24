# cv-forge

PDF CVs from JSON Resume files, fast: a Rust port of cv-builder with Typst in place of
WeasyPrint. Same JSON format, same templates, same page breaks, same messages. Cover letters
too, in the design of the CV they go with.

```
cv.json ──validate──▶ Cv ──view──▶ sections ──measure (Typst)──▶ paginate ──typeset──▶ PDF
```

## Use

```bash
cargo build --release
./target/release/cv-forge build examples/android-hmi-architect.json -t classic
./target/release/cv-forge build my-cv.json --max-pages 2
./target/release/cv-forge letter examples/android-hmi-architect.json examples/letter-de.json -t sidebar
./target/release/cv-forge validate my-cv.json
./target/release/cv-forge templates
```

The command line is cv-builder's: same arguments, output, warnings and exit codes. One
addition: `--thumbnail cv.png [--thumbnail-dpi 60]` also writes page 1 as a PNG, drawn from the
layout (6 to 10 ms, no PDF to reopen; DECISIONS.md ADR-021).
`CVFORGE_TIMING=1` prints where a build's time went.

## Speed

The bundled example, release build, this machine (24 threads), measured 2026-11-21. In-process:
the original with its imports excluded, cv-forge with a different CV each build.

| template | cv-builder | cv-forge | |
| --- | --- | --- | --- |
| classic | 661 ms | 22 ms | 30x |
| clear | 1041 ms | 31 ms | 34x |
| modern | 1193 ms | 31 ms | 39x |
| timeline | 1424 ms | 40 ms | 36x |
| compact (default) | 1735 ms | 52 ms | 33x |
| sidebar | 2270 ms | 57 ms | 40x |

- The same CV again (Typst's caches warm): 4 to 6 ms with any template.
- One build from the command line: 0.08 to 0.14 s and 40 to 54 MB, against 1.8 to 3.2 s and
  96 to 106 MB.
- A page limit: 2 measure passes when it cannot fit, 3 when it fits by tightening (up to 7 in
  the original).
- Batches: 240 different CVs in 0.8 s (classic) and 1.6 s (compact) as one process each on 24
  cores; a pool of persistent workers skips the per-process start.

`cargo run --release --example bench -- <cv.json> <template> [max_pages]` and
`bench/bench_original.py` measure both the same way.

What makes it fast (see DECISIONS.md): Typst instead of a browser engine; one layout per
measure pass with position markers (ADR-011); a page-limit search on interpolated measures,
checked with one real pass (ADR-012); no-break boxes measured only when they might overflow
(ADR-019); page numbers from the plan instead of a counter (ADR-020); release builds with fat LTO and x86-64-v3; fonts,
standard library and parsed templates shared by every build; mimalloc with on-demand commit and
batches across processes (ADR-013).

## From Python

```python
import cvforge
r = cvforge.build(cv_json, base_dir, template="compact", max_pages=2, pdf="cv.pdf", thumbnail_dpi=60)
r["pages"], r["fit"], r["notes"], r["thumbnail"]      # PNG bytes of page 1
r = cvforge.letter(cv_json, letter_json, base_dir, template="sidebar", pdf="letter.pdf")
r["pages"], r["notes"]
```

A letter (`examples/letter-de.json`) comes already worded: date line, subject, salutation and
closing in the letter's language; cv-forge draws it below the CV template's header (ADR-025).

`tools/install-into-career-ledger.sh` builds the cv-forge wheel and installs it where auto-apply
uses it (DECISIONS.md ADR-022). pdf-raster is a career-ledger workspace project, installed by its
`uv sync`.

## Layout

- `crates/cvforge`: the library (model, view, paginate, build, Typst engine).
- `crates/cvforge-cli`: the `cv-forge` binary.
- `crates/cvforge-py`: the `cvforge` Python module (maturin; `python/cvforge/__init__.py`).
- `templates/`: `template.json` (the original's) and `cv.typ` per template; `_shared/lib.typ`.
- `assets/fonts`: the original's fonts (SIL Open Font License), embedded.
- `original/`: the Python original, as reference, and rust-anything's recordings
  (`original/rust/app-cases`).
- `ports/units`: rust-anything's unit port (PyO3) that checks the library's functions against
  the original's recorded behavior.
- `tools/cvcheck.py`: reduces a PDF to what the equivalence bar compares.
- `tools/compare.py`, `tools/compare_letter.py`: a CV or a cover letter drawn by cv-builder and
  by cv-forge, compared (pages, words, word positions for letters; `--image` side by side).

## Checks

```bash
cargo test && cargo clippy --all-targets -- -D warnings
rust-anything app replay original --build "env CVCHECK_BIN=$PWD/target/release/cv-forge python3 -I $PWD/tools/cvcheck.py"
PATH=<venv>/bin:$PATH rust-anything gate ports/units
```
