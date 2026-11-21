"""Split measured column content into pages.

Each template column flows independently from page to page (like the
reference layout). Items are never split; a section that continues on a new
page repeats its title (a template that does not repeat titles passes a
title height of zero).
"""

from __future__ import annotations

from dataclasses import dataclass

TOLERANCE = 0.5  # px of slack for float rounding (the default; a template may set its own)


@dataclass(frozen=True)
class Unit:
    """One item measured in a continuous single-page layout of its column."""

    section: str
    index: int
    top: float  # border-box top; for an item opening its section, the section top
    bottom: float  # border-box bottom of the item

    @property
    def opens_section(self) -> bool:
        return self.index == 0


@dataclass
class Overflow:
    column: str
    unit: Unit
    height: float
    available: float


def paginate_column(
    column: str,
    units: list[Unit],
    title_heights: dict[str, float],
    first_page: float,
    other_pages: float,
    tolerance: float = TOLERANCE,
) -> tuple[list[list[Unit]], list[Overflow]]:
    """Greedily fill pages; returns the units per page and items too tall for a page.

    ``tolerance`` is how far (px) the content of a page may exceed the page. A page
    that is overfilled, however little, makes WeasyPrint move what sticks out to an
    extra page; pass zero or less to rule that out.
    """
    pages: list[list[Unit]] = []
    overflows: list[Overflow] = []
    i = 0
    while i < len(units):
        available = first_page if not pages else other_pages
        start = units[i]
        repeat_title = 0.0 if start.opens_section else title_heights[start.section]
        j = i
        while j < len(units) and units[j].bottom - start.top + repeat_title <= available + tolerance:
            j += 1
        if j == i:
            height = start.bottom - start.top + repeat_title
            if not pages and height <= other_pages + tolerance:
                pages.append([])  # does not fit under the header: start on page 2
                continue
            overflows.append(Overflow(column, start, height, available))
            j = i + 1
        pages.append(units[i:j])
        i = j
    return pages, overflows
