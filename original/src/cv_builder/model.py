"""The standardized CV document format.

Field names follow JSON Resume v1.0.0 (https://jsonresume.org/schema), so an
existing JSON Resume file is valid input. On top of it, cv-builder adds a few
documented extensions:

* ``achievements``                 highlighted accomplishments (title, summary, icon)
* ``education[].location``         where the school is
* ``education[].highlights``       bullet points for a degree
* ``languages[].rating``           0-5 proficiency used for the dot indicator
* ``layout``                       presentation: template, columns, titles, colors

Conventions:

* Dates are ISO 8601 with year, month or day precision: ``2019``, ``2019-07``,
  ``2019-07-15``. Precision is kept when printing (``2019`` vs ``Jul 2019``).
* An omitted ``endDate`` means "ongoing" and prints as "Present" (JSON Resume
  semantics). ``"present"`` may also be written explicitly. For a single point
  in time (e.g. a project done in 2021) set ``endDate`` equal to ``startDate``.
* Unknown keys are rejected so typos surface early. Empty strings are treated as
  missing, which keeps exports from other tools loadable.
"""

from __future__ import annotations

import re
from typing import Annotated, Any, Literal

from pydantic import (
    AfterValidator,
    BaseModel,
    BeforeValidator,
    ConfigDict,
    Field,
    model_validator,
)

SCHEMA_VERSION = "1.0.0"

SectionId = Literal[
    "summary",
    "work",
    "volunteer",
    "education",
    "projects",
    "skills",
    "languages",
    "achievements",
    "awards",
    "certificates",
    "publications",
    "interests",
    "references",
    "profiles",
]
SECTION_IDS: tuple[str, ...] = SectionId.__args__  # type: ignore[attr-defined]

# --------------------------------------------------------------------------- dates

_DATE_PATTERN = r"^\d{4}(-(0[1-9]|1[0-2])(-(0[1-9]|[12]\d|3[01]))?)?$"
_DATE_RE = re.compile(_DATE_PATTERN)


def _check_date(value: str) -> str:
    if not _DATE_RE.match(value):
        raise ValueError(f"invalid date {value!r}: use YYYY, YYYY-MM or YYYY-MM-DD")
    return value


def _normalize_end(value: Any) -> Any:
    if isinstance(value, str) and value.strip().lower() == "present":
        return "present"
    return value


def _check_end(value: str) -> str:
    return value if value == "present" else _check_date(value)


PartialDate = Annotated[
    str,
    AfterValidator(_check_date),
    Field(
        json_schema_extra={"pattern": _DATE_PATTERN},
        description="ISO 8601 date with year, month or day precision.",
        examples=["2019", "2019-07", "2019-07-15"],
    ),
]
EndDate = Annotated[
    str,
    BeforeValidator(_normalize_end),
    AfterValidator(_check_end),
    Field(
        json_schema_extra={"pattern": r"^([Pp]resent|" + _DATE_PATTERN[1:-1] + r")$"},
        description='End date, or "present". Omit for ongoing entries (prints "Present"); '
        "set equal to startDate to print a single date.",
        examples=["2022-06", "present"],
    ),
]

_HEX_COLOR = r"^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$"


def _date_bounds(value: str) -> tuple[tuple[int, int, int], tuple[int, int, int]]:
    """Earliest and latest day covered by a partial date."""
    parts = [int(p) for p in value.split("-")]
    year = parts[0]
    lo = (year, parts[1] if len(parts) > 1 else 1, parts[2] if len(parts) > 2 else 1)
    hi = (year, parts[1] if len(parts) > 1 else 12, parts[2] if len(parts) > 2 else 31)
    return lo, hi


# --------------------------------------------------------------------------- base


class _Model(BaseModel):
    model_config = ConfigDict(extra="forbid", str_strip_whitespace=True, populate_by_name=True)

    @model_validator(mode="before")
    @classmethod
    def _drop_empty_strings(cls, data: Any) -> Any:
        if isinstance(data, dict):
            return {k: v for k, v in data.items() if not (isinstance(v, str) and not v.strip())}
        return data


