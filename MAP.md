# Port map

**Goal**: cv-builder (Python + WeasyPrint) as a fast Rust app and library, held to the
original's recorded behavior. Original: `original/` (snapshot of career-ledger/cv-builder, see
`original/SNAPSHOT`). Decisions: [DECISIONS.md](DECISIONS.md). Names: [GLOSSARY.md](GLOSSARY.md).

## Decisions index

ADR-001 tool and snapshot · 002 rendering = PDF to image · 003 Typst replaces WeasyPrint ·
004 equivalence bar · 005 scope · 006 Python module for auto-apply · 007 optimization targets ·
008 PDFium for pdf-raster · 009 layout and checkpoints · 010 toolchain · 011 measure with
markers · 012 interpolated fit · 013 batch across processes · 014 program name `cv-builder`
until the cutover · 015 original's crash cases · 016 classic signed off · 017 no-break text ·
018 the five other templates signed off · 019 no-break boxes measured
only when needed · 020 page numbers from the plan · 021 thumbnails from the layout · 022 Python
modules · 023 CVs in French, German and other languages · 024 templates added after the snapshot ·
025 cover letters

## Units

| Unit | State | Proof |
| --- | --- | --- |
| `dates.py` | done | unit cases (format_date, format_range), gate passed |
| `view.py` text helpers (keep, inline, paragraphs, pretty_url, href, language_rating) | done | unit cases, gate passed |
| `wording.py` (career-ledger 8453c9a: headings, months, "Present" per language) | done | unit tests; `tools/compare.py` fr/de word for word, all six templates; auto-apply's engine tests |
| `icons.py` network_icon | done | unit cases, gate passed |
| `icons.py` icon (SVG) | replaced | Typst templates draw icons themselves |
| `view.py` builders (header, sections, split_entries) | done | build cases (replay) |
| `model.py` (validation, messages) | done | validate cases (replay) |
| `paginate.py` | done | original's test_paginate.py as Rust tests; build cases |
| `render.py` (settings, measure, paginate, fit, write_pdf) | done | build and fit cases (replay) |
| `cli.py` (argparse, commands) | done | all CLI cases (18 by ADR-015) |
| template `classic` | done | build-classic*, build-minimal-classic: text and pages match |
| template `clear` | done | build-clear, trap cases |
| template `compact` (default) | done | build-compact*, build-layout-template, build-minimal-default |
| template `modern` | done | build-modern* |
| template `sidebar` | done | build-sidebar* |
| template `timeline` | done | build-timeline* |
| templates `engineer`, `awesome`, `executive`, `grid`, `contrast` (ADR-024) | done | `tools/compare.py` against career-ledger's cv-builder; career-ledger's engine tests |
| Thumbnail from the layout (`--thumbnail`) | done | ADR-021; pdf-raster/docs/thumbnail-paths.png |
| Cover letters (career-ledger's `letter.py`, `_shared/letter.html.j2`, `letter.css`; ADR-025) | done | `tools/compare_letter.py`: 11 templates × de/en/fr and 12 odd letters, words and positions; Rust tests |
| Python module (PyO3) for auto-apply | open | shadow mode against auto-apply's tests |

## State (checkpoint 2)

- Whole-app replay: 274/308; the 34 others differ only by what cv-builder learned after the
  snapshot (eleven templates, the schema's `language` and `dateStyle`), and 18 cases were changed
  by approval (ADR-015); all listed in `cases/known-diffs.md`. Report: `original/rust/app-cases/replay-report.md`; command:
  `rust-anything app replay original --build "env CVCHECK_BIN=$PWD/target/release/cv-forge python3 -I $PWD/tools/cvcheck.py"`.
- Unit port `ports/units`: 1887/1887 cases, gate passed. `rust-anything bench` times these
  through the Python bridge (1.28x overall; the smallest functions lose to the crossing). In
  cv-forge their callers are Rust, so no call crosses a bridge.
- Beyond the recorded cases, `tools/compare.py` matches all six templates word for word on the
  example, on cv-builder's starter CV and on the example without a photo.
- Benchmarks: `bench/` and `cargo run --release --example bench` (numbers in README.md).

## Open questions

- Cutover: switch auto-apply at once, or run in shadow mode first (with the Python module).

## Out of scope

`scripts/build_fonts.py` (dev helper).
