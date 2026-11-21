"""Render a CV to HTML and PDF with WeasyPrint.

Rendering happens in two passes:

1. *Measure*: every column is laid out on one very tall page and the height of
   each item is read from WeasyPrint's layout tree.
2. *Paginate & print*: items are distributed over real pages (see
   :mod:`cv_builder.paginate`) and the final document is rendered to PDF.
"""

from __future__ import annotations

import json
import logging
import math
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from jinja2 import Environment, FileSystemLoader, StrictUndefined
from pydantic import ValidationError
from weasyprint import CSS, HTML
from weasyprint.text.fonts import FontConfiguration
from weasyprint.urls import URLFetcher

from .icons import icon
from .model import CV, SECTION_IDS, HeaderLayout
from .paginate import TOLERANCE, Overflow, Unit, paginate_column
from .view import Header, Section, build_header, build_sections, split_entries

PACKAGE_DIR = Path(__file__).resolve().parent
TEMPLATES_DIR = PACKAGE_DIR / "templates"
DEFAULT_TEMPLATE = "compact"
FONTS_DIR = PACKAGE_DIR / "assets" / "fonts"



class LocalFetcher(URLFetcher):
    """Lets WeasyPrint read only ``data:`` URLs and files under the given folders (the bundled
    templates and fonts, and the CV's own folder). A CV is data its author controls: without
    this, a photo path, a link in a stylesheet or a value pasted into the CSS could make the
    renderer read any file on the machine or request any address on its network."""

    def __init__(self, *roots: Path) -> None:
        super().__init__(allowed_protocols={"file", "data"}, allow_redirects=False)
        self.roots = [r.resolve() for r in roots]

    def fetch(self, url, headers=None):  # noqa: ANN001 - WeasyPrint's signature
        if url.lower().startswith("file:"):
            from urllib.request import url2pathname

            path = Path(url2pathname(urlparse(url).path)).resolve()
            if not any(path == root or root in path.parents for root in self.roots):
                raise ValueError(f"{path} is outside the folders a CV may use")
        return super().fetch(url, headers)


PT_TO_PX = 4 / 3
PAGE_SIZES_PT = {"A4": (595.28, 841.89), "Letter": (612.0, 792.0)}
MEASURE_PAGE_HEIGHT_PT = 14_000
# Fitting to a page limit tightens line spacing and nothing else. CV guidance agrees on cutting
# content before touching the type: text size and margins stay as the template sets them, and
# line spacing goes no lower than 1.15, the low end of the usual 1.15 to 1.5. Past that,
# content has to go. Leading values outside that range (icon boxes, initials) are design, not
# text, and are left alone.
LINE_HEIGHT_FLOOR = 1.15
TEXT_LEADING_MAX = 1.5
FIT_STEPS = 5  # binary-search steps between the template's spacing and the tightest: to within 3%
_LINE_HEIGHT_RE = re.compile(r"(line-height\s*:\s*)(\d*\.?\d+)(\s*[;}])")
FONT_WEIGHTS = {"Thin": 100, "Light": 300, "Regular": 400, "Medium": 500, "SemiBold": 600, "Bold": 700, "Black": 900}

# WeasyPrint logs every unsupported CSS property at WARNING level; keep the CLI quiet.
logging.getLogger("weasyprint").setLevel(logging.ERROR)
logging.getLogger("fontTools").setLevel(logging.ERROR)


class RenderError(Exception):
    pass


# ------------------------------------------------------------------ templates


