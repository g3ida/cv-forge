"""Several roles at one company: separate entries, the company description once, a project line
to tell them apart, and the titles held in a role on one line."""

from __future__ import annotations

from pathlib import Path

from pypdf import PdfReader

from cv_builder.model import CV
from cv_builder.render import Builder, Options
from cv_builder.view import build_sections

ABOUT = "Makes software for cars."
CV_DATA = {
    "basics": {"name": "Ada Example"},
    "work": [
        {
            "name": "Acme GmbH",
            "position": "Tech Lead",
            "project": "In-flight entertainment app",
            "description": ABOUT,
            "startDate": "2026-07",
            "highlights": ["Led the team."],
        },
        {
            "name": "ACME  GmbH",  # same company, spelled with other case and spacing
            "position": "Principal Engineer",
            "project": "Infotainment platform",
            "description": ABOUT,
            "startDate": "2022-11",
            "endDate": "2026-06",
            "titleHistory": [
                {"position": "Engineer", "startDate": "2022-11", "endDate": "2023-06"},
                {"position": "Senior Engineer", "startDate": "2023-07", "endDate": "2024-06"},
                {"position": "Principal Engineer", "startDate": "2024-07", "endDate": "2026-06"},
            ],
            "highlights": ["Built HAL components."],
        },
        {
            "name": "Initech",
            "position": "Engineer",
            "description": ABOUT,
            "startDate": "2020-01",
            "endDate": "2022-10",
            "titleHistory": [{"position": "Engineer", "startDate": "2020-01"}],
        },
    ],
}


def _work():
    return build_sections(CV.model_validate(CV_DATA), {}, [])["work"].items


def test_roles_at_one_company_stay_separate_entries():
    first, second, other = _work()
    assert [e.subtitle for e in (first, second)] == ["Acme GmbH", "ACME  GmbH"]
    assert (first.same_employer, second.same_employer, other.same_employer) == (False, True, False)


def test_company_description_is_printed_once_per_run():
    first, second, other = _work()
    assert ABOUT in first.paragraphs[0]
    assert not any(ABOUT in p for p in second.paragraphs)
    assert ABOUT in other.paragraphs[0], "another company with the same text keeps its own"


def test_project_and_titles_are_printed():
    first, second, other = _work()
    assert "In-flight entertainment app" in first.paragraphs[1]
    assert second.paragraphs[0].endswith("Infotainment platform</span>")
    assert second.paragraphs[1] == "Engineer → Senior Engineer → Principal Engineer"
    assert other.paragraphs == [ABOUT], "a single title is not repeated as a line"


def test_clear_pdf_keeps_the_company_on_every_role(tmp_path: Path):
    pdf = tmp_path / "cv.pdf"
    Builder(CV.model_validate(CV_DATA), tmp_path, Options(template="clear")).write_pdf(pdf)
    text = "\n".join(page.extract_text() for page in PdfReader(pdf).pages)
    assert text.count("Acme GmbH") + text.count("ACME GmbH") == 2
    assert text.count(ABOUT) == 2  # once for Acme, once for Initech
    assert "Engineer → Senior Engineer → Principal Engineer" in text