class _Dated(_Model):
    startDate: PartialDate | None = None
    endDate: EndDate | None = None

    @model_validator(mode="after")
    def _end_after_start(self) -> _Dated:
        if self.startDate and self.endDate and self.endDate != "present":
            if _date_bounds(self.endDate)[1] < _date_bounds(self.startDate)[0]:
                raise ValueError(f"endDate {self.endDate} is before startDate {self.startDate}")
        return self


# --------------------------------------------------------------------------- basics


class Location(_Model):
    address: str | None = None
    postalCode: str | None = None
    city: str | None = None
    region: str | None = Field(None, description="State, province or country name.")
    countryCode: str | None = Field(None, description="ISO 3166-1 alpha-2 code, e.g. DE.")


class Profile(_Model):
    network: str = Field(description="e.g. LinkedIn, GitHub, Hugging Face.")
    username: str | None = None
    url: str | None = None


class Basics(_Model):
    name: str
    label: str | None = Field(None, description="Headline shown under the name.")
    image: str | None = Field(None, description="Photo: path relative to the JSON file, or a URL.")
    email: str | None = None
    phone: str | None = None
    url: str | None = Field(None, description="Personal website.")
    summary: str | None = Field(None, description="Short bio; blank lines separate paragraphs.")
    location: Location | None = None
    profiles: list[Profile] = Field(default_factory=list)


# --------------------------------------------------------------------------- sections


class TitleSpan(_Dated):
    """Extension: one title held during a role."""

    position: str


class Work(_Dated):
    name: str | None = Field(None, description="Company name.")
    position: str | None = None
    location: str | None = None
    url: str | None = None
    description: str | None = Field(
        None,
        description="What the company does. Printed once when the entry before is at the same company with the same text.",
    )
    project: str | None = Field(
        None,
        description="Extension: the project or assignment, to tell apart several entries at one company.",
    )
    titleHistory: list[TitleSpan] = Field(
        default_factory=list,
        description="Extension: the titles held in this role, oldest first. Two or more are printed as one line.",
    )
    summary: str | None = Field(None, description="Overview of the role.")
    highlights: list[str] = Field(default_factory=list)


class Volunteer(_Dated):
    organization: str | None = None
    position: str | None = None
    location: str | None = None
    url: str | None = None
    summary: str | None = None
    highlights: list[str] = Field(default_factory=list)


class Education(_Dated):
    institution: str | None = None
    url: str | None = None
    area: str | None = Field(None, description="Field of study, e.g. Computer Science.")
    studyType: str | None = Field(None, description="Degree, e.g. Master, Bachelor.")
    score: str | None = None
    courses: list[str] = Field(default_factory=list)
    location: str | None = Field(None, description="Extension: where the school is.")
    highlights: list[str] = Field(default_factory=list, description="Extension: bullet points.")


class Award(_Model):
    title: str
    date: PartialDate | None = None
    awarder: str | None = None
    summary: str | None = None


class Certificate(_Model):
    name: str
    date: PartialDate | None = None
    issuer: str | None = None
    url: str | None = None


class Publication(_Model):
    name: str
    publisher: str | None = None
    releaseDate: PartialDate | None = None
    url: str | None = None
    summary: str | None = None


class Skill(_Model):
    name: str = Field(description="Skill group name, e.g. Systems Programming.")
    level: str | None = None
    keywords: list[str] = Field(default_factory=list)


class Language(_Model):
    language: str
    fluency: str | None = Field(None, description="e.g. Native, Fluent, B1.")
    rating: int | None = Field(
        None,
        ge=0,
        le=5,
        description="Extension: 0-5 dots. Inferred from fluency (CEFR or words) when omitted.",
    )


class Interest(_Model):
    name: str
    keywords: list[str] = Field(default_factory=list)


class Reference(_Model):
    name: str
    reference: str | None = None


class Project(_Dated):
    name: str
    description: str | None = None
    highlights: list[str] = Field(default_factory=list)
    keywords: list[str] = Field(default_factory=list)
    url: str | None = None
    roles: list[str] = Field(default_factory=list)
    entity: str | None = None
    type: str | None = None