@dataclass
class Template:
    name: str
    path: Path
    config: dict[str, Any]

    @property
    def label(self) -> str:
        return self.config.get("label") or self.name.replace("-", " ").title()

    @property
    def description(self) -> str:
        return self.config.get("description", "")

    @property
    def reading_order(self) -> bool:
        """The PDF's text extracts in reading order, so resume parsers (ATS) read it correctly."""
        return bool(self.config.get("readingOrder", False))

    @property
    def aliases(self) -> list[str]:
        """Former names, still accepted in ``layout.template`` and ``-t``."""
        return list(self.config.get("aliases", []))

    @property
    def column_names(self) -> list[str]:
        return list(self.config["columns"])

    @property
    def main_column(self) -> str:
        """Where a section goes when the template's defaults do not place it."""
        for name, sections in self.config["columns"].items():
            if "work" in sections:
                return name
        return self.column_names[0]

    def fit_columns(self, columns: dict[str, list[str]]) -> dict[str, list[str]]:
        """Sections laid out for another template, moved into this one's columns.

        The CV decides which sections appear; this template decides where and in what order,
        as in its defaults. Sections its defaults do not list go to the end of the main column.
        """
        chosen = [sid for sections in columns.values() for sid in sections]
        out = {name: [sid for sid in sections if sid in chosen] for name, sections in self.config["columns"].items()}
        placed = {sid for sections in out.values() for sid in sections}
        out[self.main_column] += [sid for sid in chosen if sid not in placed]
        return out

    @property
    def repeat_titles(self) -> bool:
        """Print a section's title again on every page the section continues on."""
        return self.config.get("repeatTitles", True)

    @property
    def split_entries(self) -> bool:
        """Let a page break fall between the bullets of an entry."""
        return self.config.get("splitEntries", False)

    @property
    def fit_tolerance(self) -> float:
        """How far (px) the content of a page may exceed the page; see :mod:`cv_builder.paginate`."""
        return self.config.get("fitTolerance", TOLERANCE)

    @classmethod
    def load(cls, name_or_path: str | Path | None) -> Template:
        name_or_path = name_or_path or DEFAULT_TEMPLATE
        path = Path(name_or_path)
        if not (path / "template.json").is_file():
            path = TEMPLATES_DIR / str(name_or_path)
        if not (path / "template.json").is_file():
            renamed = next((t for t in available_templates() if str(name_or_path) in t.aliases), None)
            if renamed:
                return renamed
            known = ", ".join(t.name for t in available_templates())
            raise RenderError(f"unknown template {name_or_path!r} (available: {known})")
        config = json.loads((path / "template.json").read_text(encoding="utf-8"))
        return cls(name=config.get("name", path.name), path=path, config=config)


def available_templates() -> list[Template]:
    return [Template.load(p) for p in sorted(TEMPLATES_DIR.iterdir()) if (p / "template.json").is_file()]


# ------------------------------------------------------------------ settings


@dataclass
class Options:
    """Command-line overrides; ``None`` keeps the CV's layout or template default."""

    template: str | None = None
    accent: str | None = None
    page_size: str | None = None
    max_pages: int | None = None


@dataclass
class Settings:
    template: Template
    page_size: str
    accent: str
    max_pages: int | None
    columns: dict[str, list[str]]
    titles: dict[str, str]
    document_title: str
    header: HeaderLayout


def resolve_header(cv: CV, template: Template) -> HeaderLayout:
    """``layout.header``, with the template's defaults for the fields the CV leaves out."""
    header = cv.layout.header
    defaults = {k: v for k, v in template.config.get("header", {}).items() if k not in header.model_fields_set}
    if not defaults:
        return header
    if defaults.get("profiles") == "*":  # every profile goes into the header
        defaults["profiles"] = [p.network for p in cv.basics.profiles]
    try:
        return HeaderLayout.model_validate({**header.model_dump(), **defaults})
    except ValidationError as exc:
        raise RenderError(f"template {template.name!r} has invalid header defaults: {exc}") from None


def _other_template(chosen: str | None, laid_out_for: str | None, template: Template) -> bool:
    """Whether ``-t`` picked a different template than the one the CV names (aliases count as the same)."""
    if chosen is None:
        return False
    try:
        return Template.load(laid_out_for).path.resolve() != template.path.resolve()
    except RenderError:
        return True


