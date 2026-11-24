//! Cover letters in a CV's design (port of career-ledger's cv-builder `letter.py`).
//!
//! A letter comes already worded: the caller writes the date line, the subject, the salutation
//! and the closing in the letter's language. cv-forge only draws it, below the header the CV's
//! template draws on its first page, in that template's fonts, colours and accent, so a letter
//! and a CV sent together read as one application. Each template draws the letter itself
//! (`lettering` in its `cv.typ`), with the shared body `letter-body` from `lib.typ`.
//!
//! A letter flows as one document, with no measuring or pagination: a letter longer than a page
//! is the caller's to shorten, and the page count comes back with a note.

use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::model::{LoadError, Loc, Problem};
use crate::pyjson;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Recipient {
    pub company: String,
    pub lines: Vec<String>,
}

/// What a letter page prints, top to bottom (`Letter` in `letter.py`; the JSON's field names).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Letter {
    /// The letter's language tag; French (`fr`) puts the recipient on the right.
    pub language: String,
    pub recipient: Recipient,
    /// "Berlin, 24. November 2026"
    pub date_line: String,
    /// "Objet :" in French; empty elsewhere.
    pub subject_label: String,
    pub subject: String,
    pub salutation: String,
    pub paragraphs: Vec<String>,
    pub closing: String,
    pub signature: String,
    /// "Anlagen"
    pub enclosures_label: String,
    pub enclosures: Vec<String>,
}

impl Default for Letter {
    fn default() -> Self {
        Letter {
            language: "en".into(),
            recipient: Recipient::default(),
            date_line: String::new(),
            subject_label: String::new(),
            subject: String::new(),
            salutation: String::new(),
            paragraphs: Vec::new(),
            closing: String::new(),
            signature: String::new(),
            enclosures_label: String::new(),
            enclosures: Vec::new(),
        }
    }
}

/// What drawing a letter gave.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LetterResult {
    pub pages: usize,
    pub notes: Vec<String>,
}

/// Validation in pydantic's words, as for a CV: fields in declaration order, then unknown keys.
struct Walker {
    problems: Vec<Problem>,
}

impl Walker {
    fn push(&mut self, loc: Vec<Loc>, msg: &str) {
        self.problems.push(Problem {
            loc,
            msg: msg.into(),
        });
    }

    fn string(&mut self, loc: Vec<Loc>, v: Option<&Value>) -> String {
        match v {
            None => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(_) => {
                self.push(loc, "Input should be a valid string");
                String::new()
            }
        }
    }

    fn strings(&mut self, loc: Vec<Loc>, v: Option<&Value>) -> Vec<String> {
        match v {
            None => Vec::new(),
            Some(Value::Array(items)) => items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let mut at = loc.clone();
                    at.push(Loc::Index(i));
                    self.string(at, Some(item))
                })
                .collect(),
            Some(_) => {
                self.push(loc, "Input should be a valid list");
                Vec::new()
            }
        }
    }

    fn extras(&mut self, loc: &[Loc], input: &Map<String, Value>, known: &[&str]) {
        for k in input.keys().filter(|k| !known.contains(&k.as_str())) {
            let mut at = loc.to_vec();
            at.push(Loc::Key(k.clone()));
            self.push(at, "Extra inputs are not permitted");
        }
    }
}

const FIELDS: [&str; 11] = [
    "language",
    "recipient",
    "dateLine",
    "subjectLabel",
    "subject",
    "salutation",
    "paragraphs",
    "closing",
    "signature",
    "enclosuresLabel",
    "enclosures",
];

