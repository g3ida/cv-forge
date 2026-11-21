"""Every bundled template: it renders cleanly, and the ones marked readingOrder read back in order."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import pytest
from pypdf import PdfReader

from cv_builder.cli import load_cv
from cv_builder.model import CV, SECTION_IDS
from cv_builder.render import Builder, Options, Template, available_templates
from test_clear import FULL, squash

ROOT = Path(__file__).resolve().parent.parent
EXAMPLE = ROOT / "examples" / "android-hmi-architect.json"
TEMPLATES = [t.name for t in available_templates()]
READING_ORDER = [t.name for t in available_templates() if t.reading_order]


def _texts(pdf: Path) -> dict[str, str]:
    texts = {"pypdf": "\n".join(page.extract_text() for page in PdfReader(pdf).pages)}
    if shutil.which("pdftotext"):
        texts["pdftotext"] = subprocess.run(["pdftotext", str(pdf), "-"], check=True, capture_output=True, text=True).stdout
    return texts


@pytest.mark.parametrize("name", TEMPLATES)
def test_template_describes_itself(name):
    t = Template.load(name)
    assert t.label and t.label[0].isupper()
    assert len(t.description) > 40, "the picker shows this sentence"
    assert isinstance(t.config.get("readingOrder"), bool)
    placed = [sid for sections in t.config["columns"].values() for sid in sections]
    assert sorted(placed) == sorted(SECTION_IDS), "every section has a default place, once"
    assert t.main_column in t.column_names


@pytest.mark.parametrize("name", TEMPLATES)
@pytest.mark.parametrize("photo", [True, False], ids=["photo", "no-photo"])
def test_template_renders_the_example_without_problems(tmp_path, name, photo):
    cv = load_cv(EXAMPLE)
    if not photo:
        cv.layout.header.photo = False
    result = Builder(cv, EXAMPLE.parent, Options(template=name)).write_pdf(tmp_path / "cv.pdf")
    problems = [n for n in result.notes if "moved into them" not in n]
    assert problems == [] and not result.overflows
    assert 1 <= result.pages <= 4
    for page in PdfReader(tmp_path / "cv.pdf").pages:
        for font in _fonts(page):
            descriptor = font["/FontDescriptor"]
            assert descriptor.get("/FontFile2") or descriptor.get("/FontFile3"), f"{font.get('/BaseFont')} is not embedded"


def _fonts(page):
    for font in (page["/Resources"].get("/Font") or {}).values():
        font = font.get_object()
        for descendant in font.get("/DescendantFonts", [font]):
            yield descendant.get_object()


@pytest.mark.parametrize("name", TEMPLATES)
def test_template_renders_every_section(tmp_path, name):
    data = {**FULL, "layout": {}}
    result = Builder(CV.model_validate(data), tmp_path, Options(template=name)).write_pdf(tmp_path / "cv.pdf")
    assert [n for n in result.notes if "not in layout.columns" in n] == []
    text = squash(_texts(tmp_path / "cv.pdf")["pypdf"]).casefold()
    for piece in ("Jane Q. Doe", "Senior Backend Engineer", "Example Corp", "Built the event pipeline", "Technical University",
                  "Certified Kubernetes Administrator", "On Consensus", "Bouldering", "Jane is a careful engineer",
                  "tinyraft", "Latency Reduction", "Best Paper", "Code Club", "Native"):
        assert squash(piece).casefold() in text, piece


@pytest.mark.parametrize("name", TEMPLATES)
def test_another_templates_columns_are_moved_in(tmp_path, name):
    data = {**FULL, "layout": {"template": "compact", "columns": {"left": ["work"], "right": ["skills", "languages"]}}}
    result = Builder(CV.model_validate(data), tmp_path, Options(template=name)).write_pdf(tmp_path / "cv.pdf")
    text = squash(_texts(tmp_path / "cv.pdf")["pypdf"]).casefold()
    assert "examplecorp" in text and "kubernetes" in text and "native" in text
    assert "technicaluniversity" not in text and "codeclub" not in text  # sections the CV did not choose
    if name != "compact":
        assert any("moved into them" in n for n in result.notes) == (Template.load(name).column_names != ["left", "right"])


# markers in reading order: header, then each section's heading and its pieces
FULL_MARKERS = [
    "Jane Q. Doe", "Staff Engineer | Platform | Go", "jane.doe@example.com",
    "Summary", "Backend engineer with", "Second paragraph",
    "Experience", "Senior Backend Engineer", "Example Corp", "Mar 2022 - Present", "Payments platform.", "Owns the ledger.",
    "Cut latency to 180ms", "to Kubernetes", "Software Engineer", "Startup GmbH", "Jan 2019 - Feb 2022",
    "First backend hire.", "Built the event pipeline",
    "Education", "MSc in Computer Science", "Technical University", "2016 - 2018", "Thesis on consensus protocols",
    "Skills", "Languages", "Go, Python, SQL", "Infrastructure", "Kubernetes, Terraform",
]


@pytest.mark.parametrize("name", READING_ORDER)
def test_reading_order_templates_extract_in_order(tmp_path, name):
    data = {**FULL, "layout": {}}
    Builder(CV.model_validate(data), tmp_path, Options(template=name)).write_pdf(tmp_path / "cv.pdf")
    for extractor, text in _texts(tmp_path / "cv.pdf").items():
        haystack, pos = squash(text), 0
        for marker in FULL_MARKERS:
            found = haystack.find(squash(marker), pos)
            assert found != -1, f"{extractor}: {marker!r} missing or out of order"
            pos = found + len(squash(marker))


def test_double_plus_is_kept_in_one_piece():
    """Unicode allows a line break inside "C++"; a parser then reads "C+ +"."""
    from cv_builder.view import NOBR, inline, keep

    html = str(inline("Built in C++17 with [docs](https://example.com/c++/ref) and **g++**"))
    assert html.count(NOBR) == 2  # C++17 and g++, not the link
    assert 'href="https://example.com/c++/ref"' in html
    assert str(keep("C++ & Rust")) == f"C{NOBR} &amp; Rust"
    assert keep(None) is None


def _text_sizes_pt(pdf: Path) -> set[float]:
    """The size of every run of text, in pt (WeasyPrint draws in CSS px under a 0.75 scale)."""
    import re

    sizes = set()
    for page in PdfReader(pdf).pages:
        data = page.get_contents().get_data().decode("latin-1")
        sizes |= {round(float(m) * 0.75, 2) for m in re.findall(r"/\S+ ([\d.]+) Tf", data)}
    return sizes


@pytest.mark.parametrize("name", TEMPLATES)
def test_no_text_is_smaller_than_10pt(tmp_path, name):
    """CV guides put body text at 10 to 12pt and contact details no smaller; nothing goes below."""
    data = {**FULL, "layout": {}}
    Builder(CV.model_validate(data), tmp_path, Options(template=name)).write_pdf(tmp_path / "cv.pdf")
    sizes = _text_sizes_pt(tmp_path / "cv.pdf")
    assert sizes and min(sizes) >= 10 - 0.01, sorted(sizes)


@pytest.mark.parametrize("name", TEMPLATES)
def test_certificates_and_awards_take_one_line_each(tmp_path, name):
    data = {**FULL, "layout": {}}
    Builder(CV.model_validate(data), tmp_path, Options(template=name)).write_pdf(tmp_path / "cv.pdf")
    text = squash(_texts(tmp_path / "cv.pdf")["pypdf"])
    assert squash("Certified Kubernetes Administrator, CNCF, 2023") in text
    assert squash("Best Paper, ACM, 2021") in text and squash("For the Raft paper.") in text
    reader = PdfReader(tmp_path / "cv.pdf")
    links = [a.get_object()["/A"].get("/URI") for page in reader.pages for a in page.get("/Annots", []) or []]
    assert "https://cncf.io/cka" in links  # the link stays, on the name, without printing the address
    assert "cncf.io/cka" not in text.casefold()
