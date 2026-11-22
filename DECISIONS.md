# Decisions

Architecture decisions for cv-forge, the Rust port of cv-builder. Numbered in order, never
renumbered; a changed decision gets a new ADR that supersedes the old one.

## ADR-001: Port with rust-anything, original kept as reference
Status: accepted (2026-11-21)
Context: The goal is a much faster cv-builder. The rust-anything workflow records the
original's behavior and holds every port to it.
Decision: Use the rust-anything CLI 2.1.0 (installed with `cargo install`). `rust-anything setup`
is skipped: the CLI is enough, and setup changes the agent configuration. The original is
snapshotted into `original/` (see `original/SNAPSHOT`) so the tool's recordings (`original/rust/`)
never touch the career-ledger repository.
Consequences: Recordings and replays run against the snapshot. When cv-builder changes upstream,
copy it again and re-record (`rust-anything app record original`).

## ADR-002: Rendering means PDF to image
Status: accepted (2026-11-21)
Context: The second project replaces auto-apply's `thumbnail()` (pypdfium2).
Decision: pdf-raster turns PDF pages into PNG (and WebP when wanted).
Consequences: See ADR-008.

## ADR-003: Typst replaces WeasyPrint
Status: accepted (2026-11-21)
Context: No Rust crate replaces WeasyPrint (an HTML/CSS engine). Choices were Typst as a library,
hand-written layout (krilla/parley/taffy) or an immature Rust HTML engine.
Decision: Typst 0.15 as a library. Each template's `cv.html.j2` + `style.css` becomes one
`cv.typ`, written box for box after the CSS (sizes, CSS line boxes, paddings, rules), so pages
break where the original's do. The bundled fonts are the original's, embedded in the binary.
Consequences: Custom templates must be Typst (`template.json` + `cv.typ`); HTML/CSS templates
from cv-builder folders do not load. `--html PATH` writes the data the template received (JSON)
for debugging, since there is no HTML.

## ADR-004: Equivalence bar
Status: accepted (2026-11-21)
Context: A different engine cannot give byte-identical PDFs.
Decision: Per case: the CLI's stdout, stderr and exit code exactly; for PDFs, the page count
and the text pdftotext extracts, word for word in reading order for templates that promise
reading order and as a sorted word list for the others (`tools/cvcheck.py`). Pages should look
nearly the same; the user signs off once per template from side-by-side images.
Consequences: Fit results (`line spacing X instead of Y`, `about N lines too long`) must match
exactly, so measurements must match the original's to well under a line.

## ADR-005: Scope: all of cv-builder
Status: accepted (2026-11-21)
Decision: `build`, `validate`, `schema`, `templates`, `init`; the JSON Resume model and its
validation messages; all six templates; fitting to a page limit (`max_pages`, `fewest_pages`);
the `Builder` API auto-apply uses. `scripts/build_fonts.py` (a dev helper that regenerates the
font files) stays out of scope.

## ADR-006: auto-apply calls Rust through a Python module
Status: accepted (2026-11-21)
Decision: A PyO3 module (maturin wheel) replaces `cv_builder.render.Builder` and
`auto_apply.render.thumbnail()` without changing callers, with rust-anything's shadow mode
(old/shadow/new) for the switch. Each project also ships a CLI.
Consequences: Comes with the cutover (later checkpoint); the library API is shaped for it
(`Builder::pdf` returns bytes and a `BuildResult` with notes, fit and timings).

## ADR-007: Optimization targets
Status: accepted (2026-11-21)
Decision: Release profile with fat LTO, one codegen unit, `panic = "abort"`, stripped symbols;
x86-64-v3 code (runs on any x86-64 CPU since about 2015, the server included); mimalloc in the
binaries; profile-guided optimization before the cutover. Goals: at most 50 ms per CV, at least
200 CVs/s in batch, at most 10 ms per thumbnail.

## ADR-008: pdf-raster draws with PDFium
Status: accepted (2026-11-21)
Decision: pdfium-render (the same PDFium as today, so the same pixels), loading the
`libpdfium.so` pypdfium2 already ships; hayro (pure Rust) benchmarked once for comparison. For
cv-forge's own CVs, a fast path draws page 1 straight from the Typst layout.

## ADR-009: Project layout and checkpoints
Status: accepted (2026-11-21)
Decision: `personal_2/cv-forge` and `personal_2/pdf-raster`, each its own Cargo workspace and git
repository. Stops for the user's go-ahead: (1) the first template end to end and benchmarked,
(2) all six templates, (3) the renderer.

