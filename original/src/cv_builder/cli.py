"""Command-line interface: ``cv-builder build|validate|schema|templates|init``."""

from __future__ import annotations

import argparse
import json
import sys
from importlib import resources
from pathlib import Path

from pydantic import ValidationError

from . import __version__
from .model import CV, json_schema


def _fail(message: str, code: int = 1) -> int:
    print(f"error: {message}", file=sys.stderr)
    return code


def _format_loc(loc: tuple) -> str:
    out = ""
    for part in loc:
        if isinstance(part, int):
            out += f"[{part}]"
        elif str(part).startswith(("function-", "function_")):  # validator frames add no info
            continue
        else:
            out += f".{part}" if out else str(part)
    return out or "(root)"


def load_cv(path: Path) -> CV:
    """Read and validate a CV file; raises ValueError with a readable report."""
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise ValueError(f"{path}: file not found") from None
    except json.JSONDecodeError as exc:
        raise ValueError(f"{path}:{exc.lineno}:{exc.colno}: invalid JSON: {exc.msg}") from None
    try:
        return CV.model_validate(data)
    except ValidationError as exc:
        lines = [f"{path}: {exc.error_count()} problem(s)"]
        for err in exc.errors():
            msg = err["msg"].removeprefix("Value error, ")
            lines.append(f"  {_format_loc(err['loc'])}: {msg}")
        raise ValueError("\n".join(lines)) from None


# ------------------------------------------------------------------ commands


def cmd_build(args: argparse.Namespace) -> int:
    from .render import Builder, Options, RenderError  # heavy import, only when needed

    try:
        cv = load_cv(args.cv)
    except ValueError as exc:
        return _fail(str(exc))
    output = args.output or args.cv.with_suffix(".pdf")
    try:
        options = Options(args.template, args.accent, args.page_size, args.max_pages)
        result = Builder(cv, args.cv.resolve().parent, options).write_pdf(output, html_target=args.html)
    except RenderError as exc:
        return _fail(str(exc))
    for note in result.notes:
        print(f"warning: {note}", file=sys.stderr)
    fit = result.fit
    if not args.quiet:
        tightened = (
            f", line spacing {fit.line_height:g} instead of {fit.designed_line_height:g} to fit"
            if fit and fit.tightness > 0 else ""
        )
        print(f"{output} ({result.pages} page{'s' if result.pages != 1 else ''}{tightened})")
    if fit and not fit.fits:
        return _fail(f"{result.pages} pages exceeds the limit of {fit.max_pages}", 3)
    return 0


def cmd_validate(args: argparse.Namespace) -> int:
    status = 0
    for path in args.cv:
        try:
            load_cv(path)
            print(f"{path}: ok")
        except ValueError as exc:
            print(exc, file=sys.stderr)
            status = 1
    return status


def cmd_schema(args: argparse.Namespace) -> int:
    text = json.dumps(json_schema(), indent=2, ensure_ascii=False) + "\n"
    if args.output:
        args.output.write_text(text, encoding="utf-8")
        print(args.output)
    else:
        sys.stdout.write(text)
    return 0


def cmd_templates(args: argparse.Namespace) -> int:
    from .render import available_templates

    for t in available_templates():
        print(f"{t.name:<10} {t.description}")
        order = "reads in order (safe for resume parsers)" if t.reading_order else "for people; parsers mix up its text"
        print(f"{'':<10} columns: {', '.join(t.column_names)}; {order}")
        if t.aliases:
            print(f"{'':<10} formerly: {', '.join(t.aliases)}")
    return 0


def cmd_init(args: argparse.Namespace) -> int:
    if args.path.exists() and not args.force:
        return _fail(f"{args.path} exists (use --force to overwrite)")
    starter = resources.files("cv_builder").joinpath("assets/starter.json").read_text(encoding="utf-8")
    args.path.parent.mkdir(parents=True, exist_ok=True)
    args.path.write_text(starter, encoding="utf-8")
    print(f"wrote {args.path}")
    schema_path = args.path.parent / "cv.schema.json"
    if not schema_path.exists():  # referenced by "$schema" for editor validation/completion
        schema_path.write_text(json.dumps(json_schema(), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"wrote {schema_path}")
    print(f"edit {args.path}, then run: cv-builder build {args.path}")
    return 0


# ------------------------------------------------------------------ parser


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="cv-builder", description="Build a PDF CV from a JSON file.")
    parser.add_argument("--version", action="version", version=f"%(prog)s {__version__}")
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("build", help="render a CV JSON file to PDF")
    p.add_argument("cv", type=Path, help="CV JSON file")
    p.add_argument("-o", "--output", type=Path, help="PDF path (default: next to the JSON)")
    p.add_argument("-t", "--template", help="template name or directory (overrides layout.template)")
    p.add_argument("--accent", help="accent color, e.g. '#0B7A75' (overrides layout.accentColor)")
    p.add_argument("--page-size", choices=["A4", "Letter"], help="overrides layout.pageSize")
    p.add_argument("--html", type=Path, help="also write the paginated HTML (for debugging)")
    p.add_argument(
        "--max-pages", type=int,
        help="page limit (overrides layout.maxPages): tighten line spacing until it fits, never below 1.15; "
        "text size and margins never change; exit with status 3 when it still does not fit",
    )
    p.add_argument("-q", "--quiet", action="store_true")
    p.set_defaults(func=cmd_build)

    p = sub.add_parser("validate", help="check CV JSON files against the schema")
    p.add_argument("cv", type=Path, nargs="+")
    p.set_defaults(func=cmd_validate)

    p = sub.add_parser("schema", help="print the JSON Schema of the CV format")
    p.add_argument("-o", "--output", type=Path)
    p.set_defaults(func=cmd_schema)

    p = sub.add_parser("templates", help="list available templates")
    p.set_defaults(func=cmd_templates)

    p = sub.add_parser("init", help="write a starter CV JSON file")
    p.add_argument("path", type=Path, nargs="?", default=Path("cv.json"))
    p.add_argument("--force", action="store_true")
    p.set_defaults(func=cmd_init)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
