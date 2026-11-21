"""Fitting to a page limit: tighten line spacing within the usual range, else say how much has to go.

Text size and margins never change: CV guidance puts body text at 10 to 12pt and margins at 0.5
to 1 inch, and says to cut content before shrinking either. Line spacing may go from the
template's own down to 1.15, the low end of the usual 1.15 to 1.5.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

import pytest
from pypdf import PdfReader

from cv_builder.cli import load_cv, main
from cv_builder.model import CV
from cv_builder.render import LINE_HEIGHT_FLOOR, TEXT_LEADING_MAX, Builder, Options, Template, available_templates
from test_clear import FULL, squash
from test_templates import FULL_MARKERS, _fonts, _texts

ROOT = Path(__file__).resolve().parent.parent
EXAMPLE = ROOT / "examples" / "android-hmi-architect.json"
SENTENCE = "Designed and shipped a service that other teams relied on every day. "


def _cv(bullets: int, template: str = "clear", **layout) -> CV:
    """``bullets`` bullets, three to a role (compact never splits a role across pages)."""
    roles = [
        {"name": f"Example Corp {n}", "position": "Engineer", "startDate": "2020",
         "highlights": [SENTENCE * 2] * min(3, bullets - n)}
        for n in range(0, bullets, 3)
    ]
    return CV.model_validate(
        {
            "basics": {"name": "Fit Example", "label": "Engineer", "email": "fit@example.com"},
            "work": roles,
            "layout": {"template": template, **layout},
        }
    )


def _just_over_one_page(template: str = "clear") -> int:
    """The fewest bullets that spill onto a second page as designed."""
    low, high = 1, 120  # one page / more than one
    while high - low > 1:
        mid = (low + high) // 2
        if Builder(_cv(mid, template), ROOT)._pages_at_designed() > 1:
            high = mid
        else:
            low = mid
    return high


def _pages_at_designed(self) -> int:
    self.tightness = 0.0
    return self._paginate_units().pages


Builder._pages_at_designed = _pages_at_designed  # test helper


def _font_sizes(pdf: Path) -> set[float]:
    """Every font size set in the PDF's content streams (Tf operators)."""
    sizes = set()
    for page in PdfReader(pdf).pages:
        data = page.get_contents().get_data().decode("latin-1")
        sizes |= {round(float(m), 2) for m in re.findall(r"/\S+ ([\d.]+) Tf", data)}
    return sizes


def test_a_cv_within_the_limit_is_drawn_as_designed(tmp_path):
    result = Builder(_cv(3), ROOT, Options(max_pages=2)).write_pdf(tmp_path / "cv.pdf")
    fit = result.fit
    assert fit.fits and fit.tightness == 0 and fit.line_height == fit.designed_line_height == 1.36
    assert fit.text_pt == 10.0 and result.pages == 1 and result.notes == []
    assert 0 < result.last_page_fill < 1


def test_a_few_lines_over_gets_tighter_line_spacing_and_nothing_else(tmp_path):
    bullets = _just_over_one_page()
    plain = Builder(_cv(bullets), ROOT).write_pdf(tmp_path / "plain.pdf")
    assert plain.pages == 2 and plain.fit is None

    result = Builder(_cv(bullets), ROOT, Options(max_pages=1)).write_pdf(tmp_path / "fit.pdf")
    fit = result.fit
    assert result.pages == 1 and fit.fits and result.notes == []
    assert 0 < fit.tightness <= 1 and LINE_HEIGHT_FLOOR <= fit.line_height < fit.designed_line_height
    # the text and the page are exactly as designed
    assert _font_sizes(tmp_path / "fit.pdf") == _font_sizes(tmp_path / "plain.pdf")
    reader = PdfReader(tmp_path / "fit.pdf")
    assert [round(float(x)) for x in reader.pages[0].mediabox[2:]] == [595, 842]
    assert squash(SENTENCE) in squash(reader.pages[0].extract_text())
    for font in _fonts(reader.pages[0]):
        assert font["/FontDescriptor"].get("/FontFile2") or font["/FontDescriptor"].get("/FontFile3")
    # the least tightening that fits: noticeably less would not
    looser = Builder(_cv(bullets), ROOT)
    assert not looser._fits_at(max(0.0, fit.tightness - 0.05), 1)


def test_the_layout_sets_the_limit_and_the_option_overrides_it(tmp_path):
    bullets = _just_over_one_page()
    by_layout = Builder(_cv(bullets, maxPages=1), ROOT).write_pdf(tmp_path / "a.pdf")
    assert by_layout.pages == 1 and by_layout.fit.max_pages == 1
    overridden = Builder(_cv(bullets, maxPages=1), ROOT, Options(max_pages=2)).write_pdf(tmp_path / "b.pdf")
    assert overridden.pages == 2 and overridden.fit.tightness == 0