class Achievement(_Model):
    """Extension: a highlighted accomplishment."""

    title: str
    summary: str | None = None
    icon: str | None = Field(None, description="Icon name, e.g. gem, star, award, zap, target.")


class Meta(BaseModel):
    model_config = ConfigDict(extra="allow")

    canonical: str | None = None
    version: str | None = None
    lastModified: str | None = None


# --------------------------------------------------------------------------- layout


class HeaderLayout(_Model):
    contacts: list[Literal["phone", "email", "url", "location"]] = Field(
        default_factory=lambda: ["phone", "email", "url"],
        description="Contact fields shown in the header, in order.",
    )
    profiles: list[str] = Field(
        default_factory=lambda: ["LinkedIn"],
        description="basics.profiles networks shown in the header (case-insensitive). "
        "The remaining profiles go to the 'profiles' section.",
    )
    photo: bool = Field(True, description="Show basics.image when set.")


class Layout(_Model):
    """Extension: how the CV is presented. Every field is optional."""

    template: str | None = Field(
        None,
        description="Template name (`cv-builder templates` lists them: clear, classic, compact, modern, sidebar, "
        "timeline) or a template directory. The former names two-column and single-column still work.",
    )
    pageSize: Literal["A4", "Letter"] | None = None
    maxPages: int | None = Field(
        None,
        ge=1,
        le=20,
        description="Page limit. A CV that runs over gets tighter line spacing until it fits, never below 1.15; "
        "text size and margins never change. If that is not enough, the build warns how many lines are too "
        "many.",
    )
    accentColor: str | None = Field(None, pattern=_HEX_COLOR, description="Hex color, e.g. #1E90FF.")
    columns: dict[str, list[SectionId]] | None = Field(
        None,
        description="Section order per template column, e.g. "
        '{"left": ["work", "education"], "right": ["summary", "skills"]} for compact or '
        '{"main": ["summary", "work", "education", "skills"]} for clear. '
        "Sections left out are not rendered.",
    )
    titles: dict[SectionId, str] = Field(default_factory=dict, description="Section title overrides.")
    header: HeaderLayout = Field(default_factory=HeaderLayout)
    documentTitle: str | None = Field(None, description="PDF document title metadata.")

    @model_validator(mode="after")
    def _no_duplicate_sections(self) -> Layout:
        if self.columns:
            seen: set[str] = set()
            for sections in self.columns.values():
                for section in sections:
                    if section in seen:
                        raise ValueError(f"section {section!r} is listed more than once in layout.columns")
                    seen.add(section)
        return self


# --------------------------------------------------------------------------- root


class CV(_Model):
    """A CV document."""

    schema_ref: str | None = Field(None, alias="$schema", description="Path or URL of this JSON Schema.")
    basics: Basics
    work: list[Work] = Field(default_factory=list)
    volunteer: list[Volunteer] = Field(default_factory=list)
    education: list[Education] = Field(default_factory=list)
    awards: list[Award] = Field(default_factory=list)
    certificates: list[Certificate] = Field(default_factory=list)
    publications: list[Publication] = Field(default_factory=list)
    skills: list[Skill] = Field(default_factory=list)
    languages: list[Language] = Field(default_factory=list)
    interests: list[Interest] = Field(default_factory=list)
    references: list[Reference] = Field(default_factory=list)
    projects: list[Project] = Field(default_factory=list)
    achievements: list[Achievement] = Field(default_factory=list)
    meta: Meta | None = None
    layout: Layout = Field(default_factory=Layout)


def json_schema() -> dict[str, Any]:
    schema = CV.model_json_schema(by_alias=True)
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": f"cv-builder CV (format {SCHEMA_VERSION})",
        "description": (
            "CV document: JSON Resume v1.0.0 compatible, with cv-builder extensions "
            "(achievements, education location/highlights, language rating, layout)."
        ),
        **{k: v for k, v in schema.items() if k not in {"title", "description"}},
    }
