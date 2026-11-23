"""cv-forge: cv-builder's PDF engine in Rust (Typst in place of WeasyPrint).

    build(cv_json, base_dir, *, template=None, accent=None, page_size=None, max_pages=None,
          pdf=None, first_pages=None, thumbnail_dpi=None) -> dict
        pages, notes, fit, last_page_fill, overflows, timings; "pdf" (bytes) when no path is
        given, "thumbnail" (PNG bytes of page 1) with thumbnail_dpi.
    fewest_pages(cv_json, base_dir, *, template=None) -> int
    fit(cv_json, base_dir, *, template=None, max_pages) -> dict | None
    problems(cv_json) -> list[str]     validation problems, as cv-builder words them
    LANGUAGES: list[str]               layout.language codes printed as cv-builder prints them
                                       ("en", "fr", "de" in full; the rest: CLDR month names)
    TEMPLATES: list[str]               the bundled templates' names

An invalid CV raises ValueError (cv-builder: pydantic's ValidationError, also a ValueError); a
CV that cannot be built raises RenderError. The GIL is released while a CV is typeset.
"""

from ._cvforge import ENGINE, LANGUAGES, TEMPLATES, RenderError, __version__, build, fewest_pages, fit, problems

__all__ = ["ENGINE", "LANGUAGES", "TEMPLATES", "RenderError", "__version__", "build", "fewest_pages", "fit", "problems"]