impl Letter {
    /// Validate a parsed JSON document: every problem, or the letter.
    pub fn from_value(value: &Value) -> Result<Letter, Vec<Problem>> {
        let Value::Object(input) = value else {
            return Err(vec![Problem {
                loc: Vec::new(),
                msg: "Input should be a valid dictionary or instance of Letter".into(),
            }]);
        };
        let mut w = Walker {
            problems: Vec::new(),
        };
        let key = |k: &str| vec![Loc::Key(k.into())];
        let language = match input.get("language") {
            None => "en".into(),
            given => w.string(key("language"), given),
        };
        let recipient = match input.get("recipient") {
            None => Recipient::default(),
            Some(Value::Object(r)) => {
                let at = |k: &str| vec![Loc::Key("recipient".into()), Loc::Key(k.into())];
                let company = w.string(at("company"), r.get("company"));
                let lines = w.strings(at("lines"), r.get("lines"));
                w.extras(&key("recipient"), r, &["company", "lines"]);
                Recipient { company, lines }
            }
            Some(_) => {
                w.push(
                    key("recipient"),
                    "Input should be a valid dictionary or instance of Recipient",
                );
                Recipient::default()
            }
        };
        let letter = Letter {
            language,
            recipient,
            date_line: w.string(key("dateLine"), input.get("dateLine")),
            subject_label: w.string(key("subjectLabel"), input.get("subjectLabel")),
            subject: w.string(key("subject"), input.get("subject")),
            salutation: w.string(key("salutation"), input.get("salutation")),
            paragraphs: w.strings(key("paragraphs"), input.get("paragraphs")),
            closing: w.string(key("closing"), input.get("closing")),
            signature: w.string(key("signature"), input.get("signature")),
            enclosures_label: w.string(key("enclosuresLabel"), input.get("enclosuresLabel")),
            enclosures: w.strings(key("enclosures"), input.get("enclosures")),
        };
        w.extras(&[], input, &FIELDS);
        if w.problems.is_empty() {
            Ok(letter)
        } else {
            Err(w.problems)
        }
    }

    /// Validate JSON text.
    pub fn from_json(text: &str) -> Result<Letter, LoadError> {
        let value = pyjson::parse(text).map_err(LoadError::Json)?;
        Letter::from_value(&value).map_err(LoadError::Invalid)
    }

    /// The language the letter is printed in: its own, or the CV's when it gives none. A tag
    /// Typst cannot take (its first subtag not two or three letters) falls back to the CV's.
    pub fn lang<'a>(&'a self, cv_lang: &'a str) -> &'a str {
        let own = self.language.as_str();
        let first = own.split('-').next().unwrap_or("");
        if (2..=3).contains(&first.len()) && first.chars().all(|c| c.is_ascii_alphabetic()) {
            own
        } else {
            cv_lang
        }
    }

    /// Whether the letter takes the French layout: cv-builder's `letter-page--fr` class, from
    /// the first subtag as written (`fr`, `fr-CA`; not `FR`, as CSS classes match exactly).
    pub fn french(&self) -> bool {
        self.language.split('-').next() == Some("fr")
    }

    /// The PDF's title: "name - subject", or the name alone.
    pub fn title(&self, name: &str) -> String {
        if self.subject.is_empty() {
            name.to_string()
        } else {
            format!("{name} - {}", self.subject)
        }
    }
}

/// HTML's whitespace rule for the letter's text: runs of ASCII whitespace (spaces, tabs,
/// newlines) are one space, none at either end. Typst keeps them as written.
pub fn collapse(text: &str) -> String {
    text.split([' ', '\t', '\n', '\r', '\x0c'])
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A part that is there when its text is not empty, as written (cv-builder's `{% if %}`); its
/// text with whitespace collapsed. A part whose text collapses to nothing takes no room, but
/// its margins still meet its neighbours'.
fn part(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| collapse(text))
}

#[derive(Serialize)]
pub(crate) struct RecipientView {
    company: Option<String>,
    lines: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct EnclosuresView {
    label: Option<String>,
    items: Vec<String>,
}

/// The letter as its template draws it (`data.letter`): each part `none` where cv-builder's
/// page leaves it out, its text as HTML would show it.
#[derive(Serialize)]
pub(crate) struct LetterView {
    french: bool,
    recipient: Option<RecipientView>,
    date: Option<String>,
    /// The label and the subject on one bold line.
    subject: Option<String>,
    salutation: Option<String>,
    paragraphs: Vec<String>,
    closing: Option<String>,
    signature: Option<String>,
    enclosures: Option<EnclosuresView>,
}

impl Letter {
    pub(crate) fn view(&self) -> LetterView {
        let r = &self.recipient;
        LetterView {
            french: self.french(),
            recipient: (!r.company.is_empty() || !r.lines.is_empty()).then(|| RecipientView {
                company: part(&r.company),
                lines: r.lines.iter().map(|l| collapse(l)).collect(),
            }),
            date: part(&self.date_line),
            subject: (!self.subject.is_empty()).then(|| {
                if self.subject_label.is_empty() {
                    collapse(&self.subject)
                } else {
                    collapse(&format!("{} {}", self.subject_label, self.subject))
                }
            }),
            salutation: part(&self.salutation),
            paragraphs: self.paragraphs.iter().map(|p| collapse(p)).collect(),
            closing: part(&self.closing),
            signature: part(&self.signature),
            enclosures: (!self.enclosures.is_empty()).then(|| EnclosuresView {
                label: part(&self.enclosures_label),
                items: self.enclosures.iter().map(|e| collapse(e)).collect(),
            }),
        }
    }
}

/// Read and validate a letter file.
pub fn load_letter(path: &Path) -> Result<Letter, LoadError> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(LoadError::NotFound),
        Err(e) => return Err(LoadError::Io(e)),
    };
    Letter::from_json(&text)
}

