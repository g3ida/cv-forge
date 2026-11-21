import json
from pathlib import Path

import pytest

from cv_builder.cli import load_cv
from cv_builder.dates import format_date, format_range
from cv_builder.model import CV, json_schema
from cv_builder.view import language_rating

ROOT = Path(__file__).resolve().parent.parent
EXAMPLE = ROOT / "examples" / "android-hmi-architect.json"
STARTER = ROOT / "src" / "cv_builder" / "assets" / "starter.json"


def cv(**extra) -> dict:
    return {"basics": {"name": "Test Person"}, **extra}


# ------------------------------------------------------------------ dates


@pytest.mark.parametrize(
    ("value", "expected"),
    [("2019", "2019"), ("2019-07", "Jul 2019"), ("2019-12-31", "Dec 2019"), (None, "")],
)
def test_format_date(value, expected):
    assert format_date(value) == expected


@pytest.mark.parametrize(
    ("start", "end", "expected"),
    [
        ("2026-07", None, "Jul 2026 - Present"),
        ("2022", "present", "2022 - Present"),
        ("2020-07", "2022-07", "Jul 2020 - Jul 2022"),
        ("2026", "2026", "2026"),
        (None, "2019", "2019"),
        (None, None, ""),
    ],
)
def test_format_range(start, end, expected):
    assert format_range(start, end) == expected


def test_range_without_ongoing_semantics():
    assert format_range("2021", None, ongoing_if_no_end=False) == "2021"


# ------------------------------------------------------------------ validation


def test_example_and_starter_are_valid():
    load_cv(EXAMPLE)
    load_cv(STARTER)


@pytest.mark.parametrize("bad", ["2019-13", "07/2019", "19", "2019-02-30x"])
def test_rejects_bad_dates(bad):
    with pytest.raises(ValueError, match=r"work\[0\]\.startDate"):
        _validate(cv(work=[{"name": "A", "startDate": bad}]))


def test_present_is_case_insensitive():
    model = CV.model_validate(cv(work=[{"name": "A", "startDate": "2020", "endDate": "Present"}]))
    assert model.work[0].endDate == "present"


def test_rejects_end_before_start():
    with pytest.raises(ValueError, match="before startDate"):
        _validate(cv(work=[{"name": "A", "startDate": "2020-05", "endDate": "2019"}]))


def test_year_precision_end_covers_whole_year():
    CV.model_validate(cv(work=[{"name": "A", "startDate": "2019-07", "endDate": "2019"}]))


def test_rejects_unknown_keys_to_catch_typos():
    with pytest.raises(ValueError, match=r"work\[0\]\.hightlights"):
        _validate(cv(work=[{"name": "A", "hightlights": ["x"]}]))


def test_empty_strings_count_as_missing():
    model = CV.model_validate(cv(work=[{"name": "A", "url": "", "startDate": "", "endDate": "  "}]))
    assert model.work[0].url is None and model.work[0].startDate is None


def test_layout_rejects_duplicate_sections():
    with pytest.raises(ValueError, match="more than once"):
        _validate(cv(layout={"columns": {"left": ["work"], "right": ["work"]}}))


def test_language_rating_inference():
    assert language_rating("Native", None) == 5
    assert language_rating("Fluent", None) == 4
    assert language_rating("B1", None) == 3
    assert language_rating("Elementary (A2)", None) == 2
    assert language_rating("Klingon", None) == 0
    assert language_rating("B1", 5) == 5


# ------------------------------------------------------------------ schema


def test_committed_schema_is_up_to_date():
    committed = json.loads((ROOT / "schema" / "cv.schema.json").read_text(encoding="utf-8"))
    assert committed == json_schema(), "run: cv-builder schema -o schema/cv.schema.json"


def _validate(data: dict) -> None:
    """Validate through the CLI loader so error locations are formatted."""
    import tempfile

    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as fh:
        json.dump(data, fh)
    try:
        load_cv(Path(fh.name))
    finally:
        Path(fh.name).unlink()