## ADR-010: Toolchain
Status: accepted (2026-11-21)
Context: rust-anything 2.1.0 needs rustc 1.97; the machine's default is 1.94, and cargo 1.100
crashes (segfault) while updating the crates.io index here.
Decision: cv-forge builds with the default stable toolchain (Typst needs 1.92). Rust 1.100 is
installed beside it, only to build rust-anything itself (`RUSTC=… cargo install`). The default
toolchain is unchanged.

## ADR-011: Measure by laying the CV out once, with markers
Status: accepted (2026-11-21)
Context: The original measures by laying every column out on one tall page and reading item
boxes. A first port measured each item with Typst's `measure()`: 60 isolated layouts, 23 ms a pass.
Decision: The template lays the CV out once on a page of any height and places invisible markers
(`#metadata(..) <cvforge-m>`) at every section and item boundary; the builder reads their
positions. All vertical space in templates is explicit (insets and spacer blocks), so positions
on the tall page hold on real pages.
Consequences: 12 ms a pass. Templates must keep vertical space explicit and emit the markers in
measure mode (see `templates/_shared/lib.typ`).

## ADR-012: Fit by interpolation, verified
Status: accepted (2026-11-21)
Context: Fitting runs the original's binary search, up to 7 measure passes. Tightening changes
line spacing only, so line breaks stay and every position is linear in the tightness (or convex,
with fixed-height boxes in a line).
Decision: Measure at tightness 0 and 1, run the binary search on interpolated measures, then
check one inner point with a real pass. Convexity makes one agreeing point proof of linearity
everywhere; on disagreement, the search reruns on real passes.
Consequences: At most 3 passes instead of 7, same results as the original (checked on the
recorded fit cases and on shortened CVs).

## ADR-013: Batch parallelism across processes
Status: accepted (2026-11-21)
Context: Typst memoizes through global caches (comemo), one lock per function. Parallel builds in
one process queue on those locks: 24 threads did no better than 8.
Decision: Batches run in parallel processes (auto-apply already uses a process pool). Cache
eviction runs every 64th build, not after each, and mimalloc commits memory on demand
(`.cargo/config.toml`): eager commit made 24 parallel processes spend 30 s in the kernel for
240 builds.
Consequences: 240 builds in 0.83 s from fresh processes; a persistent worker pool goes faster.

## ADR-014: Program name in messages
Status: accepted (2026-11-21, the user's go-ahead at checkpoint 1)
Context: The binary is `cv-forge`, but cv-builder's usage lines and `init` hints say
`cv-builder`, and the recorded cases hold that text.
Decision: Keep printing `cv-builder` until the cutover, so every case matches; switch to
`cv-forge` at the cutover, with the cases updated then.

## ADR-015: Cases where the original crashes
Status: accepted (2026-11-21, the user's go-ahead at checkpoint 1)
Context: 18 recorded cases pass `''` as a path. pathlib reads it as `.`, the current folder, and
cv-builder crashes with a Python traceback (`IsADirectoryError`).
Decision: The port prints a one-line error and exits 1. These cases' expected stderr is
replaced with the port's error (the only cases changed by hand; listed in
`cases/known-diffs.md`), so replay holds the port to it.

## ADR-016: Classic template signed off
Status: accepted (2026-11-21)
Context: Side-by-side pages of the example CV (`docs/classic-side-by-side.png`): same breaks,
same lines, same words per page.
Decision: classic meets ADR-004. Each further template gets the same image and sign-off.

## ADR-017: No-break text: whole "++" words, and nowrap boxes sized to their text
Status: accepted (2026-11-21)
Context: The original keeps "++" on one line with `white-space: nowrap`; CSS then breaks only
where Unicode allows, never between a letter and "+". Typst allows a break before any box, so
"C" ended one line and "++" began the next. And a Typst box wraps its own content at the column
edge, where CSS `nowrap` overflows: long URLs broke at a hyphen, and pdftotext joined the halves.
Decision: The no-break box holds the whole word that contains "++" ("C++17,"). `nowrap()` in
`lib.typ` gives a box the natural width of its content, so it overflows as in CSS.
Consequences: A break inside such a word after a hyphen ("C++-based") is no longer possible;
no recorded case has one.

## ADR-018: The remaining five templates signed off
Status: accepted (2026-11-21, the user's go-ahead at checkpoint 2)
Context: clear, modern, timeline, compact and sidebar match the original word for word on every
page of the example (and on the fit cases); all 308 recorded cases pass.
Decision: They meet ADR-004, pending the user's look at `docs/<template>-side-by-side.png`.

## ADR-019: No-break boxes measured only when they might overflow
Status: accepted (2026-11-21)
Context: `nowrap()` measures its content (a `context` block) so it can overflow like CSS. Each
measurement lays the content out a second time.
Decision: The builder sends an upper bound of each no-break text's width (`w`, in em: every
character at its widest advance in any bundled font). The template draws a plain box when
`w × size × 1.02 + fixed parts` fits the column, and measures only otherwise.
Consequences: Same result for everything that fits (a plain box wraps nothing it can hold); only
long URLs and the like are measured.

## ADR-020: Page numbers from the plan, not from a counter
Status: accepted (2026-11-21)
Context: sidebar prints "n / total" after page 1. With `counter(page).final()` in the footer,
Typst lays the document out again to settle the count: 30 ms of a 70 ms typeset.
Decision: The builder knows the pages it planned, so each page draws its own number.
Consequences: When an item taller than a page spills onto an extra page (a case the build
already warns about), that extra page has no number.

## ADR-021: Thumbnails from the layout
Status: accepted (2026-11-21)
Context: auto-apply builds a CV's PDF, then reopens it to draw page 1 (pdf-raster replaces that
second step, at 18 ms). cv-forge has the layout in memory when it writes the PDF.
Decision: `Builder::set_thumbnail(dpi)` / `cv-forge build --thumbnail PNG [--thumbnail-dpi N]`
also draw page 1 with typst-render, sized as pypdfium2 sizes it. 6 to 10 ms in a warm process.
cv-builder's usage text does not list the two options, so its recorded cases stay as they are.
Consequences: Another rasterizer than PDFium: the same look, not the same pixels (pdf-raster
ADR-007). A fresh process pays about 80 ms once (glyph caches, image decoding).

