"""Turn the Google Fonts variable TTFs into the static weights bundled with cv-builder.

Usage: python scripts/build_fonts.py RUBIK_VF.ttf INTER_VF.ttf

Sources (SIL Open Font License 1.1):
  https://github.com/google/fonts/tree/main/ofl/rubik
  https://github.com/google/fonts/tree/main/ofl/inter
"""

import sys
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

OUT = Path(__file__).resolve().parent.parent / "src" / "cv_builder" / "assets" / "fonts"
WEIGHTS = {400: "Regular", 500: "Medium", 600: "SemiBold", 700: "Bold"}
FAMILIES = {
    "Rubik": {"weights": (400, 500, 700), "fixed": {}},
    "Inter": {"weights": (400, 500, 600, 700), "fixed": {"opsz": 14}},
}


def main() -> None:
    rubik, inter = map(Path, sys.argv[1:3])
    OUT.mkdir(parents=True, exist_ok=True)
    for family, src in (("Rubik", rubik), ("Inter", inter)):
        spec = FAMILIES[family]
        for weight in spec["weights"]:
            font = TTFont(src)
            axes = {"wght": weight, **spec["fixed"]}
            static = instantiateVariableFont(font, axes, updateFontNames=False)
            target = OUT / f"{family}-{WEIGHTS[weight]}.ttf"
            static.save(target)
            print(f"wrote {target.name} ({target.stat().st_size // 1024} KiB)")


if __name__ == "__main__":
    main()
