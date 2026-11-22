//! Text for templates (port of the helpers at the top of `view.py`).
//!
//! The original builds HTML strings (`Markup`); here text is a list of runs, each plain, bold,
//! linked or kept on one line, which the Typst templates draw. [`to_html`] turns runs back into
//! the original's markup, so the recorded cases check the parser.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use serde::ser::{SerializeSeq, Serializer};

use crate::pyfmt;

/// One stretch of text with one style.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    /// Link target (`<a href>`).
    pub url: Option<String>,
    /// Must not break across lines (`<span class="nobr">`, used for "++").
    pub nobr: bool,
    /// A CSS class the run carries in the original (`entry-project`).
    pub class: Option<&'static str>,
}

/// Formatted text: runs in order. Serializes as a plain string when it has no formatting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rich(pub Vec<Run>);

impl Rich {
    pub fn plain(text: impl Into<String>) -> Self {
        let text = text.into();
        if text.is_empty() {
            Rich(Vec::new())
        } else {
            Rich(vec![Run {
                text,
                ..Run::default()
            }])
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|r| r.text.is_empty())
    }

    /// The text without formatting.
    pub fn text(&self) -> String {
        self.0.iter().map(|r| r.text.as_str()).collect()
    }

    fn with_class(mut self, class: &'static str) -> Self {
        for run in &mut self.0 {
            run.class = Some(class);
        }
        self
    }
}

impl Rich {
    /// Runs for the typesetter: a "++" run joins the rest of its word on both sides, so the
    /// word ("C++", "C++17") never breaks. CSS keeps only the two plus signs together, but
    /// Unicode allows no break between a letter and "+" anyway; Typst would break before a
    /// box, so the box takes the whole word.
    fn for_layout(&self) -> Vec<Run> {
        if !self.0.iter().any(|r| r.nobr) {
            return self.0.clone();
        }
        let mut out: Vec<Run> = Vec::with_capacity(self.0.len());
        let same = |a: &Run, b: &Run| a.bold == b.bold && a.url == b.url && a.class == b.class;
        let mut glue_next = false;
        for run in &self.0 {
            if run.nobr {
                // Take the end of the previous run's last word.
                let mut word = String::new();
                if let Some(prev) = out
                    .last_mut()
                    .filter(|p| same(p, run) && !p.text.ends_with(crate::pyfmt::is_space))
                {
                    let cut = prev.text.rfind(crate::pyfmt::is_space).map_or(0, |i| {
                        i + prev.text[i..].chars().next().map_or(1, char::len_utf8)
                    });
                    word = prev.text.split_off(cut);
                    if prev.text.is_empty() {
                        let prev = out.pop();
                        if let Some(p) = prev.filter(|p| p.nobr) {
                            word = p.text + &word;
                        }
                    }
                }
                word.push_str(&run.text);
                out.push(Run {
                    text: word,
                    ..run.clone()
                });
                glue_next = true;
                continue;
            }
            if glue_next && let Some(last) = out.last_mut().filter(|l| same(l, run)) {
                let end = run
                    .text
                    .find(crate::pyfmt::is_space)
                    .unwrap_or(run.text.len());
                last.text.push_str(&run.text[..end]);
                let rest = &run.text[end..];
                glue_next = false;
                if !rest.is_empty() {
                    out.push(Run {
                        text: rest.to_string(),
                        ..run.clone()
                    });
                }
                continue;
            }
            glue_next = false;
            out.push(run.clone());
        }
        out
    }
}

impl Serialize for Rich {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let runs = self.for_layout();
        match runs.as_slice() {
            [] => s.serialize_str(""),
            [run] if !run.bold && !run.nobr && run.url.is_none() && run.class.is_none() => {
                s.serialize_str(&run.text)
            }
            runs => {
                let mut seq = s.serialize_seq(Some(runs.len()))?;
                for run in runs {
                    seq.serialize_element(&RunOut(run))?;
                }
                seq.end()
            }
        }
    }
}

/// A run for the templates: `{"t": text, "b": true, "u": url, "n": true, "c": class}`.
struct RunOut<'a>(&'a Run);

impl Serialize for RunOut<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let r = self.0;
        let n = 1
            + usize::from(r.bold)
            + usize::from(r.url.is_some())
            + 2 * usize::from(r.nobr)
            + usize::from(r.class.is_some());
        let mut map = s.serialize_map(Some(n))?;
        map.serialize_entry("t", &r.text)?;
        if r.bold {
            map.serialize_entry("b", &true)?;
        }
        if let Some(url) = &r.url {
            map.serialize_entry("u", url)?;
        }
        if r.nobr {
            map.serialize_entry("n", &true)?;
            map.serialize_entry("w", &crate::engine::Engine::get().width_bound(&r.text))?;
        }
        if let Some(class) = r.class {
            map.serialize_entry("c", class)?;
        }
        map.end()
    }
}