## ADR-022: The Python modules and auto-apply's switch
Status: accepted (2026-11-21)
Context: ADR-006 (a PyO3 module, rust-anything's shadow mode). auto-apply reaches cv-builder
and pypdfium2 only through `auto_apply/render.py`.
Decision: Two maturin wheels: `cvforge` (`build`, `fewest_pages`, `fit`, `problems`; the CV as
JSON text, validated by cv-forge; the GIL released while it typesets) and `pdfraster`
(`thumbnail`, with pypdfium2's exception types and texts; libpdfium found in the installed
pypdfium2). Both build with the `release-py` profile (`panic = "unwind"`, so a Rust panic is
an exception, not the end of the worker). `render.py` switches on `RUST_ANYTHING_MODE`: `old`,
`shadow` (default: cv-builder's results served, the Rust side run too, every difference logged)
and `new`. Cached renders are keyed by the engine that drew them. Work on career-ledger's branch
`rust-pdf-engine`. The wheels are installed into the venv (`tools/install-into-career-ledger.sh`),
not added to career-ledger's pyproject, so its lockfile and Docker build stay as they are.
Consequences: Without the modules, auto-apply runs on cv-builder exactly as before. `uv sync`
removes the wheels (run the script again). The Docker image needs the Rust projects (or their
wheels) before it can use them: still to do.

## ADR-023: CVs in French, German and other languages
Status: accepted (2026-11-22)
Context: cv-builder (career-ledger 8453c9a) learned `layout.language` (a BCP-47 tag) and
`layout.dateStyle` (month | numeric): headings, "Present", "{type} in {area}", "continued" and
the PDF title word in English, French and German; other languages get CLDR's month names (Babel)
and English headings; German writes months as numbers unless told otherwise. The timeline marks
the current role from the entry, no longer from the word "Present". cv-forge rejected the two
keys, so auto-apply kept non-English CVs on cv-builder.
Decision: `wording.rs` ports `wording.py` table for table. Month names for other languages come
from `months.rs`, generated by `tools/gen_months.py` with the Babel cv-builder uses (every two-
and three-letter code Babel knows, so an alias such as `fre` gets what cv-builder gets). Typst
gets the primary subtag as `lang` and a two-letter region as `region`. The Python module
exposes `LANGUAGES`, the codes auto-apply may send cv-forge: en, fr, de, then every CLDR language
whose months all bundled fonts print, except those Typst lays out by rules WeasyPrint lacks
(`TYPST_OWN_RULES`: a hyphen repeated after a line break in es, pt, pl, cs, sk, hr, dsb; Chinese
and Japanese line breaking; right-to-left languages) and those whose uppercase WeasyPrint
tailors (`WEASYPRINT_OWN_RULES`: el, tr, az). cv-builder draws CVs in those.
Consequences: French and German match cv-builder word for word in all six templates
(`CVBUILDER_SRC=<career-ledger>/cv-builder/src tools/compare.py`), as do it, nl and ru (spot
checks; Turkish headings differed, hence the list above). The recorded cases are
English and unchanged. Regenerate `months.rs` when career-ledger's Babel changes. Spanish,
Portuguese, Polish and Czech CVs stay on cv-builder until cv-forge either follows WeasyPrint's
hyphen (Typst has no switch for it) or cv-builder follows Typst's.
