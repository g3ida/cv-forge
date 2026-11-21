# cv-builder

Build a PDF CV from a JSON file.

```
cv.json ──validate──▶ model ──template (HTML/CSS)──▶ measure ──paginate──▶ PDF
```

- **One standardized JSON format**, a superset of [JSON Resume v1.0.0](https://jsonresume.org/schema).
  Any JSON Resume file works as input, and a JSON Schema gives you validation and completion in your editor.
- **Templates are HTML + CSS** (Jinja2), rendered by [WeasyPrint](https://weasyprint.org). Six are bundled
  (see [Templates](#templates)); each says whether its text extracts in plain reading order, which is what
  an ATS resume parser needs. The default, `compact`, follows the reference CV in `examples/` to within about 1pt.
- **Column-aware pagination**: each column flows from page to page on its own, items are never split,
  and a section that continues on the next page repeats its title (the reading-order templates do not
  repeat it and may break an entry between two highlights).
- The output has selectable text and is a tagged PDF, with title and author metadata.

## Install

Requires Python ≥ 3.11 and the Pango library, which WeasyPrint uses (preinstalled on most Linux desktops).

```bash
uv sync                      # dev environment; then use `uv run cv-builder ...`
uv tool install .            # or install the `cv-builder` command globally
```

## Usage

```bash
cv-builder init my-cv.json                       # starter CV + cv.schema.json next to it
cv-builder validate my-cv.json                   # check against the schema
cv-builder build my-cv.json                      # -> my-cv.pdf
cv-builder build my-cv.json -o out/cv.pdf --accent "#0B7A75" --page-size Letter
cv-builder build my-cv.json --max-pages 2        # fit on 2 pages (see Page limit); exit code 3 if it cannot
cv-builder build my-cv.json --html debug.html    # also dump the paginated HTML
cv-builder build my-cv.json -t clear             # one column in reading order, for ATS parsers
cv-builder templates                             # list templates
cv-builder schema -o cv.schema.json              # print/write the JSON Schema
```

To try it on the bundled example (the reference CV transcribed into this format):

```bash
uv run cv-builder build examples/android-hmi-architect.json
```

Warnings go to stderr. You get one when a section has content but isn't placed in the layout, when the photo
file is missing, when a single item is taller than a page, or when the CV does not fit its page limit.

### Page limit

`--max-pages N` (or `layout.maxPages`) fits the CV on at most N pages. A CV that runs over gets tighter
line spacing, and nothing else: the text size and the margins stay as the template sets them. CV guides
agree on cutting content before shrinking either (body text 10 to 12pt, margins 0.5 to 1 inch or 1.5 to
2.5 cm); line spacing has a usual range of 1.15 to 1.5. So every unitless `line-height` between 1.15 and
1.5 in the template moves toward 1.15 by the same share, the least that fits (`LINE_HEIGHT_FLOOR` in
`render.py`); values outside that range are design, not text, and stay. That is 6% more room per page in
compact (1.23) and 15 to 19% in the others. When even 1.15 is not enough, the CV is drawn as designed and
the build warns, for example `does not fit on 2 pages: about 38 lines too long, even with the line spacing
at 1.15`, and exits with code 3: content has to go. The command line reports what it changed:
`my-cv.pdf (2 pages, line spacing 1.24 instead of 1.36 to fit)`.

The reading order holds at the tightest spacing (tested on clear, classic and modern with both extractors).
`Builder.fewest_pages()` says how few pages the content can take at all, and `Builder.fit()` returns the
`Fit` without writing a PDF.

The bundled templates follow the same guidance: body text at 10pt and no text anywhere smaller (dates,
contact details and labels included; a test checks every template), headings larger.

An item taller than a page never fits, whatever the limit: WeasyPrint breaks it over more pages than can be
predicted.

## The JSON format

The full definition is in [`schema/cv.schema.json`](schema/cv.schema.json), generated from
[`src/cv_builder/model.py`](src/cv_builder/model.py). Add `"$schema": "<path>/cv.schema.json"` to your file
to get validation and completion in your editor.

### Conventions

| Rule | Example |
|---|---|
| Dates are ISO 8601 at year, month or day precision. The precision is kept when printed. | `"2019"` → `2019`, `"2019-07"` → `Jul 2019` |
| A missing `endDate` means ongoing (JSON Resume semantics). | `"startDate": "2026-07"` → `Jul 2026 - Present` |
| `"present"` (any case) can also be written explicitly. | `"endDate": "Present"` |
| For a single point in time, set `endDate` equal to `startDate`. | `"2026"`/`"2026"` → `2026` |
| `endDate` may not be earlier than `startDate`. | `2020-05` → `2019` is an error |
| Unknown keys are rejected, so typos are caught. | `hightlights` → error with its location |
| Empty strings count as missing, so exports from other tools still load. | `"url": ""` |
| Text fields support `**bold**` and `[label](url)`. Blank lines separate paragraphs in summaries. | |

### Sections

| Key | Source | Rendered as |
|---|---|---|
| `basics` | JSON Resume | header: `name`, `label` (headline), `image` (photo path relative to the JSON file, or a URL), `email`, `phone`, `url`, `location`, `profiles[]`; `summary` is the **Summary** section |
| `work[]` | JSON Resume + ext. | `position` (title), `name` (company), dates, `location`, `description` (what the company does), `summary` (the role), `highlights[]`; **ext:** `project` (tells apart several entries at one company), `titleHistory[]` (`position` and dates of each title held; two or more print as one line, `Engineer → Senior Engineer`). Entries in a row at the same company stay separate, each with the company's name, because resume parsers lose roles stacked under one heading; the company's `description` is printed once |
| `volunteer[]` | JSON Resume | like `work` |
| `education[]` | JSON Resume + ext. | "`studyType` in `area`", `institution`, dates, `score`, `courses[]`; **ext:** `location`, `highlights[]` |
| `projects[]` | JSON Resume | `name`, dates, `url`, `description`, `highlights[]`, `keywords[]`, `roles[]`, `entity` |
| `skills[]` | JSON Resume | group `name` + comma-joined `keywords[]` (+ `level`) |
| `languages[]` | JSON Resume + ext. | `language`, `fluency`; **ext:** `rating` 0-5 dots (if omitted, inferred from CEFR level or words like *Native*/*Fluent*) |
| `awards[]`, `certificates[]`, `publications[]`, `interests[]`, `references[]` | JSON Resume | standard fields |
| `achievements[]` | **extension** | `title`, `summary`, `icon` (`gem`, `star`, `award`, `zap`, `target`) |
| `meta` | JSON Resume | free-form, not rendered |
| `layout` | **extension** | presentation, see below |

Profiles go into the header when their `network` is listed in `layout.header.profiles`. The others go into
the `profiles` section ("Find Me Online").

### `layout`

Every field is optional. If you leave a field out, the template default applies, and command-line flags
override both.

```jsonc
"layout": {
  "template": "compact",             // a bundled template (see Templates) or a template directory
  "pageSize": "A4",                  // or "Letter"
  "maxPages": 2,                     // page limit: see Page limit
  "accentColor": "#1E90FF",
  "documentTitle": "Android HMI Architect - Jane Doe",   // PDF title metadata
  "columns": {                       // column names come from the template: left and right here,
    "left":  ["work", "education", "profiles"],        // one column called main in clear
    "right": ["summary", "achievements", "skills", "projects", "languages"]
  },                                 // sections left out are not rendered
  "titles": { "profiles": "Online", "work": "Work History" },
  "header": {
    "contacts": ["phone", "email", "url"],   // also "location"
    "profiles": ["LinkedIn"],
    "photo": true
  }
}
```

Section ids: `summary work volunteer education projects skills languages achievements awards certificates
publications interests references profiles`.

`layout.columns` must use the column names of the template the CV asks for; anything else is an error.
The one exception is `-t` on the command line picking a different template than the CV does. When that
template has the same columns (all the one-column templates call theirs `main`), the CV's columns apply as
written. Otherwise the build warns and keeps the CV's choice of sections, placed and ordered the way the
new template's defaults place them; sections the template does not list go to the end of its main column.

## Templates

`cv-builder templates` lists them. "Reads in order" means the PDF's text extracts in plain reading order
with both pdftotext and pypdf, so a job portal's resume parser (ATS) gets the roles, dates, titles and
employers right; auto-apply's parse-back test checks every template that claims it.

| Template | Reads in order | Looks like | Use it for |
|---|---|---|---|
| `clear` (formerly `single-column`) | yes | One plain column, Inter, accent headings over a thin rule, real-text bullets | Job portals and any pipeline that parses the PDF; the safest choice |
| `classic` | yes | Noto Serif, centred header (photo above the name), bold headings over a rule, employer in italics | Conservative readers: finance, law, consulting, public sector, academia |
| `modern` | yes | Full-width accent band with the name and a round photo, contacts on a light strip, accent-marked headings | Tech and product roles, read by people and parsers alike |
| `compact` (formerly `two-column`; the default) | no | Two dense columns, icons, rating dots, round photo; follows the reference CV in `examples/` | A CV a person opens; fits the most on a page |
| `sidebar` | no | Grey full-height side band with the photo, skill chips and language bars; experience in the main column | Email, interviews, direct applications |
| `timeline` | no | Dates and place in a left gutter, a line with a dot per role (filled while ongoing) | Showing a career's progression to a person |

The three "no" templates lose titles and employers when parsed: their columns or gutter come out as
separate blocks. Every sentence is still in the text. A former template name still works everywhere.

All templates take `layout.accentColor` (each has its own default), A4 or Letter, and work with or without
a photo and a headline. `sidebar` shows your initials where the photo would be when there is none.

Every template sets body text at 10pt, the minimum CV guides give, and nothing smaller. Certificates and
awards take one line each, `Name, Issuer, 2026`, with the certificate's link on its name: as four-line
entries (name, issuer, date, link) they filled a third of a page on a long record. A year alone, never a
range, so a resume parser does not read the line as a role.

### Reading order (clear, classic, modern)

```bash
cv-builder build my-cv.json -t clear
```

or in the file, with the section order in the template's one column, `main`:

```jsonc
"layout": {
  "template": "clear",
  "columns": {
    "main": ["summary", "work", "education", "skills", "projects", "certificates", "awards", "languages",
             "achievements", "volunteer", "publications", "interests", "references", "profiles"]
  }
}
```

That list is also the default, so `columns` is only needed to change the order or to drop a section.

What a text extractor gets (`pdftotext file.pdf -` and pypdf's `extract_text()` both return this, line for line;
the separators differ: ` | ` in clear and modern, ` · ` in classic):

- The name, the headline, then the contacts: email, phone, location, website and every profile.
  `layout.header.contacts` and `layout.header.profiles` still override this; profiles kept out of the
  header go to the `profiles` section.
- Each section under its usual heading (`Summary`, `Experience`, `Education`, `Skills`, `Projects`,
  `Certifications`, `Awards`, `Languages`, ...; change them with `layout.titles`). A heading is printed once,
  also when its section continues on the next page.
- An entry is the position, the company, the dates (`Jul 2026 - Present`) and the location (`Munich, Germany`),
  each on its own line, then the description and summary, then one line per highlight starting with `• `.
  The dates stand alone because a resume parser attaches whatever shares their line to the role: with
  `Munich, Germany | Jul 2026 - Present` on one line, the parser used for testing read the title as "Munich"
  and the employer as "Germany".
  Education, projects and the other entry sections follow the same pattern.
- Skills and interests are `Group: keyword, keyword`, languages are `Language: fluency`.

There are no icons, text is never upper-cased or letter-spaced, and nothing is floated or positioned.
A long line wraps wherever the text allows, which includes after a `-` or `/` inside a word, so compare
extracted text with the whitespace removed. "C++" never breaks between its plus signs (in any template).

The photo is shown when `basics.image` is set and `layout.header.photo` is not `false`. It is drawn as an
image and adds no text. Set `"photo": false` for portals that reject CVs with a picture.

Page breaks can fall between the highlights of an entry, but the heading of an entry always stays with its
paragraphs and its first highlight. Expect more pages than `compact` for the same content: the type is
larger and nothing sits side by side (the example is 3 pages in clear and modern, 4 in classic, 2 in compact).

### Writing a template

A template is a directory with three files:

- `template.json`: name, `label` (display name), description (one sentence for a template picker),
  `readingOrder` (true only if the text extracts in reading order; see below), default `pageSize`, `margins`
  (pt), `accentColor`, default `columns`, `titles`. Optional: `aliases` (former names that still load), `header` (defaults for `layout.header`; `"profiles": "*"` puts every profile in the header),
  `repeatTitles` (default `true`; `false` prints a section title only where the section starts),
  `splitEntries` (default `false`; `true` allows a page break between the highlights of an entry) and
  `fitTolerance` (px by which the paginator lets the content of a page exceed the page, default `0.5`;
  with `0` or less a page is never overfilled, so nothing can spill onto an extra page)
- `cv.html.j2`: Jinja2 page template (receives `pages`, `header`, `column_names`, `cv`, `icon()`)
- `style.css`: styles. The accent color is available as `var(--accent)` and the page margins as
  `var(--margin-top)`, `--margin-right`, `--margin-bottom` and `--margin-left` (for designs that reach the
  paper's edge). The bundled fonts **Inter**, **Rubik** and **Noto Serif** (with italics) are available by name.

How pagination works: the builder first renders all the content on one very tall page and reads the height
of each item from WeasyPrint's layout tree. It then fills real pages column by column. Page 1 loses the height
of the header. Continued sections get their title again unless the template turns that off. For this to work,
the template must put `data-m="columns"` on the columns container, `data-m="section:<id>"` on each section and
`data-m="item:<id>:<index>"` on each item. The bundled templates share that markup:
`{% include "_shared/section.html.j2" %}` renders one section with its hooks, and `_shared/macros.html.j2`
has the item macros that are the same everywhere. Both are available to any template. Pass `-t path/to/dir`
to use a template that isn't bundled.

Things learned building the bundled ones:

- Space inside and between items with padding, never margin: a margin collapses out of the measured box.
- Use `fitTolerance` -0.1 when the columns sit in a flex row: if a page overfills by a fraction of a point,
  WeasyPrint moves the whole row, and everything on that page, to the next page.
- For `readingOrder`, keep entry titles, dates and location at the body's line height and font size.
  A tighter line height under a short title, or smaller meta text, made pdftotext end its text block there,
  and the parser then took the employer for the title. Test with a short CV as well as a long one.
- In WeasyPrint, a `linear-gradient` with two-position colour stops is dropped, a one-colour gradient ignores
  `background-size`, and a page background is placed inside the margins: draw a full-height band as
  `linear-gradient(c, c) -<left margin> 0 / <width> 100% repeat-y` on `@page`. `color-mix()` is not supported.
- No text below 10pt, labels and dates included (`test_no_text_is_smaller_than_10pt`). Give a gutter or
  a fixed-width label room for that: timeline's date gutter is 112pt because "May 2008 – May 2008" is
  about 109pt wide at 10pt.
- Register the shared `line` macro (certificates, awards) in the template's renderers.
- A page limit rewrites the unitless `line-height` values between 1.15 and 1.5 in `style.css`. Give text its
  leading as a plain number in that range; use a length (`12pt`) or a value outside it for anything that
  must not move, such as an icon box or the initials circle.

## Development

```bash
uv run pytest                                          # tests (incl. schema-in-sync check)
uv run cv-builder schema -o schema/cv.schema.json      # after changing model.py
python scripts/build_fonts.py Rubik[wght].ttf Inter[opsz,wght].ttf   # regenerate static fonts
```

Layout:

```
src/cv_builder/
  model.py        standardized format (pydantic) + JSON Schema
  view.py         model -> display-ready sections (dates, titles, links)
  render.py       templates, measuring pass, PDF output
  paginate.py     column-by-column page filling (pure, unit-tested)
  dates.py        date formatting
  icons.py        inline SVG icons (Feather/Lucide paths)
  cli.py          command line
  templates/clear/ classic/ modern/ compact/ sidebar/ timeline/
  templates/_shared/          partials used by every template
  assets/fonts/   Inter + Rubik static instances, Noto Serif (SIL OFL 1.1, licenses alongside)
  assets/starter.json
schema/cv.schema.json
examples/android-hmi-architect.json  (+ photo.jpg)
```

WeasyPrint prints a deprecation notice when `libharfbuzz-subset` is missing and falls back to fontTools.
To silence it, install `libharfbuzz-subset0` (Debian/Ubuntu).