/// Split runs at "++" so the pair never breaks ("C++"): Unicode allows a break between the two
/// plus signs, and a reader or a resume parser then gets "C+ +".
fn keep_pluses(runs: Vec<Run>) -> Vec<Run> {
    let mut out = Vec::with_capacity(runs.len());
    for run in runs {
        if run.nobr || !run.text.contains("++") {
            out.push(run);
            continue;
        }
        let mut rest = run.text.as_str();
        while let Some(at) = rest.find("++") {
            if at > 0 {
                out.push(Run {
                    text: rest[..at].to_string(),
                    ..run.clone()
                });
            }
            out.push(Run {
                text: "++".into(),
                nobr: true,
                ..run.clone()
            });
            rest = &rest[at + 2..];
        }
        if !rest.is_empty() {
            out.push(Run {
                text: rest.to_string(),
                ..run
            });
        }
    }
    out
}

/// Plain text for a template, with "++" kept together (`keep`).
pub fn keep(text: Option<&str>) -> Option<Rich> {
    text.map(|t| Rich(keep_pluses(Rich::plain(t).0)))
}

/// `keep` for text that is always there.
pub fn keep_str(text: &str) -> Rich {
    Rich(keep_pluses(Rich::plain(text).0))
}

static BOLD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\*\*(.+?)\*\*").expect("static regex"));
static LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\(([^)\s]+)\)").expect("static regex"));

// Private-use marks for the tags `inline` inserts, so its two substitutions run on one string
// exactly as the original's run on HTML.
const B_OPEN: char = '\u{e000}';
const B_CLOSE: char = '\u{e001}';
const A_OPEN: char = '\u{e002}';
const A_MID: char = '\u{e003}';
const A_CLOSE: char = '\u{e004}';

/// Collapse whitespace and apply a tiny markdown subset: `**bold**` and `[label](url)` (`inline`).
pub fn inline(text: Option<&str>) -> Rich {
    let Some(text) = text.filter(|t| !t.is_empty()) else {
        return Rich::default();
    };
    let collapsed = pyfmt::collapse(text);
    let bolded = BOLD.replace_all(&collapsed, format!("{B_OPEN}${{1}}{B_CLOSE}"));
    let linked = LINK.replace_all(&bolded, format!("{A_OPEN}${{2}}{A_MID}${{1}}{A_CLOSE}"));

    // Walk the marks like an HTML parser would: misnested bold and links overlap as runs.
    let mut runs: Vec<Run> = Vec::new();
    let mut bold = false;
    let mut url: Option<String> = None;
    let mut buf = String::new();
    let mut chars = linked.chars().peekable();
    let flush = |buf: &mut String, runs: &mut Vec<Run>, bold: bool, url: &Option<String>| {
        if !buf.is_empty() {
            runs.push(Run {
                text: std::mem::take(buf),
                bold,
                url: url.clone(),
                ..Run::default()
            });
        }
    };
    while let Some(c) = chars.next() {
        match c {
            B_OPEN | B_CLOSE => {
                flush(&mut buf, &mut runs, bold, &url);
                bold = c == B_OPEN;
            }
            A_OPEN => {
                flush(&mut buf, &mut runs, bold, &url);
                // The URL is an attribute: bold marks inside it are literal tags.
                let mut href = String::new();
                for c in chars.by_ref() {
                    match c {
                        A_MID => break,
                        B_OPEN => href.push_str("<strong>"),
                        B_CLOSE => href.push_str("</strong>"),
                        c => href.push(c),
                    }
                }
                url = Some(href);
            }
            A_CLOSE => {
                flush(&mut buf, &mut runs, bold, &url);
                url = None;
            }
            c => buf.push(c),
        }
    }
    flush(&mut buf, &mut runs, bold, &url);
    Rich(keep_pluses(runs))
}

static PARAGRAPH_BREAK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\n\s*\n").expect("static regex"));

/// Paragraphs separated by blank lines, each through `inline` (`paragraphs`).
pub fn paragraphs(text: Option<&str>) -> Vec<Rich> {
    let Some(text) = text.filter(|t| !t.is_empty()) else {
        return Vec::new();
    };
    PARAGRAPH_BREAK
        .split(text)
        .filter(|p| !pyfmt::strip(p).is_empty())
        .map(|p| inline(Some(p)))
        .collect()
}

/// The project line of a work entry (`<span class="entry-project">`).
pub fn project(text: &str) -> Rich {
    keep_str(text).with_class("entry-project")
}

