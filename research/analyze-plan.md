# Rust migration plan: cv-builder (Python)

1 packages, 10 source files (1480 lines), 7 test files, 0 UI files, grouped into 2 units.

| Ready | Caution | Blocked | Keep in source language |
| ---: | ---: | ---: | ---: |
| 0 | 2 | 0 | 0 |

## Start here

Best first ports: safe to port (readiness) and worth porting (loops and string work, tests, size).

| # | Unit | Lines | Priority | Readiness | Tests | Bridge | Main risks |
| ---: | --- | ---: | ---: | ---: | ---: | --- | --- |
| 1 | `src/cv_builder/` | 1450 | 34 | 2 | 7 | PyO3 | Hash iteration order (11); String offsets and slicing (9); Regex features the `regex` crate lacks (1) |

### `src/cv_builder/`

1450 lines in 9 files, wave 0, priority 34, readiness 2, bridge **PyO3**.

- 2 PyPI dependencies not in the crate table: check them by hand.
- Leaf: imports no other unit.
- 7 test files import it directly.
- Hash iteration order x11: `src/cv_builder/model.py:342`, `src/cv_builder/model.py:384`, `src/cv_builder/render.py:47`
- String offsets and slicing x9: `src/cv_builder/model.py:94`, `src/cv_builder/paginate.py:69`, `src/cv_builder/render.py:329`
- Regex features the `regex` crate lacks x1: `src/cv_builder/render.py:445`
- Regexes built at runtime x1: `src/cv_builder/model.py:61`
- Class inheritance x24: `src/cv_builder/model.py:116`, `src/cv_builder/model.py:127`, `src/cv_builder/model.py:142`
- Dynamic typing x20: `src/cv_builder/cli.py:25`, `src/cv_builder/model.py:71`, `src/cv_builder/model.py:122`
- Number semantics x110: `src/cv_builder/dates.py:8`, `src/cv_builder/dates.py:19`, `src/cv_builder/dates.py:20`
- Exceptions x18: `src/cv_builder/cli.py:36`, `src/cv_builder/cli.py:39`, `src/cv_builder/cli.py:41`

## Order by package

Packages come after the workspace packages they use. Inside a package, port by wave: a unit's imports are in earlier waves.

### 1. cv-builder (`.`, portable, 7 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `scripts/` | caution | 30 | 93 | PyO3 | String offsets and slicing (1); Division and modulo (1) |
| 0 | `src/cv_builder/` | caution | 1450 | 2 | PyO3 | Hash iteration order (11); String offsets and slicing (9) |

## PyPI dependencies

| Package | Units | Rust | Fit | Note |
| --- | ---: | --- | --- | --- |
| `fonttools` | 1 | ? | unknown |  |
| `markupsafe` | 1 | ? | unknown |  |
| `weasyprint` | 1 | ? | unknown |  |
| `pydantic` | 1 | serde, garde | partial | Validation errors will differ. |
| `jinja2` | 1 | minijinja | good | Same template language, by Jinja's author. |

## Risk guide

- **String offsets and slicing** (High): JS indexes strings in UTF-16 code units, Rust in UTF-8 bytes. Offsets that cross the boundary must be converted, or non-ASCII text gets wrong ranges silently.
- **Regex features the `regex` crate lacks** (High): Lookaround, backreferences and the `y` flag are not in the `regex` crate. Use `fancy-regex` for those patterns, or rewrite them, and test them on non-ASCII text: `\w`, `\b` and case-insensitive matching differ between engines even where a pattern compiles. JS `/i` without `u` never matches a non-ASCII letter to an ASCII one, Rust `(?i)` does (`ſ` matches `s`, the Kelvin sign `K` matches `k`): write `(?i-u)` or compare with `eq_ignore_ascii_case`.
- **Regexes built at runtime** (High): Patterns come from data or config in JS syntax. Rust regex syntax differs, so user-visible behavior can change. Pin it with tests on real patterns.
- **Dynamic typing** (Medium): Values whose shape is checked at runtime need an explicit Rust enum or `serde_json::Value`.
- **Exceptions** (Low): Thrown errors become `Result`. Errors crossing the bridge must map back to the same JS error shape.
- **Class inheritance** (Medium): Class hierarchies become traits plus composition; `instanceof` checks become enums.
- **Number semantics** (Low): JS numbers are f64; bitwise ops truncate to i32; parseInt is lenient. Pick Rust integer types deliberately. Printing differs: `${n}` gives `1e+21`, `1e-7` and `0` for -0 where Rust's `{}` does not, so use `js::number_to_string`. `Math.round` rounds halves up (-2.5 gives -2), Rust's `round` away from zero: use `js::math_round`. `toFixed` rounds exact halves up ((2.5).toFixed(0) is "3"), Rust's `{:.0}` to even.
- **Division and modulo** (Medium): Python `//` and `%` floor toward negative infinity; Rust `/` and `%` truncate: use `div_euclid`/`rem_euclid` or explicit floor math. C division by zero is undefined.
- **Hash iteration order** (High): Set and map iteration order comes from the source language's hash function. A port cannot reproduce it: sort before output, or make the order part of the contract.

## Whole app

Shape: **cli**.

| Entry point | Kind | Found from |
| --- | --- | --- |
| cv-builder: `cv_builder.cli:main` | cli | pyproject.toml scripts |
| `scripts/build_fonts.py` | cli | `if __name__ == "__main__"` at scripts/build_fonts.py:38 |
| `src/cv_builder/cli.py` | cli | `if __name__ == "__main__"` at src/cv_builder/cli.py:177 |

### Every dependency

| Dependency | Declared | Used by | Rust | Fit | Note |
| --- | --- | --- | --- | --- | --- |
| `fonttools` | no | 1 | ? | unknown |  |
| `markupsafe` | no | 1 | ? | unknown |  |
| `weasyprint` | yes | 1 | ? | unknown |  |
| `pydantic` | yes | 1 | serde, garde | partial | Validation errors will differ. |
| `jinja2` | yes | 1 | minijinja | good | Same template language, by Jinja's author. |

### Pieces, in port order

- wave 0: `scripts/` (30 lines)
- wave 0: `src/cv_builder/` (1450 lines)

### Decisions for the user

**D1** (shape): The app is a cli, entered through cv-builder (cv_builder.cli:main), scripts/build_fonts.py, src/cv_builder/cli.py. Port every entry point, in that shape?

> Yes: a clap front door (`app front-door`) takes every command over first

**D2** (dependency fonttools): Which crate replaces fonttools (used by 1 unit)?

> Research it against primary sources into rust/research/fonttools.md before porting what uses it

**D3** (dependency markupsafe): Which crate replaces markupsafe (used by 1 unit)?

> Research it against primary sources into rust/research/markupsafe.md before porting what uses it

**D4** (dependency weasyprint): Which crate replaces weasyprint (used by 1 unit)?

> Research it against primary sources into rust/research/weasyprint.md before porting what uses it

**D5** (dependency pydantic): pydantic maps to serde, garde only in part (used by 1 unit; Validation errors will differ). Use it?

> Use serde, garde, and record cases for the behavior it does not cover

