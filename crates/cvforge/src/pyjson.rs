//! JSON parsing with Python's `json` module's errors (CPython 3.13), for `load_cv`'s
//! `path:line:column: invalid JSON: message` reports.
//!
//! The fast path is serde_json. Only when it rejects the text does this scanner run, to either
//! accept what Python accepts (NaN, Infinity, lone surrogates) or name the first error at the
//! position Python names it, counted in characters.

use serde_json::{Map, Number, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub msg: String,
    pub line: usize,
    pub column: usize,
}

pub fn parse(text: &str) -> Result<Value, JsonError> {
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        return Ok(v);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut p = Scanner { s: &chars };
    p.document().map_err(|(msg, pos)| {
        let before = &chars[..pos.min(chars.len())];
        let line = before.iter().filter(|&&c| c == '\n').count() + 1;
        let column = match before.iter().rposition(|&c| c == '\n') {
            Some(nl) => pos - nl,
            None => pos + 1,
        };
        JsonError { msg, line, column }
    })
}

type Res<T> = Result<T, (String, usize)>;

struct Scanner<'a> {
    s: &'a [char],
}

fn err<T>(msg: &str, pos: usize) -> Res<T> {
    Err((msg.to_string(), pos))
}

impl Scanner<'_> {
    fn ws(&self, mut i: usize) -> usize {
        while i < self.s.len() && matches!(self.s[i], ' ' | '\t' | '\n' | '\r') {
            i += 1;
        }
        i
    }

    fn at(&self, i: usize) -> Option<char> {
        self.s.get(i).copied()
    }

    fn starts(&self, i: usize, word: &str) -> bool {
        let w: Vec<char> = word.chars().collect();
        self.s.len() >= i + w.len() && self.s[i..i + w.len()] == w[..]
    }

    fn document(&mut self) -> Res<Value> {
        if self.at(0) == Some('\u{feff}') {
            return err("Unexpected UTF-8 BOM (decode using utf-8-sig)", 0);
        }
        let start = self.ws(0);
        let (value, end) = match self.value(start)? {
            Some(x) => x,
            None => return err("Expecting value", start),
        };
        let end = self.ws(end);
        if end != self.s.len() {
            return err("Extra data", end);
        }
        Ok(value)
    }

    /// `scan_once`: `None` is Python's StopIteration (no value starts here).
    fn value(&mut self, i: usize) -> Res<Option<(Value, usize)>> {
        let Some(c) = self.at(i) else { return Ok(None) };
        let v = match c {
            '"' => {
                let (s, end) = self.string(i + 1)?;
                (Value::String(s), end)
            }
            '{' => self.object(i + 1)?,
            '[' => self.array(i + 1)?,
            'n' if self.starts(i, "null") => (Value::Null, i + 4),
            't' if self.starts(i, "true") => (Value::Bool(true), i + 4),
            'f' if self.starts(i, "false") => (Value::Bool(false), i + 5),
            'N' if self.starts(i, "NaN") => (Value::Null, i + 3),
            'I' if self.starts(i, "Infinity") => (Value::Null, i + 8),
            '-' if self.starts(i, "-Infinity") => (Value::Null, i + 9),
            _ => match self.number(i) {
                Some(x) => x,
                None => return Ok(None),
            },
        };
        Ok(Some(v))
    }

    fn number(&self, i: usize) -> Option<(Value, usize)> {
        let mut j = i;
        if self.at(j) == Some('-') {
            j += 1;
        }
        match self.at(j) {
            Some('0') => j += 1,
            Some('1'..='9') => {
                while matches!(self.at(j), Some('0'..='9')) {
                    j += 1;
                }
            }
            _ => return None,
        }
        let int_end = j;
        if self.at(j) == Some('.') && matches!(self.at(j + 1), Some('0'..='9')) {
            j += 2;
            while matches!(self.at(j), Some('0'..='9')) {
                j += 1;
            }
        }
        if matches!(self.at(j), Some('e' | 'E')) {
            let mut k = j + 1;
            if matches!(self.at(k), Some('+' | '-')) {
                k += 1;
            }
            if matches!(self.at(k), Some('0'..='9')) {
                while matches!(self.at(k), Some('0'..='9')) {
                    k += 1;
                }
                j = k;
            }
        }
        let text: String = self.s[i..j].iter().collect();
        let value = if j == int_end {
            text.parse::<i64>().map(Value::from).unwrap_or_else(|_| {
                text.parse::<f64>()
                    .ok()
                    .and_then(Number::from_f64)
                    .map_or(Value::Null, Value::Number)
            })
        } else {
            text.parse::<f64>()
                .ok()
                .and_then(Number::from_f64)
                .map_or(Value::Null, Value::Number)
        };
        Some((value, j))
    }

    fn string(&self, mut i: usize) -> Res<(String, usize)> {
        let begin = i - 1;
        let mut out = String::new();
        loop {
            let Some(c) = self.at(i) else {
                return err("Unterminated string starting at", begin);
            };
            match c {
                '"' => return Ok((out, i + 1)),
                '\\' => {
                    let Some(e) = self.at(i + 1) else {
                        return err("Unterminated string starting at", begin);
                    };
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let hex = |at: usize| -> Option<u32> {
                                let digits: String = self.s.get(at..at + 4)?.iter().collect();
                                if digits.chars().all(|c| c.is_ascii_hexdigit()) {
                                    u32::from_str_radix(&digits, 16).ok()
                                } else {
                                    None
                                }
                            };
                            let Some(mut code) = hex(i + 2) else {
                                return err("Invalid \\uXXXX escape", i + 1);
                            };
                            i += 6;
                            if (0xd800..0xdc00).contains(&code)
                                && self.at(i) == Some('\\')
                                && self.at(i + 1) == Some('u')
                                && let Some(low) =
                                    hex(i + 2).filter(|l| (0xdc00..0xe000).contains(l))
                            {
                                code = 0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00);
                                i += 6;
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                            continue;
                        }
                        _ => return err("Invalid \\escape", i),
                    }
                    i += 2;
                }
                c if (c as u32) < 0x20 => return err("Invalid control character at", i),
                c => {
                    out.push(c);
                    i += 1;
                }
            }
        }
    }

    fn object(&mut self, i: usize) -> Res<(Value, usize)> {
        let mut map = Map::new();
        let mut i = self.ws(i);
        if self.at(i) == Some('}') {
            return Ok((Value::Object(map), i + 1));
        }
        loop {
            if self.at(i) != Some('"') {
                return err("Expecting property name enclosed in double quotes", i);
            }
            let (key, end) = self.string(i + 1)?;
            i = self.ws(end);
            if self.at(i) != Some(':') {
                return err("Expecting ':' delimiter", i);
            }
            i = self.ws(i + 1);
            let Some((value, end)) = self.value(i)? else {
                return err("Expecting value", i);
            };
            map.insert(key, value);
            i = self.ws(end);
            match self.at(i) {
                Some('}') => return Ok((Value::Object(map), i + 1)),
                Some(',') => {
                    let comma = i;
                    i = self.ws(i + 1);
                    if self.at(i) == Some('}') {
                        return err("Illegal trailing comma before end of object", comma);
                    }
                }
                _ => return err("Expecting ',' delimiter", i),
            }
        }
    }

    fn array(&mut self, i: usize) -> Res<(Value, usize)> {
        let mut items = Vec::new();
        let mut i = self.ws(i);
        if self.at(i) == Some(']') {
            return Ok((Value::Array(items), i + 1));
        }
        loop {
            let Some((value, end)) = self.value(i)? else {
                return err("Expecting value", i);
            };
            items.push(value);
            i = self.ws(end);
            match self.at(i) {
                Some(']') => return Ok((Value::Array(items), i + 1)),
                Some(',') => {
                    let comma = i;
                    i = self.ws(i + 1);
                    if self.at(i) == Some(']') {
                        return err("Illegal trailing comma before end of array", comma);
                    }
                }
                _ => return err("Expecting ',' delimiter", i),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(text: &str) -> String {
        let e = parse(text).expect_err("invalid");
        format!("{}:{}: {}", e.line, e.column, e.msg)
    }

    #[test]
    fn errors_like_python() {
        assert_eq!(
            error("{\"basics\": {\"name\": \"A\",}\n"),
            "1:24: Illegal trailing comma before end of object"
        );
        assert_eq!(error(""), "1:1: Expecting value");
        assert_eq!(error("{\"a\" 1}"), "1:6: Expecting ':' delimiter");
        assert_eq!(error("[1 2]"), "1:4: Expecting ',' delimiter");
        assert_eq!(error("{}\n x"), "2:2: Extra data");
        assert_eq!(error("\"abc"), "1:1: Unterminated string starting at");
        assert!(parse("[NaN]").is_ok());
    }
}
