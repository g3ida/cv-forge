"""Turn the validated CV model into display-ready sections for templates."""

from __future__ import annotations

import re
from dataclasses import dataclass, field, replace
from typing import Any

from markupsafe import Markup, escape

from .dates import format_date, format_range
from .icons import network_icon
from .model import CV, Profile

DEFAULT_TITLES: dict[str, str] = {
    "summary": "Summary",
    "work": "Experience",
    "volunteer": "Volunteering",
    "education": "Education",
    "projects": "Projects",
    "skills": "Skills",
    "languages": "Languages",
    "achievements": "Key Achievements",
    "awards": "Awards",
    "certificates": "Certifications",
    "publications": "Publications",
    "interests": "Interests",
    "references": "References",
    "profiles": "Find Me Online",
}

# ------------------------------------------------------------------ text helpers

_BOLD = re.compile(r"\*\*(.+?)\*\*")
_LINK = re.compile(r"\[([^\]]+)\]\(([^)\s]+)\)")


_TAG_OR_PLUSES = re.compile(r"(<[^>]+>)|\+\+")
NOBR = '<span class="nobr">++</span>'


def _keep_pluses(html: str) -> str:
    """Keep "++" in one piece ("C++"): Unicode allows a line break between the two plus signs,
    and a reader or resume parser then gets "C+ +". Text only, never inside a tag."""
    return _TAG_OR_PLUSES.sub(lambda m: m.group(1) or NOBR, html)


def keep(text: str | None) -> Markup | None:
    """Plain text for a template, escaped, with "++" kept together."""
    if text is None:
        return None
    return Markup(_keep_pluses(str(escape(text))))


def inline(text: str | None) -> Markup:
    """Escape text and apply a tiny markdown subset: **bold** and [label](url)."""
    if not text:
        return Markup("")
    html = str(escape(" ".join(text.split())))
    html = _BOLD.sub(r"<strong>\1</strong>", html)
    html = _LINK.sub(r'<a href="\2">\1</a>', html)
    return Markup(_keep_pluses(html))


def paragraphs(text: str | None) -> list[Markup]:
    if not text:
        return []
    return [inline(p) for p in re.split(r"\n\s*\n", text) if p.strip()]


def pretty_url(url: str) -> str:
    return re.sub(r"^(https?://)?(www\.)?", "", url).rstrip("/")


def href(url: str | None) -> str | None:
    if not url:
        return None
    return url if re.match(r"^[a-z][a-z0-9+.-]*:", url, re.I) else f"https://{url}"


# ------------------------------------------------------------------ view models


@dataclass
class MetaItem:
    icon: str
    text: str
    href: str | None = None


@dataclass
class Entry:
    title: str | None = None
    title_url: str | None = None
    subtitle: str | None = None
    subtitle_url: str | None = None
    meta: list[list[MetaItem]] = field(default_factory=list)
    paragraphs: list[Markup] = field(default_factory=list)
    bullets: list[Markup] = field(default_factory=list)
    tags: str | None = None
    continued: bool = False  # a later piece of an entry cut up by split_entries()
    same_employer: bool = False  # a work entry at the same company as the entry before it


@dataclass
class Section:
    id: str
    title: str
    kind: str  # name of the template macro rendering each item
    items: list[Any]

    def source_index(self, index: int) -> int:
        """Index in the CV's own list of the entry that ``items[index]`` belongs to."""
        return sum(1 for item in self.items[: index + 1] if not getattr(item, "continued", False)) - 1


@dataclass
class Header:
    name: str
    label: str | None
    contacts: list[MetaItem]
    photo: str | None


# ------------------------------------------------------------------ builders


def _meta_line(*items: MetaItem | None) -> list[MetaItem]:
    return [i for i in items if i and i.text]


def _date(text: str) -> MetaItem | None:
    return MetaItem("calendar", text) if text else None


def _place(text: str | None) -> MetaItem | None:
    return MetaItem("pin", text) if text else None


def _same(a: str | None, b: str | None) -> bool:
    return bool(a and b) and " ".join(a.split()).casefold() == " ".join(b.split()).casefold()


def _titles_line(w) -> list[Markup]:
    """``Engineer → Senior Engineer → Principal Engineer`` when a role held two titles or more.

    No dates on this line: a resume parser takes a line with dates for the start of another role,
    and the role's own dates already span the whole progression.
    """
    titles: list[str] = []
    for span in w.titleHistory:
        if not titles or not _same(titles[-1], span.position):
            titles.append(span.position)
    return [keep(" → ".join(titles))] if len(titles) > 1 else []