def resolve_settings(cv: CV, options: Options) -> tuple[Settings, list[str]]:
    layout = cv.layout
    template = Template.load(options.template or layout.template)
    cfg = template.config
    notes: list[str] = []

    columns = layout.columns or cfg["columns"]
    if layout.columns and set(columns) != set(template.column_names):
        if not _other_template(options.template, layout.template, template):
            unknown = set(columns) - set(template.column_names)
            if unknown:
                raise RenderError(
                    f"layout.columns uses {sorted(unknown)} but template {template.name!r} "
                    f"has columns {template.column_names}"
                )
        else:
            # The command line picked another template than the one the CV is laid out for:
            # keep the CV's choice and order of sections, in this template's columns.
            columns = template.fit_columns(columns)
            notes.append(
                f"layout.columns names {sorted(layout.columns)} but template {template.name!r} has columns "
                f"{template.column_names}; its sections were moved into them"
            )
    header = resolve_header(cv, template)
    placed = {s for sections in columns.values() for s in sections}
    sections = build_sections(cv, {}, header.profiles)
    for sid in SECTION_IDS:
        if sid in sections and sid not in placed:
            notes.append(f"section {sid!r} has content but is not in layout.columns; it is not rendered")

    page_size = options.page_size or layout.pageSize or cfg.get("pageSize", "A4")
    return (
        Settings(
            template=template,
            page_size=page_size,
            accent=options.accent or layout.accentColor or cfg.get("accentColor", "#1E90FF"),
            max_pages=options.max_pages or layout.maxPages,
            columns={name: list(columns.get(name, [])) for name in template.column_names},
            titles={**cfg.get("titles", {}), **layout.titles},
            document_title=layout.documentTitle or f"{cv.basics.name} - CV",
            header=header,
        ),
        notes,
    )


# ------------------------------------------------------------------ page model


@dataclass
class Chunk:
    """The part of a section that lands in one column of one page."""

    id: str
    title: str
    kind: str
    continued: bool
    items: list[tuple[int, Any]]


@dataclass
class Page:
    header: bool
    columns: dict[str, list[Chunk]] = field(default_factory=dict)


@dataclass
class Measure:
    """One measure pass, in CSS px."""

    units: dict[str, list[Unit]]
    title_heights: dict[str, float]
    header_height: float
    text_px: float  # the body text's font size
    line_px: float  # and its line height
    line_width_px: float  # the width of the main column


@dataclass
class Pagination:
    """Measured units distributed over pages, per column."""

    columns: dict[str, list[list[Unit]]]
    overflows: list[Overflow]
    title_heights: dict[str, float]  # what a continued section repeats (zero when it repeats nothing)
    header_height: float
    content_height: float

    def _available(self, index: int) -> float:
        return self.content_height - (self.header_height if index == 0 else 0.0)

    def _used(self, page: list[Unit]) -> float:
        if not page:
            return 0.0
        start = page[0]
        return page[-1].bottom - start.top + (0.0 if start.opens_section else self.title_heights[start.section])

    @property
    def pages(self) -> int:
        """Pages the PDF will have. An item taller than a page runs on over the next ones."""
        counts = [1]
        for pages in self.columns.values():
            spill = sum(max(0, math.ceil(self._used(p) / self._available(i) - 1e-6) - 1) for i, p in enumerate(pages))
            counts.append(len(pages) + spill)
        return max(counts)

    def beyond(self, limit: int) -> float:
        """Height of the content that does not fit on the first ``limit`` pages, in the column
        that has the most: what runs past the bottom of those pages, and every page after them."""
        worst = 0.0
        for pages in self.columns.values():
            over = sum(max(0.0, self._used(p) - self._available(i)) for i, p in enumerate(pages[:limit]))
            worst = max(worst, over + sum(self._used(p) for p in pages[limit:]))
        return worst

    def last_page_fill(self) -> float:
        """How full the last page is (0 to 1), in the column that fills it most."""
        n = max([len(p) for p in self.columns.values()] + [1])
        used = [self._used(pages[n - 1]) for pages in self.columns.values() if len(pages) == n]
        return round(min(1.0, max(used, default=0.0) / self._available(n - 1)), 3)


@dataclass
class Fit:
    """What fitting to a page limit (``layout.maxPages`` or ``--max-pages``) did.

    ``tightness`` is how far line spacing went from the template's own (0) toward the floor of
    1.15 (1); ``line_height`` is the body's as drawn, ``designed_line_height`` the template's.
    Text size and margins never change. When the content does not fit even at the tightest, it
    is drawn as designed and ``over_lines`` says about how many lines of body text (of about
    ``line_chars`` characters) have to go.
    """

    max_pages: int
    fits: bool
    tightness: float
    line_height: float
    designed_line_height: float
    text_pt: float  # the body text's size, which fitting never changes
    over_lines: int = 0
    line_chars: int = 0


@dataclass
class BuildResult:
    pages: int
    notes: list[str]
    overflows: list[Overflow]
    fit: Fit | None = None
    last_page_fill: float = 1.0


# ------------------------------------------------------------------ builder