def test_too_much_says_how_many_lines_to_cut_and_is_drawn_as_designed(tmp_path):
    cv = load_cv(EXAMPLE)
    plain = Builder(cv, EXAMPLE.parent, Options(template="clear")).write_pdf(tmp_path / "plain.pdf")
    result = Builder(cv, EXAMPLE.parent, Options(template="clear", max_pages=1)).write_pdf(tmp_path / "cv.pdf")
    fit = result.fit
    assert not fit.fits and fit.tightness == 0 and result.pages == plain.pages
    assert fit.over_lines > 50 and 80 < fit.line_chars < 160
    assert any(n.startswith("does not fit on 1 page: about") and "1.15" in n for n in result.notes)
    two = Builder(cv, EXAMPLE.parent, Options(template="clear", max_pages=2)).write_pdf(tmp_path / "two.pdf")
    assert not two.fit.fits and two.fit.over_lines < fit.over_lines


def test_fewest_pages_is_the_tightest_layout(tmp_path):
    cv = load_cv(EXAMPLE)
    builder = Builder(cv, EXAMPLE.parent, Options(template="classic"))
    fewest = builder.fewest_pages()
    assert builder.tightness == 0  # left as designed
    assert Builder(cv, EXAMPLE.parent, Options(template="classic", max_pages=fewest)).fit().fits
    assert not Builder(cv, EXAMPLE.parent, Options(template="classic", max_pages=fewest - 1)).fit().fits


@pytest.mark.parametrize("name", [t.name for t in available_templates()])
def test_only_text_line_spacing_is_tightened(name):
    builder = Builder(_cv(3, name), ROOT)
    designed = (Template.load(name).path / "style.css").read_text()
    builder.tightness = 1.0
    tightest = builder._template_css()
    values = re.findall(r"line-height\s*:\s*([\d.]+)(?=\s*[;}])", tightest)
    assert all(not (LINE_HEIGHT_FLOOR + 1e-6 < float(v) <= TEXT_LEADING_MAX) for v in values)
    # everything else, lengths in pt included, is untouched
    strip = lambda css: re.sub(r"line-height\s*:\s*[\d.]+(?=\s*[;}])", "", css)  # noqa: E731
    assert strip(tightest) == strip(designed)


def test_an_item_taller_than_a_page_counts_every_page_it_runs_over(tmp_path):
    cv = CV.model_validate({"basics": {"name": "Huge"}, "work": [
        {"name": "Huge Corp", "position": "Engineer", "highlights": [SENTENCE * 3] * 40}], "layout": {"template": "compact"}})
    builder = Builder(cv, ROOT, Options(max_pages=1))
    result = builder.write_pdf(tmp_path / "cv.pdf")
    assert result.overflows and not result.fit.fits and result.fit.over_lines > 20
    assert result.pages > 1 and builder._paginate_units().pages > 1
    # an oversized item does not fit however many pages are allowed
    assert not Builder(cv, ROOT, Options(max_pages=20)).fit().fits


@pytest.mark.parametrize("name", [t.name for t in available_templates() if t.reading_order])
def test_the_tightest_spacing_keeps_the_reading_order(tmp_path, name):
    builder = Builder(CV.model_validate({**FULL, "layout": {}}), tmp_path, Options(template=name))
    builder.tightness = 1.0  # as tight as it ever gets
    builder.write_pdf(tmp_path / "cv.pdf")
    for extractor, text in _texts(tmp_path / "cv.pdf").items():
        haystack, pos = squash(text), 0
        for marker in FULL_MARKERS:
            found = haystack.find(squash(marker), pos)
            assert found != -1, f"{extractor}: {marker!r} missing or out of order"
            pos = found + len(squash(marker))


def test_command_line_fits_and_reports(tmp_path, capsys):
    bullets = _just_over_one_page()
    path = tmp_path / "cv.json"
    path.write_text(json.dumps(_cv(bullets).model_dump(mode="json", exclude_none=True, by_alias=True)))
    assert main(["build", str(path), "--max-pages", "1"]) == 0
    out = capsys.readouterr().out
    assert "(1 page, line spacing 1." in out and "instead of 1.36 to fit" in out
    path.write_text(json.dumps(_cv(60).model_dump(mode="json", exclude_none=True, by_alias=True)))
    assert main(["build", str(path), "-q", "--max-pages", "1"]) == 3
    assert "exceeds the limit of 1" in capsys.readouterr().err