def _work(w, previous=None) -> Entry:
    """One role. Several roles at one company stay separate entries, each with the company's
    name, its dates and its own bullets, because resume parsers attach a role to the company
    line above it and lose roles stacked under a single heading. The company description is
    printed once, and ``project`` tells the entries apart."""
    same_employer = previous is not None and _same(previous.name, w.name)
    description = [] if same_employer and _same(previous.description, w.description) else paragraphs(w.description)
    project = [Markup(f'<span class="entry-project">{keep(w.project)}</span>')] if w.project else []
    return Entry(
        title=keep(w.position),
        subtitle=keep(w.name),
        subtitle_url=href(w.url),
        meta=[_meta_line(_date(format_range(w.startDate, w.endDate)), _place(w.location))],
        paragraphs=description + project + _titles_line(w) + paragraphs(w.summary),
        bullets=[inline(h) for h in w.highlights],
        same_employer=same_employer,
    )


def _volunteer(v) -> Entry:
    return Entry(
        title=keep(v.position),
        subtitle=keep(v.organization),
        subtitle_url=href(v.url),
        meta=[_meta_line(_date(format_range(v.startDate, v.endDate)), _place(v.location))],
        paragraphs=paragraphs(v.summary),
        bullets=[inline(h) for h in v.highlights],
    )


def _education(e) -> Entry:
    if e.studyType and e.area:
        title = f"{e.studyType} in {e.area}"
    else:
        title = e.studyType or e.area
    score = MetaItem("award", e.score) if e.score else None
    return Entry(
        title=keep(title),
        subtitle=keep(e.institution),
        subtitle_url=href(e.url),
        meta=[_meta_line(_date(format_range(e.startDate, e.endDate)), _place(e.location), score)],
        bullets=[inline(h) for h in e.highlights],
        tags=keep(", ".join(e.courses) or None),
    )


def _project(p) -> Entry:
    roles = ", ".join(p.roles)
    subtitle = " · ".join(x for x in (roles, p.entity) if x) or None
    meta = [_meta_line(_date(format_range(p.startDate, p.endDate)))]
    if p.url:
        meta.append([MetaItem("link", p.url, href(p.url))])
    return Entry(
        title=keep(p.name),
        subtitle=keep(subtitle),
        meta=[line for line in meta if line],
        paragraphs=paragraphs(p.description),
        bullets=[inline(h) for h in p.highlights],
        tags=keep(", ".join(p.keywords) or None),
    )


def _year(date: str | None) -> str | None:
    return date[:4] if date else None


def _line(title: str | None, url: str | None, *details: str | None, summary: str | None = None) -> dict[str, Any]:
    """One line: the name in bold (linked when there is a URL), then ``, detail, detail``.

    Certificates and awards are listed like this: the four lines of an entry (name, issuer,
    date, link) take a page's worth of room on a long record. A year alone, not a range, so a
    resume parser does not take the line for a role."""
    return {
        "title": keep(title),
        "url": href(url),
        "detail": keep(", ".join(d for d in details if d) or None),
        "summary": inline(summary) if summary else None,
    }


def _award(a) -> dict[str, Any]:
    return _line(a.title, None, a.awarder, _year(a.date), summary=a.summary)


def _certificate(c) -> dict[str, Any]:
    return _line(c.name, c.url, c.issuer, _year(c.date))


def _publication(p) -> Entry:
    meta = [_meta_line(_date(format_date(p.releaseDate)))]
    if p.url:
        meta.append([MetaItem("link", pretty_url(p.url), href(p.url))])
    return Entry(
        title=keep(p.name),
        title_url=href(p.url),
        subtitle=keep(p.publisher),
        meta=[m for m in meta if m],
        paragraphs=paragraphs(p.summary),
    )


_CEFR = {"a1": 1, "a2": 2, "b1": 3, "b2": 4, "c1": 4, "c2": 5}
_FLUENCY_WORDS = (
    ("native", 5), ("mother", 5), ("bilingual", 5), ("full professional", 5),
    ("fluent", 4), ("advanced", 4), ("professional", 4), ("proficient", 4),
    ("upper", 3), ("intermediate", 3), ("conversational", 3),
    ("elementary", 2), ("limited", 2), ("basic", 1), ("beginner", 1),
)


