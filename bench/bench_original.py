"""The same scenarios on the original (cv-builder + WeasyPrint), in-process, imports excluded.

python -I bench/bench_original.py <cv.json> <template> [max_pages] [iterations]
"""
import sys, tempfile, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "original" / "src"))
from cv_builder.cli import load_cv  # noqa: E402
from cv_builder.render import Builder, Options  # noqa: E402

path = Path(sys.argv[1]); template = sys.argv[2]
max_pages = int(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3] != "0" else None
n = int(sys.argv[4]) if len(sys.argv) > 4 else 5
cv = load_cv(path); out = Path(tempfile.mkdtemp()) / "x.pdf"
opts = Options(template=template, max_pages=max_pages)
t = time.perf_counter(); Builder(cv, path.resolve().parent, opts).write_pdf(out); first = time.perf_counter() - t
t = time.perf_counter()
for _ in range(n):
    Builder(cv, path.resolve().parent, opts).write_pdf(out)
same = (time.perf_counter() - t) / n
label = template + (f" max {max_pages}" if max_pages else "")
print(f"{label:<26} first {first*1e3:7.1f} ms | same CV {same*1e3:6.1f} ms | {out.stat().st_size // 1024} KB")