/// The CV's notes that say something about a letter: not those about its sections and columns.
pub fn letter_notes(cv_notes: &[String]) -> Vec<String> {
    cv_notes
        .iter()
        .filter(|n| !n.contains("layout.columns") && !n.starts_with("section "))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problems(json: &str) -> Vec<String> {
        match Letter::from_json(json) {
            Err(LoadError::Invalid(p)) => p
                .iter()
                .map(|p| format!("{}: {}", p.location(), p.msg))
                .collect(),
            Err(e) => vec![format!("{e:?}")],
            Ok(_) => vec![],
        }
    }

    #[test]
    fn defaults_and_problems() {
        let l = Letter::from_json("{}").expect("valid");
        assert_eq!(l, Letter::default());
        assert_eq!(l.language, "en");
        assert_eq!(
            problems(
                r#"{"subject": 5, "recipient": {"lines": ["a", null], "x": 1}, "paragraphs": "p", "foo": 1}"#
            ),
            [
                "recipient.lines[1]: Input should be a valid string",
                "recipient.x: Extra inputs are not permitted",
                "subject: Input should be a valid string",
                "paragraphs: Input should be a valid list",
                "foo: Extra inputs are not permitted",
            ]
        );
        assert_eq!(
            problems("[]"),
            ["(root): Input should be a valid dictionary or instance of Letter"]
        );
    }

    #[test]
    fn whitespace_as_html_shows_it() {
        assert_eq!(collapse("  a \n\tb  c "), "a b c");
        assert_eq!(collapse("a\u{a0} b"), "a\u{a0} b");
        assert_eq!(collapse(" "), "");
        let l = Letter {
            subject_label: "Objet :".into(),
            subject: " Candidature\n".into(),
            salutation: " ".into(),
            enclosures_label: "".into(),
            enclosures: vec!["CV".into()],
            ..Letter::default()
        };
        let v = serde_json::to_value(l.view()).expect("serializable");
        assert_eq!(v["subject"], "Objet : Candidature");
        assert_eq!(v["salutation"], "");
        assert_eq!(v["closing"], Value::Null);
        assert_eq!(v["recipient"], Value::Null);
        assert_eq!(v["enclosures"]["label"], Value::Null);
    }

    #[test]
    fn language_title_and_notes() {
        let l = Letter {
            language: "fr-CA".into(),
            subject: "Candidature".into(),
            ..Letter::default()
        };
        assert!(l.french());
        assert_eq!(l.lang("de"), "fr-CA");
        assert_eq!(l.title("Jane Doe"), "Jane Doe - Candidature");
        let odd = Letter {
            language: "français".into(),
            ..Letter::default()
        };
        assert!(!odd.french());
        assert_eq!(odd.lang("de"), "de");
        assert_eq!(Letter::default().title("Jane Doe"), "Jane Doe");
        let notes = [
            "section 'awards' has content but is not in layout.columns; it is not rendered".into(),
            "photo 'x.jpg' not found (looked in /tmp); rendering without it".into(),
        ];
        assert_eq!(letter_notes(&notes), [notes[1].clone()]);
    }
}
