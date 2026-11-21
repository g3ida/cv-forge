#!/usr/bin/env python3
"""Run a cv-builder compatible CLI and print what the equivalence bar compares (DECISIONS.md ADR-004).

The CLI's own stdout, stderr and exit code pass through unchanged. When a PDF was written, its
page count and its text are appended to stdout and the PDF is deleted: per page, the words as
pdftotext extracts them; in reading order for templates that promise it (template.json
"readingOrder"), sorted for the others, whose two columns any parser interleaves.

CVCHECK_BIN picks the build: unset runs the original (the Python snapshot in original/).
"""
import json
import os
import shlex
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
TEMPLATES = HERE / "templates"
ORIGINAL = [str(Path.home() / "Documents/projects/personal/career/career-ledger/.venv/bin/python"), "-m", "cv_builder"]

args = sys.argv[1:]
cmd = shlex.split(os.environ["CVCHECK_BIN"]) if os.environ.get("CVCHECK_BIN") else ORIGINAL
env = dict(os.environ, PYTHONPATH=str(HERE / "original" / "src"))
proc = subprocess.run(cmd + args, capture_output=True, text=True, env=env)
sys.stdout.write(proc.stdout)
sys.stderr.write(proc.stderr)
sys.stdout.flush()

if args[:1] == ["build"] and len(args) > 1:
    rest = args[1:]
    out = template = None
    cv = None
    i = 0
    while i < len(rest):
        a = rest[i]
        if a in ("-o", "--output"):
            out, i = rest[i + 1], i + 2
        elif a in ("-t", "--template"):
            template, i = rest[i + 1], i + 2
        elif a in ("--accent", "--page-size", "--max-pages", "--html"):
            i += 2
        elif a.startswith("-"):
            i += 1
        else:
            cv, i = a, i + 1
    pdf = Path(out) if out else (Path(cv).with_suffix(".pdf") if cv else None)
    if pdf and pdf.is_file():
        if template is None:
            try:
                template = json.loads(Path(cv).read_text(encoding="utf-8")).get("layout", {}).get("template")
            except Exception:
                template = None
        name = template or "compact"
        config = TEMPLATES / name / "template.json"
        if not config.is_file():
            config = next((c for c in TEMPLATES.glob("*/template.json") if name in json.loads(c.read_text()).get("aliases", [])), config)
        reading = config.is_file() and json.loads(config.read_text()).get("readingOrder", False)
        info = subprocess.run(["pdfinfo", str(pdf)], capture_output=True, text=True).stdout
        pages = next((line.split()[-1] for line in info.splitlines() if line.startswith("Pages:")), "?")
        text = subprocess.run(["pdftotext", "-enc", "UTF-8", str(pdf), "-"], capture_output=True, text=True).stdout
        print(f"[pdf] pages={pages} order={'reading' if reading else 'sorted'}")
        for n, page in enumerate(text.split("\f")[: int(pages) if pages.isdigit() else None], 1):
            words = page.split()
            print(f"[page {n}] " + " ".join(words if reading else sorted(words)))
        pdf.unlink()
sys.exit(proc.returncode)
