#!/usr/bin/env python3
"""Draw a cover letter with cv-builder and with cv-forge and compare them (ADR-025).

    python3 -I tools/compare_letter.py <template|all> [letter.json] [cv.json] [--image] [--quiet]

Prints page counts, whether the words match in reading order, and how far each word moved:
the largest shift of the words' boxes (pt) over the whole page and over the letter below the
header, with the first words that moved more than half a point. With --image writes
docs/letter-<template>-side-by-side.png. cv-builder is career-ledger's (CVBUILDER_SRC), which
draws letters; cv-forge loads the template from templates/<name>, so a Typst change needs no
rebuild. Defaults: examples/letter-de.json and the example CV.
"""
import difflib
import html
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
LEDGER = Path.home() / "Documents/projects/personal/career/career-ledger"
VENV = Path(os.environ.get("CVBUILDER_PYTHON", LEDGER / ".venv/bin/python"))
SRC = os.environ.get("CVBUILDER_SRC", str(LEDGER / "cv-builder/src"))
FORGE = os.environ.get("CVFORGE", str(HERE / "target/quick/cv-forge"))
TEMPLATES = sorted(p.name for p in (HERE / "templates").iterdir() if (p / "cv.typ").is_file())

flags = {a for a in sys.argv[1:] if a.startswith("--")}
args = [a for a in sys.argv[1:] if not a.startswith("--")]
which = TEMPLATES if args[0] == "all" else [args[0]]
letter = args[1] if len(args) > 1 else str(HERE / "examples/letter-de.json")
cv = args[2] if len(args) > 2 else str(HERE / "examples/android-hmi-architect.json")
out = Path(tempfile.mkdtemp())


def run(cmd, env=None):
    p = subprocess.run(cmd, capture_output=True, text=True, env=env)
    return p.stdout.strip() + (" | " + p.stderr.strip().replace("\n", " | ") if p.stderr.strip() else "")


def words(pdf):
    """Per page: (word, xMin, yMin, xMax, yMax) in pdftotext's reading order."""
    xml = subprocess.run(["pdftotext", "-bbox", "-enc", "UTF-8", str(pdf), "-"], capture_output=True, text=True).stdout
    pages = []
    for page in xml.split("<page ")[1:]:
        found = re.findall(r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">(.*?)</word>', page)
        pages.append([(html.unescape(w), float(a), float(b), float(c), float(d)) for a, b, c, d, w in found])
    return pages


def compare(template):
    env = dict(os.environ, PYTHONPATH=SRC)
    py, rs = out / f"{template}-py.pdf", out / f"{template}-rs.pdf"
    said_py = run([str(VENV), "-m", "cv_builder", "letter", cv, letter, "-t", template, "-o", str(py)], env)
    said_rs = run([FORGE, "letter", cv, letter, "-t", str(HERE / "templates" / template), "-o", str(rs)])
    a, b = words(py), words(rs)
    same_words = [[w[0] for w in p] for p in a] == [[w[0] for w in p] for p in b]
    worst = worst_letter = 0.0
    moved = []
    for pa, pb in zip(a, b):
        if [w[0] for w in pa] != [w[0] for w in pb]:
            continue
        start = next((i for i, w in enumerate(pa) if w[0] == FIRST), len(pa))
        for i, (x, y) in enumerate(zip(pa, pb)):
            d = max(abs(x[1] - y[1]), abs(x[2] - y[2]), abs(x[3] - y[3]), abs(x[4] - y[4]))
            worst = max(worst, d)
            if i >= start:
                worst_letter = max(worst_letter, d)
            if d > 0.5 and len(moved) < 6:
                moved.append(f"{x[0]!r} {x[1]:.1f},{x[2]:.1f} vs {y[1]:.1f},{y[2]:.1f}")
    ok = len(a) == len(b) and same_words and worst_letter <= 0.5
    if "--quiet" not in flags or not ok:
        print(f"{template:10} pages {len(a)} vs {len(b)}; words {'same' if same_words else 'DIFFER'}; "
              f"largest shift {worst:.2f}pt, in the letter {worst_letter:.2f}pt")
        if "--quiet" not in flags:
            print("  cv-builder:", said_py)
            print("  cv-forge:  ", said_rs)
        for m in moved:
            print("   ", m)
        if not same_words:
            for n, (pa, pb) in enumerate(zip(a, b), 1):
                x, y = [w[0] for w in pa], [w[0] for w in pb]
                for op in difflib.SequenceMatcher(a=x, b=y, autojunk=False).get_opcodes():
                    if op[0] != "equal":
                        print(f"    page {n}: {op[0]} {x[op[1]:op[2]][:8]} -> {y[op[3]:op[4]][:8]}")
                        break
    if "--image" in flags:
        for name, pdf in (("py", py), ("rs", rs)):
            subprocess.run(["pdftoppm", "-r", "70", "-png", "-f", "1", "-l", "1", "-singlefile", str(pdf), str(out / f"{template}-{name}")], check=True)
        target = HERE / "docs" / f"letter-{template}-side-by-side.png"
        subprocess.run(["montage", str(out / f"{template}-py.png"), str(out / f"{template}-rs.png"),
                        "-tile", "2x1", "-geometry", "+8+8", "-background", "#888", str(target)], check=True)
        print("  wrote", target.relative_to(HERE))
    return ok


# The letter starts at its first word: the recipient's, else the date's, else the subject's.
data = json.loads(Path(letter).read_text(encoding="utf-8"))
r = data.get("recipient", {})
first_text = " ".join([r.get("company", ""), *r.get("lines", []), data.get("dateLine", ""), data.get("subject", "")]).split()
FIRST = first_text[0] if first_text else ""
results = [compare(t) for t in which]
print(f"{sum(results)}/{len(results)} letters match (same pages and words, no word in the letter moved more than 0.5pt)")
sys.exit(0 if all(results) else 1)
