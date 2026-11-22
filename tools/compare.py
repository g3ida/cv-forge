#!/usr/bin/env python3
"""Build a CV with cv-builder and with cv-forge and compare them (ADR-004).

    python3 -I tools/compare.py <template> [cv.json] [--max-pages N] [--image]

Prints page counts, words per page and the first differences in each page's extracted words
(in reading order for reading-order templates, sorted otherwise), and with --image writes
docs/<template>-side-by-side.png. cv-forge loads the template from templates/<name>, so a
Typst change needs no rebuild.
"""
import difflib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
VENV = Path.home() / "Documents/projects/personal/career/career-ledger/.venv/bin/python"
FORGE = os.environ.get("CVFORGE", str(HERE / "target/quick/cv-forge"))

argv = sys.argv[1:]
if "--max-pages" in argv:
    i = argv.index("--max-pages")
    argv = argv[:i] + argv[i + 2:]
args = [a for a in argv if not a.startswith("--")]
template = args[0]
cv = args[1] if len(args) > 1 else str(HERE / "examples/android-hmi-architect.json")
extra = []
if "--max-pages" in sys.argv:
    extra = ["--max-pages", sys.argv[sys.argv.index("--max-pages") + 1]]
reading = json.loads((HERE / "templates" / template / "template.json").read_text()).get("readingOrder", False)
out = Path(tempfile.mkdtemp())


def run(cmd, env=None):
    p = subprocess.run(cmd, capture_output=True, text=True, env=env)
    return p.stdout.strip() + (" | " + p.stderr.strip().replace("\n", " | ") if p.stderr.strip() else "")


# CVBUILDER_SRC: another cv-builder to compare with (career-ledger's own, for what it learned since).
env = dict(os.environ, PYTHONPATH=os.environ.get("CVBUILDER_SRC", str(HERE / "original/src")))
print("cv-builder:", run([str(VENV), "-m", "cv_builder", "build", cv, "-t", template, "-o", str(out / "py.pdf"), *extra], env))
print("cv-forge:  ", run([FORGE, "build", cv, "-t", str(HERE / "templates" / template), "-o", str(out / "rs.pdf"), *extra]))


def pages(pdf):
    text = subprocess.run(["pdftotext", "-enc", "UTF-8", str(pdf), "-"], capture_output=True, text=True).stdout
    return [p.split() for p in text.split("\f")[:-1]]


a, b = pages(out / "py.pdf"), pages(out / "rs.pdf")
print(f"pages: {len(a)} vs {len(b)}; words per page: {[len(p) for p in a]} vs {[len(p) for p in b]}")
same = True
for n, (x, y) in enumerate(zip(a, b), 1):
    if not reading:
        x, y = sorted(x), sorted(y)
    if x != y:
        same = False
        diff = [d for d in difflib.unified_diff(x, y, lineterm="", n=0) if d[:1] in "+-" and d[:3] not in ("+++", "---")]
        print(f"page {n}: {len(diff)} words differ: {' '.join(diff[:24])}")
print("TEXT MATCHES" if same and len(a) == len(b) else "TEXT DIFFERS")

if "--image" in sys.argv:
    for name in ("py", "rs"):
        subprocess.run(["pdftoppm", "-r", "70", "-png", str(out / f"{name}.pdf"), str(out / name)], check=True)
    script = f"""
from PIL import Image, ImageDraw
import glob
py = sorted(glob.glob({str(out)!r} + "/py-*.png")); rs = sorted(glob.glob({str(out)!r} + "/rs-*.png"))
w, h = Image.open(py[0]).size
gap, head = 16, 34
n = max(len(py), len(rs))
img = Image.new("RGB", (2 * w + 3 * gap, n * (h + gap) + gap + head), "#d0d4da")
d = ImageDraw.Draw(img)
d.text((gap, 10), "cv-builder (WeasyPrint)", fill="black"); d.text((2 * gap + w, 10), "cv-forge (Typst)", fill="black")
for i in range(n):
    y = head + gap + i * (h + gap)
    if i < len(py): img.paste(Image.open(py[i]), (gap, y))
    if i < len(rs): img.paste(Image.open(rs[i]), (2 * gap + w, y))
img.save({str(HERE / "docs" / f"{template}-side-by-side.png")!r}, optimize=True)
"""
    subprocess.run([str(VENV), "-I", "-c", script], check=True)
    print("image:", HERE / "docs" / f"{template}-side-by-side.png")
