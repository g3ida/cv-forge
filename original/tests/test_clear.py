"""The clear template (formerly single-column): the text of the PDF must come out in plain reading order.

Every order check runs with two extractors, pypdf and ``pdftotext`` without ``-layout``.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import unicodedata
from pathlib import Path

import pytest
from pypdf import PdfReader

from cv_builder.cli import load_cv, main
from cv_builder.dates import format_range
from cv_builder.model import CV
from cv_builder.render import Builder, Options, RenderError, Template, available_templates
from cv_builder.view import Entry, Section, pretty_url, split_entries

ROOT = Path(__file__).resolve().parent.parent
EXAMPLE = ROOT / "examples" / "android-hmi-architect.json"

# Characters the template or the shared view add on their own: the bullet, the separator
# in "Group · level" / "role · entity", and the quotes around a reference.
GENERATED = set("•·“”")


# ------------------------------------------------------------------ extraction


def _pypdf(pdf: Path) -> str:
    return "\n".join(page.extract_text() for page in PdfReader(pdf).pages)


def _pdftotext(pdf: Path) -> str:
    return subprocess.run(["pdftotext", str(pdf), "-"], check=True, capture_output=True, text=True).stdout


EXTRACTORS = [
    pytest.param(_pypdf, id="pypdf"),
    pytest.param(
        _pdftotext,
        id="pdftotext",
        marks=pytest.mark.skipif(shutil.which("pdftotext") is None, reason="pdftotext is not installed"),
    ),
]


def text_lines(text: str) -> list[str]:
    return [" ".join(line.split()) for line in text.replace("\f", "\n").splitlines() if line.strip()]


def squash(text: str) -> str:
    """Drop all whitespace, so a marker matches wherever its line was wrapped."""
    return "".join(text.split())


def assert_order(text: str, markers: list[str]) -> None:
    """Every marker occurs in one piece, after the end of the one before it."""
    haystack, pos, previous = squash(text), 0, "(start of text)"
    for marker in markers:
        found = haystack.find(squash(marker), pos)
        assert found != -1, f"{marker!r} is missing, broken up, or not after {previous!r}"
        pos, previous = found + len(squash(marker)), marker


def assert_clean(text: str, source: str) -> None:
    """No private-use, control or replacement characters, and nothing the source did not contain."""
    for ch in set(text) - set("\n\f"):
        assert unicodedata.category(ch) not in {"Co", "Cn", "Cc", "Cf", "Cs"}, f"U+{ord(ch):04X} in extracted text"
        assert ch != "�"
    stray = {ch for ch in text if not ch.isascii()} - {ch for ch in source if not ch.isascii()} - GENERATED
    assert not stray, f"characters that are not in the CV: {sorted(stray)}"


def assert_bullets(text: str, count: int) -> None:
    """Each highlight starts its own line with "• "; the bullet shows up nowhere else."""
    lines = text_lines(text)
    assert sum(line.startswith("• ") for line in lines) == count
    assert sum(line.count("•") for line in lines) == count


def entry_markers(position, company, location, dates, paragraphs, highlights) -> list[str]:
    """Reading order inside an entry: position, company, dates alone, location, text, highlights."""
    return [m for m in (position, company, dates, location, *paragraphs) if m] + [f"• {h}" for h in highlights]


def image_count(pdf: Path) -> int:
    """Images on page 1, also those inside form XObjects and patterns (a CSS background image is one)."""
    seen: set[int] = set()

    def walk(resources) -> int:
        found = 0
        for kind in ("/XObject", "/Pattern"):
            for ref in (resources.get(kind) or {}).values():
                if ref.idnum in seen:
                    continue
                seen.add(ref.idnum)
                obj = ref.get_object()
                if obj.get("/Subtype") == "/Image":
                    found += 1
                elif "/Resources" in obj:
                    found += walk(obj["/Resources"])
        return found

    return walk(PdfReader(pdf).pages[0]["/Resources"])


def build(cv: CV, base_dir: Path, target: Path, template: str | None = None):
    return Builder(cv, base_dir, Options(template=template)).write_pdf(target)


# ------------------------------------------------------------------ the example CV


@pytest.fixture(scope="module")
def example(tmp_path_factory):
    """The example rendered with `-t clear`: its own layout block is written for compact."""
    pdf = tmp_path_factory.mktemp("example") / "cv.pdf"
    result = build(load_cv(EXAMPLE), EXAMPLE.parent, pdf, template="clear")
    return pdf, result


def test_example_renders_from_the_command_line(tmp_path, capsys):
    out = tmp_path / "cv.pdf"
    assert main(["build", str(EXAMPLE), "-t", "clear", "-o", str(out), "-q"]) == 0
    err = capsys.readouterr().err
    assert "layout.columns names ['left', 'right']" in err and "moved into them" in err
    reader = PdfReader(out)
    assert len(reader.pages) == 4  # 10pt body text; three pages at the 9pt it had before
    assert reader.metadata.title == load_cv(EXAMPLE).layout.documentTitle


def test_example_has_no_layout_problems(example):
    _, result = example
    assert result.pages == 4
    assert not result.overflows
    assert not [note for note in result.notes if "layout.columns" not in note]


@pytest.mark.parametrize("extract", EXTRACTORS)
def test_example_reading_order(example, extract):
    pdf, _ = example
    cv = load_cv(EXAMPLE)
    b = cv.basics
    text = extract(pdf)
    lines = text_lines(text)

    # Header: name, headline, then the contacts the example asks for (phone, email, url, LinkedIn).
    assert lines[0] == b.name
    assert lines[1] == b.label
    linkedin, github, hugging_face = (pretty_url(p.url) for p in b.profiles)
    markers = [b.name, b.label, b.phone, b.email, pretty_url(b.url), linkedin]

    markers += ["Summary", b.summary, "Experience"]
    for w in cv.work:
        markers += entry_markers(
            w.position, w.name, w.location, format_range(w.startDate, w.endDate),
            [w.description, w.summary], w.highlights,
        )
    markers.append("Education")
    for e in cv.education:
        markers += entry_markers(
            f"{e.studyType} in {e.area}", e.institution, e.location, format_range(e.startDate, e.endDate),
            [], e.highlights,
        )
    markers.append("Skills")
    markers += [f"{s.name}: {', '.join(s.keywords)}" for s in cv.skills]
    markers.append("Projects")
    for p in cv.projects:
        markers += [p.name, format_range(p.startDate, p.endDate), p.url, p.description]
    markers.append("Languages")
    markers += [f"{lang.language}: {lang.fluency}" for lang in cv.languages]
    markers.append("Key Achievements")
    for a in cv.achievements:
        markers += [a.title, a.summary]
    markers += ["Find Me Online", f"GitHub: {github}", f"Hugging Face: {hugging_face}"]
    assert_order(text, markers)

    # Each heading is a line of its own and is printed once, also when its section runs over a page.
    headings = {"Summary", "Experience", "Education", "Skills", "Projects", "Certifications", "Awards",
                "Languages", "Key Achievements", "Find Me Online"}
    assert [line for line in lines if line in headings] == [
        "Summary", "Experience", "Education", "Skills", "Projects", "Languages", "Key Achievements",
        "Find Me Online",
    ]
    # Position, company, the dates alone, then the location: four consecutive lines. A parser
    # attaches whatever shares a line with the dates to the role, so nothing does.
    at = lines.index("Principal Engineer - In-Flight Entertainment")
    assert lines[at + 1 : at + 4] == ["LUMEN FIELD GmbH", "Jul 2026 - Present", "Munich, Germany"]
    at = lines.index("Full Stack Developer")
    assert lines[at + 1 : at + 3] == ["Ledgerly (Spendo)", "Jul 2020 - Jul 2022"]

    assert_bullets(text, sum(len(x.highlights) for x in (*cv.work, *cv.education)))
    assert_clean(text, EXAMPLE.read_text(encoding="utf-8"))


def test_fonts_are_embedded_and_text_is_mapped_to_unicode(example):
    pdf, _ = example
    fonts = {}
    for page in PdfReader(pdf).pages:
        for font in page["/Resources"]["/Font"].values():
            font = font.get_object()
            fonts[str(font["/BaseFont"])] = font
    assert fonts
    for name, font in fonts.items():
        descendant = font["/DescendantFonts"][0].get_object() if "/DescendantFonts" in font else font
        descriptor = descendant["/FontDescriptor"].get_object()
        assert {"/FontFile", "/FontFile2", "/FontFile3"} & set(descriptor), f"{name} is not embedded"
        assert "/ToUnicode" in font, f"{name} has no ToUnicode map"


def test_photo_does_not_change_the_text(tmp_path):
    cv = load_cv(EXAMPLE)
    assert cv.basics.image and cv.layout.header.photo
    hidden = cv.model_copy(deep=True)
    hidden.layout.header.photo = False
    with_photo, without_photo = tmp_path / "photo.pdf", tmp_path / "plain.pdf"
    build(cv, EXAMPLE.parent, with_photo, template="clear")
    build(hidden, EXAMPLE.parent, without_photo, template="clear")
    assert image_count(with_photo) == 1
    assert image_count(without_photo) == 0
    # The header lines are narrower next to the photo, so only the wrapping may differ.
    assert squash(_pypdf(with_photo)) == squash(_pypdf(without_photo))
    assert text_lines(_pypdf(with_photo))[:2] == [cv.basics.name, cv.basics.label]


# ------------------------------------------------------------------ every section


FULL = {
    "basics": {
        "name": "Jane Q. Doe",
        "label": "Staff Engineer | Platform | Go",
        "email": "jane.doe@example.com",
        "phone": "+1 555 0100",
        "url": "https://janedoe.dev",
        "summary": "Backend engineer with **reliable** services.\n\nSecond paragraph: office, affinity, 1x2 -> fi fl.",
        "location": {"city": "Berlin", "countryCode": "DE"},
        "profiles": [
            {"network": "LinkedIn", "username": "janedoe", "url": "https://www.linkedin.com/in/janedoe"},
            {"network": "GitHub", "username": "janedoe", "url": "https://github.com/janedoe"},
        ],
    },
    "work": [
        {
            "name": "Example Corp", "position": "Senior Backend Engineer", "location": "Berlin, Germany",
            "startDate": "2022-03", "description": "Payments platform.", "summary": "Owns the ledger.",
            "highlights": ["Cut latency to 180ms", "Moved [40 services](https://example.com/x) to Kubernetes"],
        },
        {
            "name": "Startup GmbH", "position": "Software Engineer", "startDate": "2019-01", "endDate": "2022-02",
            "summary": "First backend hire.", "highlights": ["Built the event pipeline"],
        },
    ],
    "volunteer": [
        {"organization": "Code Club", "position": "Mentor", "location": "Berlin", "startDate": "2020",
         "endDate": "2021", "summary": "Taught children to program.", "highlights": ["Ran weekly sessions"]}
    ],
    "education": [
        {"institution": "Technical University", "studyType": "MSc", "area": "Computer Science",
         "location": "Munich, Germany", "startDate": "2016", "endDate": "2018", "score": "1.3",
         "courses": ["Distributed Systems", "Compilers"], "highlights": ["Thesis on consensus protocols"]}
    ],
    "awards": [{"title": "Best Paper", "date": "2021-06", "awarder": "ACM", "summary": "For the Raft paper."}],
    "certificates": [
        {"name": "Certified Kubernetes Administrator", "issuer": "CNCF", "date": "2023-05",
         "url": "https://cncf.io/cka"}
    ],
    "publications": [
        {"name": "On Consensus", "publisher": "USENIX", "releaseDate": "2019-10", "url": "https://usenix.org/p",
         "summary": "A study of leader election."}
    ],
    "skills": [
        {"name": "Languages", "level": "Expert", "keywords": ["Go", "Python", "SQL"]},
        {"name": "Infrastructure", "keywords": ["Kubernetes", "Terraform"]},
    ],
    "languages": [{"language": "English", "fluency": "Native"}, {"language": "German", "fluency": "B2"}],
    "interests": [{"name": "Climbing", "keywords": ["Bouldering", "Alpine"]}],
    "references": [{"name": "John Smith", "reference": "Jane is a careful engineer."}],
    "projects": [
        {"name": "tinyraft", "startDate": "2021", "endDate": "2021", "url": "https://github.com/janedoe/tinyraft",
         "description": "Educational Raft implementation.", "highlights": ["1,500 lines of Go"],
         "keywords": ["Go", "Raft"], "roles": ["Author"], "entity": "Open Source"}
    ],
    "achievements": [{"title": "Latency Reduction", "summary": "Made checkout 5x faster.", "icon": "zap"}],
    "layout": {"template": "clear"},
}

FULL_ORDER = [
    # header: name, headline, then email, phone, location, url and every profile (at 10pt the
    # contact line wraps after the separator, so each contact stays whole)
    "Jane Q. Doe", "Staff Engineer | Platform | Go",
    "jane.doe@example.com | +1 555 0100 | Berlin, DE | janedoe.dev | linkedin.com/in/janedoe |",
    "github.com/janedoe",
    "Summary", "Backend engineer with reliable services.", "Second paragraph: office, affinity, 1x2 -> fi fl.",
    "Experience",
    "Senior Backend Engineer", "Example Corp", "Mar 2022 - Present", "Berlin, Germany",
    "Payments platform.", "Owns the ledger.", "• Cut latency to 180ms", "• Moved 40 services to Kubernetes",
    "Software Engineer", "Startup GmbH", "Jan 2019 - Feb 2022", "First backend hire.", "• Built the event pipeline",
    "Education",
    "MSc in Computer Science", "Technical University", "2016 - 2018", "Munich, Germany | 1.3",
    "• Thesis on consensus protocols", "Distributed Systems, Compilers",
    "Skills", "Languages · Expert: Go, Python, SQL", "Infrastructure: Kubernetes, Terraform",
    "Projects",
    "tinyraft", "Author · Open Source", "2021", "https://github.com/janedoe/tinyraft",
    "Educational Raft implementation.", "• 1,500 lines of Go", "Go, Raft",
    "Certifications", "Certified Kubernetes Administrator, CNCF, 2023",  # one line; the link is on the name
    "Awards", "Best Paper, ACM, 2021", "For the Raft paper.",
    "Languages", "English: Native", "German: B2",
    "Key Achievements", "Latency Reduction", "Made checkout 5x faster.",
    "Volunteering", "Mentor", "Code Club", "2020 - 2021", "Berlin", "Taught children to program.",
    "• Ran weekly sessions",
    "Publications", "On Consensus", "USENIX", "Oct 2019", "usenix.org/p", "A study of leader election.",
    "Interests", "Climbing: Bouldering, Alpine",
    "References", "“Jane is a careful engineer.”", "John Smith",
]


@pytest.fixture(scope="module")
def full(tmp_path_factory):
    tmp = tmp_path_factory.mktemp("full")
    pdf = tmp / "cv.pdf"
    return pdf, build(CV.model_validate(FULL), tmp, pdf)


@pytest.mark.parametrize("extract", EXTRACTORS)
def test_every_section_renders_in_the_default_order(full, extract):
    pdf, result = full
    assert not result.notes  # nothing left unplaced, nothing oversized
    text = extract(pdf)
    lines = text_lines(text)
    assert_order(text, FULL_ORDER)
    # The markers above are whole lines, in this order, with nothing in between.
    assert lines == FULL_ORDER
    assert_bullets(text, 6)
    assert_clean(text, json.dumps(FULL, ensure_ascii=False))


@pytest.mark.parametrize("extract", EXTRACTORS)
def test_layout_columns_sets_the_section_order_and_titles(tmp_path, extract):
    data = {
        **FULL,
        "layout": {
            "template": "clear",
            "columns": {"main": ["skills", "work", "summary"]},
            "titles": {"work": "Professional Experience"},
            "header": {"contacts": ["phone"], "profiles": ["GitHub"]},
        },
    }
    pdf = tmp_path / "cv.pdf"
    result = build(CV.model_validate(data), tmp_path, pdf)
    lines = text_lines(extract(pdf))
    assert lines[:3] == ["Jane Q. Doe", "Staff Engineer | Platform | Go", "+1 555 0100 | github.com/janedoe"]
    assert [line for line in lines if line in {"Skills", "Professional Experience", "Summary", "Education"}] == [
        "Skills", "Professional Experience", "Summary",
    ]
    assert any("'education' has content but is not in layout.columns" in note for note in result.notes)


def test_header_without_headline_keeps_the_name_on_its_own_line(tmp_path):
    # pypdf joins two lines when the gap under large type is too small.
    data = {"basics": {"name": "Jane Q. Doe", "email": "jane.doe@example.com"}, "layout": {"template": "clear"}}
    pdf = tmp_path / "cv.pdf"
    build(CV.model_validate(data), tmp_path, pdf)
    assert text_lines(_pypdf(pdf)) == ["Jane Q. Doe", "jane.doe@example.com"]


# ------------------------------------------------------------------ template selection


def test_template_is_listed_with_one_column(capsys):
    assert {"clear", "compact"} <= {t.name for t in available_templates()}
    assert main(["templates"]) == 0
    out = capsys.readouterr().out
    assert "clear" in out and "columns: main; reads in order" in out and "columns: left, right; for people" in out
    assert "formerly: single-column" in out and "formerly: two-column" in out


def test_template_settings_that_the_reading_order_depends_on():
    clear, compact = Template.load("clear"), Template.load("compact")
    assert clear.column_names == ["main"]
    assert clear.reading_order and not compact.reading_order
    assert not clear.repeat_titles  # a repeated heading would land in the middle of an entry
    assert clear.split_entries
    assert clear.fit_tolerance <= 0  # an overfilled page spills its last lines onto an extra page
    # never overfill: WeasyPrint would move the whole flex row of columns to the next page
    assert (compact.repeat_titles, compact.split_entries, compact.fit_tolerance) == (True, False, -0.1)
    assert Template.load(None).name == "compact"


def test_former_names_still_load():
    assert Template.load("single-column").name == "clear"
    assert Template.load("two-column").name == "compact"
    cv = CV.model_validate({"basics": {"name": "A"}, "layout": {"template": "single-column", "columns": {"main": ["work"]}}})
    assert Builder(cv, Path(".")).template.name == "clear"


def test_columns_of_another_template_are_an_error_in_the_cv_itself(tmp_path):
    cv = CV.model_validate(
        {"basics": {"name": "A"}, "layout": {"template": "clear", "columns": {"left": ["work"]}}}
    )
    with pytest.raises(RenderError, match=r"has columns \['main'\]"):
        Builder(cv, tmp_path)


def test_command_line_template_keeps_the_cvs_sections_in_its_own_columns(tmp_path, capsys):
    data = {**FULL, "layout": {"template": "clear", "columns": {"main": ["work", "skills"]}}}
    path = tmp_path / "cv.json"
    path.write_text(json.dumps(data), encoding="utf-8")
    assert main(["build", str(path), "-t", "compact", "-q"]) == 0
    assert "layout.columns names ['main'] but template 'compact'" in capsys.readouterr().err
    text = _pypdf(path.with_suffix(".pdf"))
    assert "EXPERIENCE" in text and "SKILLS" in text  # the two sections the CV chose
    assert "EDUCATION" not in text and "SUMMARY" not in text  # and nothing else


def test_former_name_on_the_command_line_is_the_same_template(tmp_path, capsys):
    data = {**FULL, "layout": {"template": "clear", "columns": {"main": ["skills", "work"]}}}
    path = tmp_path / "cv.json"
    path.write_text(json.dumps(data), encoding="utf-8")
    assert main(["build", str(path), "-t", "single-column", "-q"]) == 0
    assert "moved into them" not in capsys.readouterr().err  # same template: the CV's own order applies
    text = squash(_pypdf(path.with_suffix(".pdf")))
    assert text.index("Skills") < text.index("Experience")


# ------------------------------------------------------------------ pagination


def test_split_entries_keeps_the_heading_with_the_first_bullet():
    entry = Entry(title="Engineer", subtitle="Corp", paragraphs=["p"], bullets=["a", "b", "c"], tags="x, y")
    short = Entry(title="Intern", bullets=["only"])
    section = split_entries(Section("work", "Experience", "entry", [entry, short]))
    head, second, third, single = section.items
    assert (head.title, head.subtitle, head.paragraphs) == ("Engineer", "Corp", ["p"])
    assert (head.bullets, head.tags) == (["a"], None)
    assert not head.continued
    assert (second.title, second.bullets, second.continued, second.tags) == (None, ["b"], True, None)
    assert (third.bullets, third.continued, third.tags) == (["c"], True, "x, y")
    assert single is short
    assert [section.source_index(i) for i in range(4)] == [0, 0, 0, 1]
    assert entry.bullets == ["a", "b", "c"]  # the original entry is left alone

    skills = Section("skills", "Skills", "tags", [{"name": "Go", "level": None, "text": ""}])
    assert split_entries(skills) is skills


def _long_cv(filler: int) -> CV:
    """Seven long jobs after `filler` short summary paragraphs, which move the page breaks around."""
    return CV.model_validate({
        "basics": {"name": "Page Break", "summary": "\n\n".join(f"Filler paragraph {i}." for i in range(filler))},
        "work": [
            {
                "name": f"Company{n}", "position": f"Position{n}", "location": "City",
                "startDate": "2020-01", "endDate": "2021-02", "summary": f"Summary{n} " + "lorem ipsum " * 20,
                "highlights": [f"Bullet{n}.{k} " + "dolor sit amet " * (4 + 5 * (k % 2)) for k in range(5)],
            }
            for n in range(7)
        ],
        "layout": {"template": "clear"},
    })


def test_page_breaks_fall_between_bullets_but_never_under_a_heading(tmp_path):
    pages_starting_inside_an_entry = 0
    for filler in (0, 4, 8, 12):
        pdf = tmp_path / f"cv{filler}.pdf"
        result = build(_long_cv(filler), tmp_path, pdf)
        assert not result.notes, result.notes  # the planned pages are the pages WeasyPrint produced
        pages = [page.extract_text() for page in PdfReader(pdf).pages]
        assert len(pages) >= 2
        for text in pages:
            for n in range(7):
                if f"Position{n}" in text:  # the whole heading, the summary and the first bullet come along
                    for part in (f"Company{n}", "Jan 2020 - Feb 2021", "City", f"Summary{n}", f"Bullet{n}.0"):
                        assert part in text, f"filler={filler}: Position{n} is on a page without {part}"
        pages_starting_inside_an_entry += sum(text.lstrip().startswith("•") for text in pages[1:])
        whole = "\n".join(pages)
        assert whole.count("Experience") == 1  # the title is not repeated on the pages that follow
        assert_order(whole, [f"Bullet{n}.{k}" for n in range(7) for k in range(5)])
    assert pages_starting_inside_an_entry, "no page break fell inside an entry; the test checks nothing"
