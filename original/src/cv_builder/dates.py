"""Display formatting for the partial ISO dates used in the CV format."""

from __future__ import annotations

MONTHS = ("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")


def format_date(value: str | None) -> str:
    """``2019`` -> ``2019``; ``2019-07`` and ``2019-07-15`` -> ``Jul 2019``."""
    if not value:
        return ""
    parts = value.split("-")
    if len(parts) == 1:
        return parts[0]
    return f"{MONTHS[int(parts[1]) - 1]} {parts[0]}"


def format_range(
    start: str | None,
    end: str | None,
    *,
    ongoing_if_no_end: bool = True,
    present: str = "Present",
    separator: str = " - ",
) -> str:
    """Format a start/end pair.

    A missing end means "ongoing" (JSON Resume semantics) unless
    ``ongoing_if_no_end`` is false. Identical ends collapse to one date.
    """
    first = format_date(start)
    if end == "present" or (end is None and start and ongoing_if_no_end):
        last = present
    else:
        last = format_date(end)
    if not first:
        return last if last != present else ""
    if not last or last == first:
        return first
    return f"{first}{separator}{last}"
