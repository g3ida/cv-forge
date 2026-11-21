//! Python's text rules, where the original's output depends on them: `repr()`, `str.strip()`,
//! `int()`, `round()` and the `g` format. Messages and notes must read exactly as cv-builder's.

use std::fmt::Write;

/// Python's `str.isspace()`: Unicode White_Space plus the four ASCII separators U+001C..U+001F.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `str.strip()` with no argument.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// `" ".join(s.split())`: whitespace runs become one space, none at either end.
pub fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for word in s.split(is_space).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// Not printable for `str.isprintable()`: escaped by `repr()`. Control and format characters,
/// separators other than the space, private use. (Unassigned code points are not listed.)
fn escaped_by_repr(c: char) -> bool {
    let n = c as u32;
    c.is_control()
        || matches!(n, 0xa0 | 0xad | 0x1680 | 0x2000..=0x200f | 0x2028..=0x202f | 0x205f..=0x2064)
        || matches!(n, 0x2066..=0x206f | 0x3000 | 0xfeff | 0xfff9..=0xfffb | 0x180e | 0x061c | 0x06dd)
        || matches!(n, 0x0600..=0x0605 | 0x070f | 0xe000..=0xf8ff | 0xf0000..=0x10ffff)
}

/// `repr()` of a str.
pub fn repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if escaped_by_repr(c) => {
                let n = c as u32;
                // INFALLIBLE: writing to a String cannot fail.
                let _ = match n {
                    0..=0xff => write!(out, "\\x{n:02x}"),
                    0x100..=0xffff => write!(out, "\\u{n:04x}"),
                    _ => write!(out, "\\U{n:08x}"),
                };
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// `repr()` of a list of str: `['a', 'b']`.
pub fn repr_list<S: AsRef<str>>(items: &[S]) -> String {
    let parts: Vec<String> = items.iter().map(|s| repr(s.as_ref())).collect();
    format!("[{}]", parts.join(", "))
}

/// The value of a Unicode decimal digit (general category Nd), as `int()` reads it.
fn digit(c: char) -> Option<u32> {
    if let Some(d) = c.to_digit(10) {
        return Some(d);
    }
    // First code point (the zero) of every run of ten decimal digits outside ASCII.
    const ZEROS: &[u32] = &[
        0x0660, 0x06f0, 0x07c0, 0x0966, 0x09e6, 0x0a66, 0x0ae6, 0x0b66, 0x0be6, 0x0c66, 0x0ce6,
        0x0d66, 0x0de6, 0x0e50, 0x0ed0, 0x0f20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0,
        0x1a80, 0x1a90, 0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0,
        0xaa50, 0xabf0, 0xff10, 0x104a0, 0x10d30, 0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0,
        0x11450, 0x114d0, 0x11650, 0x116c0, 0x11730, 0x118e0, 0x11950, 0x11c50, 0x11d50, 0x11da0,
        0x16a60, 0x16b50, 0x1d7ce, 0x1d7d8, 0x1d7e2, 0x1d7ec, 0x1d7f6, 0x1e140, 0x1e2f0, 0x1e950,
        0x1fbf0,
    ];
    let n = c as u32;
    ZEROS
        .iter()
        .find(|&&z| (z..z + 10).contains(&n))
        .map(|z| n - z)
}

/// `int(s)` for a str: whitespace around, a sign, Unicode decimal digits, single underscores
/// between digits. Values beyond `i128` saturate (they only ever index out of range).
pub fn int(s: &str) -> Result<i128, String> {
    let bad = || {
        format!(
            "ValueError: invalid literal for int() with base 10: {}",
            repr(s)
        )
    };
    let t = strip(s);
    let (negative, digits) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    if digits.is_empty()
        || digits.starts_with('_')
        || digits.ends_with('_')
        || digits.contains("__")
    {
        return Err(bad());
    }
    let mut value: i128 = 0;
    for c in digits.chars().filter(|&c| c != '_') {
        let d = digit(c).ok_or_else(bad)?;
        value = value.saturating_mul(10).saturating_add(i128::from(d));
    }
    Ok(if negative { -value } else { value })
}

/// `round(x, ndigits)`: correctly rounded, ties to even, as CPython does.
pub fn round(x: f64, ndigits: usize) -> f64 {
    format!("{x:.ndigits$}").parse().unwrap_or(x)
}

/// `round(x)` to an int: ties to even.
pub fn round_int(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// `format(x, "g")`: six significant digits, no trailing zeros.
pub fn fmt_g(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let exp = x.abs().log10().floor() as i32;
    let rounded: f64 = format!("{x:.5e}").parse().unwrap_or(x);
    let exp = if rounded.abs() >= 10f64.powi(exp + 1) {
        exp + 1
    } else {
        exp
    };
    if (-4..6).contains(&exp) {
        let decimals = (5 - exp).max(0) as usize;
        let s = format!("{x:.decimals$}");
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        }
    } else {
        let s = format!("{x:.5e}");
        let (mantissa, e) = s.split_once('e').unwrap_or((&s, "0"));
        let mantissa = if mantissa.contains('.') {
            mantissa.trim_end_matches('0').trim_end_matches('.')
        } else {
            mantissa
        };
        let e: i32 = e.parse().unwrap_or(0);
        format!("{mantissa}e{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
    }
}

/// `str(Path(p))`: repeated slashes and `.` parts dropped, no trailing slash; `.` when empty.
pub fn path_str(p: &str) -> String {
    if p.is_empty() {
        return ".".into();
    }
    let lead = if p.starts_with("//") && !p.starts_with("///") {
        "//"
    } else if p.starts_with('/') {
        "/"
    } else {
        ""
    };
    let parts: Vec<&str> = p
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    let joined = format!("{lead}{}", parts.join("/"));
    if joined.is_empty() {
        ".".into()
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repr_matches_python() {
        assert_eq!(repr("nope"), "'nope'");
        assert_eq!(repr("it's"), "\"it's\"");
        assert_eq!(repr("a'b\"c"), "'a\\'b\"c'");
        assert_eq!(repr("tab\there"), "'tab\\there'");
        assert_eq!(repr("nb\u{a0}sp"), "'nb\\xa0sp'");
        assert_eq!(repr("héllo"), "'héllo'");
        assert_eq!(repr_list(&["left", "right"]), "['left', 'right']");
    }

    #[test]
    fn int_matches_python() {
        assert_eq!(int(" 07 "), Ok(7));
        assert_eq!(int("1_0"), Ok(10));
        assert_eq!(int("\u{661}\u{662}"), Ok(12));
        assert!(int("1__0").is_err());
        assert!(int("x").is_err());
        assert_eq!(int("-3"), Ok(-3));
    }

    #[test]
    fn number_formats() {
        assert_eq!(fmt_g(1.42), "1.42");
        assert_eq!(fmt_g(1.0), "1");
        assert_eq!(fmt_g(1.234_567_8), "1.23457");
        assert_eq!(round(0.125, 2), 0.12);
        assert_eq!(round_int(2.5), 2);
        assert_eq!(path_str("./out//x.pdf"), "out/x.pdf");
        assert_eq!(path_str("dir/"), "dir");
    }
}
