import json
from pathlib import Path

import pytest
from pypdf import PdfReader

from cv_builder.cli import main

ROOT = Path(__file__).resolve().parent.parent
EXAMPLE = ROOT / "examples" / "android-hmi-architect.json"


def pdf_text(path: Path) -> list[str]:
    return [page.extract_text() for page in PdfReader(path).pages]


def test_example_paginates_by_column_with_titles_repeated(tmp_path):
    """The reference PDF was two pages at 8.25pt body text; compact now sets 10pt, the minimum CV
    guides give, so the same content runs to three. Each column still flows on by itself and a
    continued section repeats its title once per page."""
    out = tmp_path / "cv.pdf"
    assert main(["build", str(EXAMPLE), "-o", str(out), "-q"]) == 0
    pages = pdf_text(out)
    assert len(pages) == 3
    assert "Jul 2026 - Present" in pages[0]
    assert "Full Stack Developer" in pages[1] and "Full Stack Developer" not in pages[0] + pages[2]
    assert "Research and Development - Computer Vision" in pages[1]
    assert "Algorithms" in pages[1]
    assert [p.count("EXPERIENCE") for p in pages] == [1, 1, 1]
    assert [p.count("SKILLS") for p in pages] == [1, 1, 0]
    meta = PdfReader(out).metadata
    assert meta.title == "Android HMI Architect - Samuel Antoine Remi Duval"


def test_init_then_build_starter(tmp_path, capsys):
    cv_path = tmp_path / "me" / "cv.json"
    assert main(["init", str(cv_path)]) == 0
    assert (tmp_path / "me" / "cv.schema.json").is_file()
    assert main(["init", str(cv_path)]) == 1  # refuses to overwrite
    assert main(["build", str(cv_path), "--page-size", "Letter", "--accent", "#0B7A75"]) == 0
    pdf = cv_path.with_suffix(".pdf")
    reader = PdfReader(pdf)
    assert float(reader.pages[0].mediabox.height) == pytest.approx(792, abs=1)
    text = "".join(pdf_text(pdf))
    assert "Jane Doe".upper() in text
    assert "Certified Kubernetes Administrator" in text
    assert "Mar 2022 - Present" in text


def test_long_content_paginates_and_warns_on_oversized_item(tmp_path, capsys):
    data = {
        "basics": {"name": "Long CV"},
        "work": [
            {"name": f"Company {i}", "position": "Engineer", "startDate": "2020", "highlights": ["x " * 60] * 6}
            for i in range(12)
        ]
        + [{"name": "Huge", "position": "Engineer", "highlights": ["word " * 50] * 80}],
    }
    path = tmp_path / "long.json"
    path.write_text(json.dumps(data))
    assert main(["build", str(path), "-q", "--max-pages", "2"]) == 3
    err = capsys.readouterr().err
    assert "work[12] is taller than a page" in err
    assert "exceeds the limit of 2" in err
    assert "does not fit on 2 pages" in err


def test_unplaced_section_and_missing_photo_warn(tmp_path, capsys):
    data = {
        "basics": {"name": "A", "image": "nope.jpg"},
        "awards": [{"title": "Prize"}],
        "layout": {"columns": {"left": ["work"], "right": ["skills"]}},
    }
    path = tmp_path / "a.json"
    path.write_text(json.dumps(data))
    assert main(["build", str(path), "-q"]) == 0
    err = capsys.readouterr().err
    assert "'awards' has content but is not in layout.columns" in err
    assert "photo 'nope.jpg' not found" in err


def test_invalid_file_reports_location(tmp_path, capsys):
    path = tmp_path / "bad.json"
    path.write_text(json.dumps({"basics": {"name": "A"}, "education": [{"startDate": "2019/01"}]}))
    assert main(["validate", str(path)]) == 1
    assert "education[0].startDate: invalid date '2019/01'" in capsys.readouterr().err


def test_unknown_template(tmp_path, capsys):
    path = tmp_path / "a.json"
    path.write_text(json.dumps({"basics": {"name": "A"}}))
    assert main(["build", str(path), "-t", "nope"]) == 1
    assert "unknown template 'nope'" in capsys.readouterr().err


def test_first_pages_draws_only_those(tmp_path):
    """For previews: the layout is worked out for the whole CV, only page 1 is drawn."""
    from cv_builder.cli import load_cv
    from cv_builder.render import Builder, Options

    cv = load_cv(EXAMPLE)
    whole = Builder(cv, EXAMPLE.parent, Options(template="clear")).write_pdf(tmp_path / "all.pdf")
    first = Builder(cv, EXAMPLE.parent, Options(template="clear")).write_pdf(tmp_path / "one.pdf", first_pages=1)
    assert whole.pages > 1 and first.pages == 1
    assert pdf_text(tmp_path / "one.pdf")[0] == pdf_text(tmp_path / "all.pdf")[0]


def test_files_outside_the_cv_folder_are_not_read(tmp_path):
    """A photo path or URL from the CV cannot reach other files or the network."""
    import pytest
    from weasyprint.urls import URLFetchingError, fetch

    from cv_builder.model import CV
    from cv_builder.render import Builder, LocalFetcher

    own = tmp_path / "me"
    other = tmp_path / "other"
    own.mkdir()
    other.mkdir()
    (other / "photo.png").write_bytes(b"\x89PNG\r\n\x1a\n")
    for image in ("../other/photo.png", str(other / "photo.png"), "http://169.254.169.254/latest", "file:///etc/passwd"):
        cv = CV.model_validate({"basics": {"name": "Ada Example", "image": image}, "layout": {"header": {"photo": True}}})
        builder = Builder(cv, own)
        assert builder._photo_url() is None
        assert any("photo" in note for note in builder.notes)

    fetcher = LocalFetcher(own)
    with pytest.raises(URLFetchingError):
        with fetch(fetcher, (other / "photo.png").as_uri()):
            pass
    with pytest.raises(URLFetchingError):
        with fetch(fetcher, "http://127.0.0.1:9/"):
            pass