class Builder:
    def __init__(self, cv: CV, base_dir: Path, options: Options | None = None):
        self.cv = cv
        self.base_dir = base_dir
        self.settings, self.notes = resolve_settings(cv, options or Options())
        self.template = self.settings.template
        self.tightness = 0.0  # line spacing: 0 the template's own, 1 the floor (see fit)
        self._measures: dict[float, Measure] = {}
        self.sections: dict[str, Section] = build_sections(
            cv, self.settings.titles, self.settings.header.profiles
        )
        if self.template.split_entries:
            self.sections = {sid: split_entries(section) for sid, section in self.sections.items()}
        self.header: Header = build_header(cv, self.settings.header, self._photo_url())
        self.font_config = FontConfiguration()
        self.fetcher = LocalFetcher(PACKAGE_DIR, self.template.path, self.base_dir)
        self.env = Environment(
            # The template's own files first, then the bundled ones (for "_shared/...").
            loader=FileSystemLoader([str(self.template.path), str(TEMPLATES_DIR)]),
            autoescape=True,
            undefined=StrictUndefined,
            trim_blocks=True,
            lstrip_blocks=True,
        )
        self.env.globals["icon"] = icon

    # -------------------------------------------------------------- inputs

    def _photo_url(self) -> str | None:
        image = self.cv.basics.image
        if not image or not self.settings.header.photo:
            return None
        scheme = urlparse(image).scheme
        if scheme == "data":
            return image
        if scheme:
            self.notes.append(f"photo {image!r} is not a file in the CV's folder; rendering without it")
            return None
        base = self.base_dir.resolve()
        path = (base / image).resolve()
        if base not in path.parents:
            self.notes.append(f"photo {image!r} is outside the CV's folder; rendering without it")
            return None
        if not path.is_file():
            self.notes.append(f"photo {image!r} not found (looked in {path.parent}); rendering without it")
            return None
        return path.as_uri()

    def _template_css(self) -> str:
        """The template's stylesheet, its text line spacing tightened by ``self.tightness``."""
        text = (self.template.path / "style.css").read_text(encoding="utf-8")
        if self.tightness <= 0:
            return text

        def tighten(m: re.Match) -> str:
            value = float(m.group(2))
            if not LINE_HEIGHT_FLOOR < value <= TEXT_LEADING_MAX:
                return m.group(0)
            value -= self.tightness * (value - LINE_HEIGHT_FLOOR)
            return f"{m.group(1)}{value:.4f}{m.group(3)}"

        return _LINE_HEIGHT_RE.sub(tighten, text)

    def _stylesheets(self, measuring: bool) -> list[CSS]:
        width, height = PAGE_SIZES_PT[self.settings.page_size]
        if measuring:
            height = MEASURE_PAGE_HEIGHT_PT
        m = self.template.config["margins"]
        fonts = []
        for f in sorted(FONTS_DIR.glob("*.ttf")):
            # "NotoSerif-BoldItalic.ttf" is family "Noto Serif", weight 700, italic
            family, _, style = f.stem.partition("-")
            family = re.sub(r"(?<=[a-z])(?=[A-Z])", " ", family)
            italic = style.endswith("Italic")
            weight = FONT_WEIGHTS.get(style.removesuffix("Italic") or "Regular", 400)
            fonts.append(
                f'@font-face {{ font-family: "{family}"; font-weight: {weight}; '
                f'font-style: {"italic" if italic else "normal"}; src: url("{f.as_uri()}"); }}'
            )
        # .nobr keeps "++" in one piece (see view.keep)
        page = (
            ".nobr { white-space: nowrap; }\n"
            f"@page {{ size: {width}pt {height}pt; "
            f"margin: {m['top']}pt {m['right']}pt {m['bottom']}pt {m['left']}pt; }}"
        )
        # the page margins as variables, for designs that reach the paper's edge
        accent = (
            f":root {{ --accent: {self.settings.accent}; --margin-top: {m['top']}pt; --margin-right: {m['right']}pt; "
            f"--margin-bottom: {m['bottom']}pt; --margin-left: {m['left']}pt; }}"
        )
        return [
            CSS(string="\n".join(fonts) + "\n" + page, font_config=self.font_config, url_fetcher=self.fetcher),
            CSS(string=self._template_css(), base_url=str(self.template.path / "style.css"), font_config=self.font_config,
                url_fetcher=self.fetcher),
            CSS(string=accent, font_config=self.font_config, url_fetcher=self.fetcher),
        ]

    # -------------------------------------------------------------- html

    def render_html(self, pages: list[Page]) -> str:
        return self.env.get_template("cv.html.j2").render(
            cv=self.cv,
            header=self.header,
            pages=pages,
            column_names=self.template.column_names,
            repeat_titles=self.template.repeat_titles,
            accent=self.settings.accent,
            title=self.settings.document_title,
            lang="en",
        )

    def _document(self, html: str, measuring: bool):
        return HTML(string=html, base_url=str(self.template.path), url_fetcher=self.fetcher).render(
            stylesheets=self._stylesheets(measuring), font_config=self.font_config
        )

    def _single_page(self) -> Page:
        page = Page(header=True)
        for col, section_ids in self.settings.columns.items():
            page.columns[col] = [
                Chunk(s.id, s.title, s.kind, False, list(enumerate(s.items)))
                for s in (self.sections[sid] for sid in section_ids if sid in self.sections)
            ]
        return page

    # -------------------------------------------------------------- layout

    def measure(self) -> Measure:
        """Lay every column out on one tall page and read the height of each item (px)."""
        if self.tightness in self._measures:
            return self._measures[self.tightness]
        doc = self._document(self.render_html([self._single_page()]), measuring=True)
        if len(doc.pages) != 1:
            raise RenderError("content is too long to measure; is something unbounded in the template?")
        page_box = doc.pages[0]._page_box
        boxes: dict[str, Any] = {}
        root = main = None
        main_class = f"col--{self.template.main_column}"
        for box in page_box.descendants():
            element = getattr(box, "element", None)
            if element is None:
                continue
            if root is None and element.tag == "html":
                root = box
            if main is None and main_class in (element.get("class") or "").split():
                main = box
            key = element.get("data-m")
            if key and key not in boxes:
                boxes[key] = box

        def top(b) -> float:
            return b.position_y + b.margin_top

        def bottom(b) -> float:
            return top(b) + b.border_height()

        columns_top = top(boxes["columns"])
        header_height = columns_top - page_box.content_box_y()
        units: dict[str, list[Unit]] = {}
        title_heights: dict[str, float] = {}
        for col, section_ids in self.settings.columns.items():
            units[col] = []
            for sid in section_ids:
                if sid not in self.sections:
                    continue
                section_box = boxes[f"section:{sid}"]
                first_item = boxes[f"item:{sid}:0"]
                title_heights[sid] = top(first_item) - top(section_box)
                for idx in range(len(self.sections[sid].items)):
                    item = boxes[f"item:{sid}:{idx}"]
                    start = top(section_box) if idx == 0 else top(item)
                    units[col].append(Unit(sid, idx, start - columns_top, bottom(item) - columns_top))
        text_px = root.style["font_size"] if root is not None else 12.0
        line_height = root.style["line_height"] if root is not None else "normal"
        if isinstance(line_height, (int, float)):
            line_px = text_px * line_height
        elif getattr(line_height, "unit", None) == "px":
            line_px = line_height.value
        else:  # "normal"
            line_px = text_px * 1.2
        line_width_px = main.width if main is not None else page_box.width
        measure = Measure(units, title_heights, header_height, text_px, line_px, line_width_px)
        self._measures[self.tightness] = measure
        return measure

    def _paginate_units(self) -> Pagination:
        measure = self.measure()
        m = self.template.config["margins"]
        content_height = (PAGE_SIZES_PT[self.settings.page_size][1] - m["top"] - m["bottom"]) * PT_TO_PX
        title_heights = measure.title_heights
        if not self.template.repeat_titles:
            title_heights = dict.fromkeys(title_heights, 0.0)  # nothing to reserve on continuation pages
        per_column: dict[str, list[list[Unit]]] = {}
        overflows: list[Overflow] = []
        for col, col_units in measure.units.items():
            per_column[col], col_overflows = paginate_column(
                col, col_units, title_heights, content_height - measure.header_height, content_height,
                self.template.fit_tolerance,
            )
            overflows += col_overflows
        return Pagination(per_column, overflows, title_heights, measure.header_height, content_height)

    def paginate(self) -> tuple[list[Page], list[Overflow]]:
        laid_out = self._paginate_units()
        pages = []
        for n in range(laid_out.pages):
            page = Page(header=n == 0)
            for col in self.settings.columns:
                chunks: list[Chunk] = []
                col_pages = laid_out.columns.get(col, [])
                for unit in col_pages[n] if n < len(col_pages) else []:
                    section = self.sections[unit.section]
                    if not chunks or chunks[-1].id != unit.section:
                        chunks.append(Chunk(section.id, section.title, section.kind, not unit.opens_section, []))
                    chunks[-1].items.append((unit.index, section.items[unit.index]))
                page.columns[col] = chunks
            pages.append(page)
        return pages, laid_out.overflows

    def _fits_at(self, tightness: float, limit: int) -> bool:
        """Whether the CV fits ``limit`` pages at ``tightness``. An item taller than a page never
        fits: WeasyPrint breaks it over more pages than anyone can predict."""
        self.tightness = tightness
        laid_out = self._paginate_units()
        return not laid_out.overflows and laid_out.pages <= limit

    def _line_height(self) -> float:
        measure = self.measure()
        return round(measure.line_px / measure.text_px, 3)

    def fewest_pages(self) -> int:
        """The fewest pages this content can take under any page limit: at the tightest spacing."""
        self.tightness = 1.0
        pages = self._paginate_units().pages
        self.tightness = 0.0
        return pages

    def fit(self) -> Fit | None:
        """Pick the line spacing for the page limit: the template's own when the CV fits as it
        is, else the least tightening that fits, down to LINE_HEIGHT_FLOOR. Sets
        ``self.tightness``; returns None when there is no limit."""
        limit = self.settings.max_pages
        if not limit:
            return None
        self.tightness = 0.0
        designed = self._line_height()
        text_pt = round(self.measure().text_px / PT_TO_PX, 2)
        if self._fits_at(0.0, limit):
            return Fit(limit, True, 0.0, designed, designed, text_pt)
        if self._fits_at(1.0, limit):
            too_loose, fits = 0.0, 1.0
            for _ in range(FIT_STEPS):
                mid = (too_loose + fits) / 2
                if self._fits_at(mid, limit):
                    fits = mid
                else:
                    too_loose = mid
            self.tightness = fits
            return Fit(limit, True, round(fits, 4), self._line_height(), designed, text_pt)
        # It does not fit. Say how much has to go at the tightest, and draw it as designed.
        self.tightness = 1.0
        measure = self.measure()
        beyond = self._paginate_units().beyond(limit)
        self.tightness = 0.0
        return Fit(
            limit, False, 0.0, designed, designed, text_pt,
            over_lines=max(1, math.ceil(beyond / measure.line_px)),
            line_chars=round(measure.line_width_px / (measure.text_px * 0.5)),
        )

    # -------------------------------------------------------------- output

    def html(self) -> str:
        self.fit()
        pages, _ = self.paginate()
        return self.render_html(pages)

    def write_pdf(self, target: Path, html_target: Path | None = None, *, first_pages: int | None = None) -> BuildResult:
        """``first_pages`` draws only that many pages (a preview of page 1, say): the layout is
        worked out for the whole CV, but WeasyPrint only lays out and writes those pages."""
        fit = self.fit()
        pages, overflows = self.paginate()
        if first_pages:
            pages = pages[:first_pages]
        html = self.render_html(pages)
        if html_target:
            html_target.write_text(html, encoding="utf-8")
        doc = self._document(html, measuring=False)
        doc.write_pdf(target, pdf_tags=True)
        notes = list(self.notes)
        for o in overflows:
            index = self.sections[o.unit.section].source_index(o.unit.index)
            notes.append(
                f"{o.unit.section}[{index}] is taller than a page "
                f"({o.height / PT_TO_PX:.0f}pt > {o.available / PT_TO_PX:.0f}pt) "
                "and overflows; shorten it"
            )
        if fit and not fit.fits:
            pages_word = "page" if fit.max_pages == 1 else "pages"
            notes.append(
                f"does not fit on {fit.max_pages} {pages_word}: about {fit.over_lines} lines too long, even with "
                f"the line spacing at {LINE_HEIGHT_FLOOR}"
            )
        if len(doc.pages) != len(pages):
            notes.append(f"expected {len(pages)} pages but WeasyPrint produced {len(doc.pages)}")
        return BuildResult(
            pages=len(doc.pages), notes=notes, overflows=overflows, fit=fit,
            last_page_fill=self._paginate_units().last_page_fill(),
        )