def language_rating(fluency: str | None, rating: int | None) -> int:
    if rating is not None:
        return rating
    text = (fluency or "").lower()
    for code, value in _CEFR.items():
        if re.search(rf"\b{code}\b", text):
            return value
    for word, value in _FLUENCY_WORDS:
        if word in text:
            return value
    return 0


def _profile_item(p: Profile) -> dict[str, Any]:
    text = pretty_url(p.url) if p.url else (p.username or p.network)
    return {"network": p.network, "text": text, "href": href(p.url), "icon": network_icon(p.network)}


def build_header(cv: CV, header_layout, photo_url: str | None) -> Header:
    b = cv.basics
    wanted = {n.lower() for n in header_layout.profiles}
    contacts: list[MetaItem] = []
    for key in header_layout.contacts:
        if key == "phone" and b.phone:
            contacts.append(MetaItem("phone", b.phone, "tel:" + re.sub(r"[^\d+]", "", b.phone)))
        elif key == "email" and b.email:
            contacts.append(MetaItem("at", b.email, f"mailto:{b.email}"))
        elif key == "url" and b.url:
            contacts.append(MetaItem("link", pretty_url(b.url), href(b.url)))
        elif key == "location" and b.location:
            loc = b.location
            text = ", ".join(x for x in (loc.city, loc.region or loc.countryCode) if x)
            if text:
                contacts.append(MetaItem("pin", text))
    for p in b.profiles:
        if p.network.lower() in wanted:
            item = _profile_item(p)
            contacts.append(MetaItem("link", item["text"], item["href"]))
    return Header(name=b.name, label=keep(b.label), contacts=contacts, photo=photo_url)


def build_sections(cv: CV, titles: dict[str, str], header_profiles: list[str]) -> dict[str, Section]:
    """All non-empty sections keyed by section id."""
    t = {**DEFAULT_TITLES, **titles}
    in_header = {n.lower() for n in header_profiles}
    raw: dict[str, tuple[str, list[Any]]] = {
        "summary": ("text", [paragraphs(cv.basics.summary)] if cv.basics.summary else []),
        "work": ("entry", [_work(w, cv.work[n - 1] if n else None) for n, w in enumerate(cv.work)]),
        "volunteer": ("entry", [_volunteer(v) for v in cv.volunteer]),
        "education": ("entry", [_education(e) for e in cv.education]),
        "projects": ("entry", [_project(p) for p in cv.projects]),
        "awards": ("line", [_award(a) for a in cv.awards]),
        "certificates": ("line", [_certificate(c) for c in cv.certificates]),
        "publications": ("entry", [_publication(p) for p in cv.publications]),
        "skills": (
            "tags",
            [{"name": keep(s.name), "level": s.level, "text": keep(", ".join(s.keywords))} for s in cv.skills],
        ),
        "interests": (
            "tags",
            [{"name": keep(i.name), "level": None, "text": keep(", ".join(i.keywords))} for i in cv.interests],
        ),
        "languages": (
            "language",
            [
                {"name": lang.language, "fluency": lang.fluency,
                 "rating": language_rating(lang.fluency, lang.rating)}
                for lang in cv.languages
            ],
        ),
        "achievements": (
            "achievement",
            [{"title": keep(a.title), "summary": inline(a.summary), "icon": a.icon or "gem"} for a in cv.achievements],
        ),
        "references": (
            "reference",
            [{"name": r.name, "text": paragraphs(r.reference)} for r in cv.references],
        ),
    }
    online = [_profile_item(p) for p in cv.basics.profiles if p.network.lower() not in in_header]
    raw["profiles"] = ("profiles", [online] if online else [])
    return {sid: Section(sid, t[sid], kind, items) for sid, (kind, items) in raw.items() if items}


def split_entries(section: Section) -> Section:
    """Cut the entries of a section into pieces that a page break may fall between.

    The first piece of an entry holds its heading, its paragraphs and its first
    bullet, so a heading never ends up alone at the bottom of a page. Each further
    bullet is a piece of its own; the tags line stays with the last bullet.
    Sections that are not made of entries are returned unchanged.
    """
    if section.kind != "entry":
        return section
    pieces: list[Entry] = []
    for entry in section.items:
        if len(entry.bullets) < 2:
            pieces.append(entry)
            continue
        pieces.append(replace(entry, bullets=entry.bullets[:1], tags=None))
        pieces += [Entry(bullets=[bullet], continued=True) for bullet in entry.bullets[1:]]
        pieces[-1].tags = entry.tags
    return replace(section, items=pieces)