static URL_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(https?://)?(www\.)?").expect("static regex"));
static HAS_SCHEME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^[a-z][a-z0-9+.-]*:").expect("static regex"));

/// `https://www.example.org/` -> `example.org`.
pub fn pretty_url(url: &str) -> String {
    URL_PREFIX
        .replace(url, "")
        .trim_end_matches('/')
        .to_string()
}

/// A link target: the URL as it is when it has a scheme, else `https://` in front.
pub fn href(url: Option<&str>) -> Option<String> {
    let url = url.filter(|u| !u.is_empty())?;
    Some(if HAS_SCHEME.is_match(url) {
        url.to_string()
    } else {
        format!("https://{url}")
    })
}

const CEFR: [(&str, u8); 6] = [
    ("a1", 1),
    ("a2", 2),
    ("b1", 3),
    ("b2", 4),
    ("c1", 4),
    ("c2", 5),
];
const FLUENCY_WORDS: [(&str, u8); 30] = [
    ("native", 5),
    ("mother", 5),
    ("bilingual", 5),
    ("full professional", 5),
    ("fluent", 4),
    ("advanced", 4),
    ("professional", 4),
    ("proficient", 4),
    ("upper", 3),
    ("intermediate", 3),
    ("conversational", 3),
    ("elementary", 2),
    ("limited", 2),
    ("basic", 1),
    ("beginner", 1),
    // French and German, as a CV in those languages prints them
    ("maternelle", 5),
    ("bilingue", 5),
    ("muttersprache", 5),
    ("zweisprachig", 5),
    ("courant", 4),
    ("avancé", 4),
    ("fließend", 4),
    ("verhandlungssicher", 4),
    ("fortgeschritten", 4),
    ("intermédiaire", 3),
    ("mittelstufe", 3),
    ("élémentaire", 2),
    ("grundkenntnisse", 2),
    ("débutant", 1),
    ("anfänger", 1),
];
static CEFR_RE: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    CEFR.iter()
        .map(|(code, _)| Regex::new(&format!(r"\b{code}\b")).expect("static regex"))
        .collect()
});

/// The 0-5 dots of a language: its rating, else a CEFR level or a word in its fluency.
pub fn language_rating(fluency: Option<&str>, rating: Option<i64>) -> i64 {
    if let Some(r) = rating {
        return r;
    }
    let text = fluency.unwrap_or("").to_lowercase();
    for ((_, value), re) in CEFR.iter().zip(CEFR_RE.iter()) {
        if re.is_match(&text) {
            return i64::from(*value);
        }
    }
    FLUENCY_WORDS
        .iter()
        .find(|(w, _)| text.contains(w))
        .map_or(0, |(_, v)| i64::from(*v))
}

/// markupsafe's `escape`.
fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&#34;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
}

/// The original's markup for runs: what `keep`/`inline` return in Python. For the recorded cases.
pub fn to_html(rich: &Rich) -> String {
    let mut out = String::new();
    let mut i = 0;
    let runs = &rich.0;
    while i < runs.len() {
        let url = runs[i].url.clone();
        let mut j = i;
        while j < runs.len() && runs[j].url == url {
            j += 1;
        }
        if let Some(u) = &url {
            out.push_str("<a href=\"");
            // The original inserts the URL as matched, from already escaped text.
            escape(u, &mut out);
            out.push_str("\">");
        }
        let mut k = i;
        while k < j {
            let bold = runs[k].bold;
            let mut m = k;
            while m < j && runs[m].bold == bold {
                m += 1;
            }
            if bold {
                out.push_str("<strong>");
            }
            for run in &runs[k..m] {
                if run.nobr {
                    out.push_str("<span class=\"nobr\">++</span>");
                } else {
                    escape(&run.text, &mut out);
                }
            }
            if bold {
                out.push_str("</strong>");
            }
            k = m;
        }
        if url.is_some() {
            out.push_str("</a>");
        }
        i = j;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_markdown() {
        assert_eq!(
            to_html(&inline(Some("a  **b** [c](d)  e"))),
            "a <strong>b</strong> <a href=\"d\">c</a> e"
        );
        assert_eq!(
            to_html(&inline(Some("C++ & Rust"))),
            "C<span class=\"nobr\">++</span> &amp; Rust"
        );
        assert_eq!(to_html(&keep_str("x<y")), "x&lt;y");
        assert!(inline(None).is_empty());
    }

    #[test]
    fn plus_words_stay_whole() {
        let words = |r: &Rich| {
            r.for_layout()
                .iter()
                .map(|r| (r.text.clone(), r.nobr))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            words(&keep_str("in C++ now")),
            [
                ("in ".into(), false),
                ("C++".into(), true),
                (" now".into(), false)
            ]
        );
        assert_eq!(
            words(&keep_str("C++17, Rust")),
            [("C++17,".into(), true), (" Rust".into(), false)]
        );
        assert_eq!(words(&keep_str("C++")), [("C++".into(), true)]);
    }

    #[test]
    fn urls() {
        assert_eq!(pretty_url("https://www.example.org/"), "example.org");
        assert_eq!(
            href(Some("example.org")).as_deref(),
            Some("https://example.org")
        );
        assert_eq!(href(Some("mailto:a@b")).as_deref(), Some("mailto:a@b"));
        assert_eq!(href(Some("")), None);
    }

    #[test]
    fn ratings() {
        assert_eq!(language_rating(Some("Native"), None), 5);
        assert_eq!(language_rating(Some("B1 (intermediate)"), None), 3);
        assert_eq!(language_rating(None, Some(2)), 2);
        assert_eq!(language_rating(Some("??"), None), 0);
        assert_eq!(language_rating(Some("Langue maternelle"), None), 5);
        assert_eq!(language_rating(Some("Intermédiaire avancé"), None), 4);
        assert_eq!(language_rating(Some("Anfänger"), None), 1);
    }

    #[test]
    fn paragraphs_split_on_blank_lines() {
        let p = paragraphs(Some("one\n\n  two\nlines\n \n"));
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].text(), "two lines");
    }
}
