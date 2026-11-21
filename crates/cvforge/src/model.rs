//! The CV document format (port of `model.py`): JSON Resume v1.0.0 plus cv-builder's extensions.
//!
//! Validation walks the JSON against a schema table and reports every problem with pydantic's
//! wording, location and order (fields in declaration order, then unknown keys), so `validate`
//! prints what cv-builder printed. A valid document is then cleaned the way the original's
//! validators clean it (blank strings dropped, whitespace stripped, "Present" normalized) and
//! read into typed structs.

use std::path::Path;
use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{pyfmt, pyjson};

pub const SECTION_IDS: [&str; 14] = [
    "summary",
    "work",
    "volunteer",
    "education",
    "projects",
    "skills",
    "languages",
    "achievements",
    "awards",
    "certificates",
    "publications",
    "interests",
    "references",
    "profiles",
];

/// The JSON Schema cv-builder publishes (`cv-builder schema`), byte for byte.
pub const JSON_SCHEMA: &str = include_str!("../../../schema/cv.schema.json");

// ------------------------------------------------------------------ problems

/// One step of a problem's location: a key or a list index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Loc {
    Key(String),
    Index(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub loc: Vec<Loc>,
    pub msg: String,
}

impl Problem {
    /// `work[0].startDate`, as the CLI prints locations; `(root)` for the document itself.
    pub fn location(&self) -> String {
        let mut out = String::new();
        for part in &self.loc {
            match part {
                Loc::Index(i) => out.push_str(&format!("[{i}]")),
                Loc::Key(k) if out.is_empty() => out.push_str(k),
                Loc::Key(k) => {
                    out.push('.');
                    out.push_str(k);
                }
            }
        }
        if out.is_empty() { "(root)".into() } else { out }
    }
}

// ------------------------------------------------------------------ schema table

#[derive(Clone, Copy)]
enum Kind {
    Str,
    Date,
    EndDate,
    Int { ge: i64, le: i64 },
    Bool,
    Lit(&'static [&'static str]),
    Pattern(&'static str),
    List(&'static Kind),
    Model(&'static Spec),
    DictOf(&'static Kind),
    DictLitKeys(&'static [&'static str], &'static Kind),
}

#[derive(Clone, Copy)]
struct Field {
    name: &'static str,
    alias: Option<&'static str>,
    kind: Kind,
    required: bool,
    nullable: bool,
}

const fn opt(name: &'static str, kind: Kind) -> Field {
    Field {
        name,
        alias: None,
        kind,
        required: false,
        nullable: true,
    }
}
const fn req(name: &'static str, kind: Kind) -> Field {
    Field {
        name,
        alias: None,
        kind,
        required: true,
        nullable: false,
    }
}
/// Has a default and does not take `null` (lists, dicts and models with `default_factory`).
const fn def(name: &'static str, kind: Kind) -> Field {
    Field {
        name,
        alias: None,
        kind,
        required: false,
        nullable: false,
    }
}

type After = fn(&Map<String, Value>) -> Result<(), String>;

struct Spec {
    name: &'static str,
    fields: &'static [Field],
    /// `_Model` subclasses: forbid unknown keys, drop blank strings, strip whitespace.
    strict: bool,
    after: Option<After>,
}

const CONTACTS: &[&str] = &["phone", "email", "url", "location"];
const PAGE_SIZES: &[&str] = &["A4", "Letter"];
const HEX_COLOR: &str = r"^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$";
const STR_LIST: Kind = Kind::List(&Kind::Str);

static LOCATION: Spec = Spec {
    name: "Location",
    fields: &[
        opt("address", Kind::Str),
        opt("postalCode", Kind::Str),
        opt("city", Kind::Str),
        opt("region", Kind::Str),
        opt("countryCode", Kind::Str),
    ],
    strict: true,
    after: None,
};
static PROFILE: Spec = Spec {
    name: "Profile",
    fields: &[
        req("network", Kind::Str),
        opt("username", Kind::Str),
        opt("url", Kind::Str),
    ],
    strict: true,
    after: None,
};
static BASICS: Spec = Spec {
    name: "Basics",
    fields: &[
        req("name", Kind::Str),
        opt("label", Kind::Str),
        opt("image", Kind::Str),
        opt("email", Kind::Str),
        opt("phone", Kind::Str),
        opt("url", Kind::Str),
        opt("summary", Kind::Str),
        opt("location", Kind::Model(&LOCATION)),
        def("profiles", Kind::List(&Kind::Model(&PROFILE))),
    ],
    strict: true,
    after: None,
};
static TITLE_SPAN: Spec = Spec {
    name: "TitleSpan",
    fields: &[
        opt("startDate", Kind::Date),
        opt("endDate", Kind::EndDate),
        req("position", Kind::Str),
    ],
    strict: true,
    after: Some(end_after_start),
};
static WORK: Spec = Spec {
    name: "Work",
    fields: &[
        opt("startDate", Kind::Date),
        opt("endDate", Kind::EndDate),
        opt("name", Kind::Str),
        opt("position", Kind::Str),
        opt("location", Kind::Str),
        opt("url", Kind::Str),
        opt("description", Kind::Str),
        opt("project", Kind::Str),
        def("titleHistory", Kind::List(&Kind::Model(&TITLE_SPAN))),
        opt("summary", Kind::Str),
        def("highlights", STR_LIST),
    ],
    strict: true,
    after: Some(end_after_start),
};
static VOLUNTEER: Spec = Spec {
    name: "Volunteer",
    fields: &[
        opt("startDate", Kind::Date),
        opt("endDate", Kind::EndDate),
        opt("organization", Kind::Str),
        opt("position", Kind::Str),
        opt("location", Kind::Str),
        opt("url", Kind::Str),
        opt("summary", Kind::Str),
        def("highlights", STR_LIST),
    ],
    strict: true,
    after: Some(end_after_start),
};
static EDUCATION: Spec = Spec {
    name: "Education",
    fields: &[
        opt("startDate", Kind::Date),
        opt("endDate", Kind::EndDate),
        opt("institution", Kind::Str),
        opt("url", Kind::Str),
        opt("area", Kind::Str),
        opt("studyType", Kind::Str),
        opt("score", Kind::Str),
        def("courses", STR_LIST),
        opt("location", Kind::Str),
        def("highlights", STR_LIST),
    ],
    strict: true,
    after: Some(end_after_start),
};
static AWARD: Spec = Spec {
    name: "Award",
    fields: &[
        req("title", Kind::Str),
        opt("date", Kind::Date),
        opt("awarder", Kind::Str),
        opt("summary", Kind::Str),
    ],
    strict: true,
    after: None,
};
static CERTIFICATE: Spec = Spec {
    name: "Certificate",
    fields: &[
        req("name", Kind::Str),
        opt("date", Kind::Date),
        opt("issuer", Kind::Str),
        opt("url", Kind::Str),
    ],
    strict: true,
    after: None,
};
static PUBLICATION: Spec = Spec {
    name: "Publication",
    fields: &[
        req("name", Kind::Str),
        opt("publisher", Kind::Str),
        opt("releaseDate", Kind::Date),
        opt("url", Kind::Str),
        opt("summary", Kind::Str),
    ],
    strict: true,
    after: None,
};
static SKILL: Spec = Spec {
    name: "Skill",
    fields: &[
        req("name", Kind::Str),
        opt("level", Kind::Str),
        def("keywords", STR_LIST),
    ],
    strict: true,
    after: None,
};
static LANGUAGE: Spec = Spec {
    name: "Language",
    fields: &[
        req("language", Kind::Str),
        opt("fluency", Kind::Str),
        opt("rating", Kind::Int { ge: 0, le: 5 }),
    ],
    strict: true,
    after: None,
};
static INTEREST: Spec = Spec {
    name: "Interest",
    fields: &[req("name", Kind::Str), def("keywords", STR_LIST)],
    strict: true,
    after: None,
};
static REFERENCE: Spec = Spec {
    name: "Reference",
    fields: &[req("name", Kind::Str), opt("reference", Kind::Str)],
    strict: true,
    after: None,
};
static PROJECT: Spec = Spec {
    name: "Project",
    fields: &[
        opt("startDate", Kind::Date),
        opt("endDate", Kind::EndDate),
        req("name", Kind::Str),
        opt("description", Kind::Str),
        def("highlights", STR_LIST),
        def("keywords", STR_LIST),
        opt("url", Kind::Str),
        def("roles", STR_LIST),
        opt("entity", Kind::Str),
        opt("type", Kind::Str),
    ],
    strict: true,
    after: Some(end_after_start),
};
static ACHIEVEMENT: Spec = Spec {
    name: "Achievement",
    fields: &[
        req("title", Kind::Str),
        opt("summary", Kind::Str),
        opt("icon", Kind::Str),
    ],
    strict: true,
    after: None,
};
static META: Spec = Spec {
    name: "Meta",
    fields: &[
        opt("canonical", Kind::Str),
        opt("version", Kind::Str),
        opt("lastModified", Kind::Str),
    ],
    strict: false,
    after: None,
};
static HEADER_LAYOUT: Spec = Spec {
    name: "HeaderLayout",
    fields: &[
        def("contacts", Kind::List(&Kind::Lit(CONTACTS))),
        def("profiles", STR_LIST),
        def("photo", Kind::Bool),
    ],
    strict: true,
    after: None,
};
static LAYOUT: Spec = Spec {
    name: "Layout",
    fields: &[
        opt("template", Kind::Str),
        opt("pageSize", Kind::Lit(PAGE_SIZES)),
        opt("maxPages", Kind::Int { ge: 1, le: 20 }),
        opt("accentColor", Kind::Pattern(HEX_COLOR)),
        opt(
            "columns",
            Kind::DictOf(&Kind::List(&Kind::Lit(&SECTION_IDS))),
        ),
        def("titles", Kind::DictLitKeys(&SECTION_IDS, &Kind::Str)),
        def("header", Kind::Model(&HEADER_LAYOUT)),
        opt("documentTitle", Kind::Str),
    ],
    strict: true,
    after: Some(no_duplicate_sections),
};
static CV_SPEC: Spec = Spec {
    name: "CV",
    fields: &[
        Field {
            name: "schema_ref",
            alias: Some("$schema"),
            kind: Kind::Str,
            required: false,
            nullable: true,
        },
        req("basics", Kind::Model(&BASICS)),
        def("work", Kind::List(&Kind::Model(&WORK))),
        def("volunteer", Kind::List(&Kind::Model(&VOLUNTEER))),
        def("education", Kind::List(&Kind::Model(&EDUCATION))),
        def("awards", Kind::List(&Kind::Model(&AWARD))),
        def("certificates", Kind::List(&Kind::Model(&CERTIFICATE))),
        def("publications", Kind::List(&Kind::Model(&PUBLICATION))),
        def("skills", Kind::List(&Kind::Model(&SKILL))),
        def("languages", Kind::List(&Kind::Model(&LANGUAGE))),
        def("interests", Kind::List(&Kind::Model(&INTEREST))),
        def("references", Kind::List(&Kind::Model(&REFERENCE))),
        def("projects", Kind::List(&Kind::Model(&PROJECT))),
        def("achievements", Kind::List(&Kind::Model(&ACHIEVEMENT))),
        opt("meta", Kind::Model(&META)),
        def("layout", Kind::Model(&LAYOUT)),
    ],
    strict: true,
    after: None,
};

static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{4}(-(0[1-9]|1[0-2])(-(0[1-9]|[12]\d|3[01]))?)?$").expect("static regex")
});
static HEX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(HEX_COLOR).expect("static regex"));

fn check_date(value: &str) -> Result<(), String> {
    if DATE_RE.is_match(value) {
        Ok(())
    } else {
        Err(format!(
            "invalid date {}: use YYYY, YYYY-MM or YYYY-MM-DD",
            pyfmt::repr(value)
        ))
    }
}

/// Earliest and latest day a partial date covers.
fn date_bounds(value: &str) -> ((i128, i128, i128), (i128, i128, i128)) {
    // INFALLIBLE: called on dates that passed `check_date`, whose parts are digits.
    let parts: Vec<i128> = value
        .split('-')
        .map(|p| pyfmt::int(p).unwrap_or(0))
        .collect();
    let year = parts[0];
    let lo = (
        year,
        parts.get(1).copied().unwrap_or(1),
        parts.get(2).copied().unwrap_or(1),
    );
    let hi = (
        year,
        parts.get(1).copied().unwrap_or(12),
        parts.get(2).copied().unwrap_or(31),
    );
    (lo, hi)
}

fn end_after_start(m: &Map<String, Value>) -> Result<(), String> {
    let (Some(Value::String(start)), Some(Value::String(end))) =
        (m.get("startDate"), m.get("endDate"))
    else {
        return Ok(());
    };
    if end != "present" && date_bounds(end).1 < date_bounds(start).0 {
        return Err(format!("endDate {end} is before startDate {start}"));
    }
    Ok(())
}

fn no_duplicate_sections(m: &Map<String, Value>) -> Result<(), String> {
    let Some(Value::Object(columns)) = m.get("columns") else {
        return Ok(());
    };
    let mut seen: Vec<&str> = Vec::new();
    for sections in columns.values() {
        for s in sections
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if seen.contains(&s) {
                return Err(format!(
                    "section {} is listed more than once in layout.columns",
                    pyfmt::repr(s)
                ));
            }
            seen.push(s);
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ walker

struct Walker {
    problems: Vec<Problem>,
    path: Vec<Loc>,
}

impl Walker {
    fn push(&mut self, msg: impl Into<String>) {
        self.problems.push(Problem {
            loc: self.path.clone(),
            msg: msg.into(),
        });
    }

    fn at<T>(&mut self, loc: Loc, f: impl FnOnce(&mut Self) -> T) -> T {
        self.path.push(loc);
        let out = f(self);
        self.path.pop();
        out
    }

    fn literal_message(choices: &[&str]) -> String {
        let quoted: Vec<String> = choices.iter().map(|c| format!("'{c}'")).collect();
        match quoted.split_last() {
            Some((last, [])) => format!("Input should be {last}"),
            Some((last, rest)) => format!("Input should be {} or {last}", rest.join(", ")),
            None => "Input should be".into(),
        }
    }

    fn string(&mut self, v: &Value, strip: bool) -> Option<String> {
        match v {
            Value::String(s) => Some(if strip {
                pyfmt::strip(s).to_string()
            } else {
                s.clone()
            }),
            _ => {
                self.push("Input should be a valid string");
                None
            }
        }
    }

    fn kind(&mut self, kind: Kind, v: &Value, strip: bool) -> Option<Value> {
        match kind {
            Kind::Str => self.string(v, strip).map(Value::String),
            Kind::Date => {
                let s = self.string(v, strip)?;
                match check_date(&s) {
                    Ok(()) => Some(Value::String(s)),
                    Err(e) => {
                        self.push(e);
                        None
                    }
                }
            }
            Kind::EndDate => {
                let v = match v {
                    Value::String(s) if pyfmt::strip(s).to_lowercase() == "present" => {
                        Value::String("present".into())
                    }
                    other => other.clone(),
                };
                let s = self.string(&v, strip)?;
                if s == "present" {
                    return Some(Value::String(s));
                }
                match check_date(&s) {
                    Ok(()) => Some(Value::String(s)),
                    Err(e) => {
                        self.push(e);
                        None
                    }
                }
            }
            Kind::Int { ge, le } => {
                let n: i64 = match v {
                    Value::Bool(b) => i64::from(*b),
                    Value::Number(n) => match (n.as_i64(), n.as_f64()) {
                        (Some(i), _) => i,
                        (None, Some(f)) if f.fract() == 0.0 && f.is_finite() => f as i64,
                        (None, Some(f)) if f.is_finite() => {
                            self.push("Input should be a valid integer, got a number with a fractional part");
                            return None;
                        }
                        _ => {
                            self.push("Input should be a finite number");
                            return None;
                        }
                    },
                    Value::String(s) => match pyfmt::strip(s).parse::<i64>() {
                        Ok(i) => i,
                        Err(_) => {
                            self.push("Input should be a valid integer, unable to parse string as an integer");
                            return None;
                        }
                    },
                    _ => {
                        self.push("Input should be a valid integer");
                        return None;
                    }
                };
                if n < ge {
                    self.push(format!("Input should be greater than or equal to {ge}"));
                    return None;
                }
                if n > le {
                    self.push(format!("Input should be less than or equal to {le}"));
                    return None;
                }
                Some(Value::from(n))
            }
            Kind::Bool => {
                let b = match v {
                    Value::Bool(b) => Some(*b),
                    Value::Number(n) => match n.as_f64() {
                        Some(0.0) => Some(false),
                        Some(1.0) => Some(true),
                        _ => {
                            self.push("Input should be a valid boolean, unable to interpret input");
                            return None;
                        }
                    },
                    Value::String(s) => match s.to_lowercase().as_str() {
                        "0" | "off" | "f" | "false" | "n" | "no" => Some(false),
                        "1" | "on" | "t" | "true" | "y" | "yes" => Some(true),
                        _ => {
                            self.push("Input should be a valid boolean, unable to interpret input");
                            return None;
                        }
                    },
                    _ => None,
                };
                match b {
                    Some(b) => Some(Value::Bool(b)),
                    None => {
                        self.push("Input should be a valid boolean");
                        None
                    }
                }
            }
            Kind::Lit(choices) => match v {
                Value::String(s) if choices.contains(&s.as_str()) => Some(v.clone()),
                _ => {
                    self.push(Self::literal_message(choices));
                    None
                }
            },
            Kind::Pattern(pattern) => {
                let s = self.string(v, strip)?;
                if HEX_RE.is_match(&s) {
                    Some(Value::String(s))
                } else {
                    self.push(format!("String should match pattern '{pattern}'"));
                    None
                }
            }
            Kind::List(inner) => match v {
                Value::Array(items) => {
                    let mut out = Vec::with_capacity(items.len());
                    let mut ok = true;
                    for (i, item) in items.iter().enumerate() {
                        match self.at(Loc::Index(i), |w| w.kind(*inner, item, strip)) {
                            Some(x) => out.push(x),
                            None => ok = false,
                        }
                    }
                    ok.then_some(Value::Array(out))
                }
                _ => {
                    self.push("Input should be a valid list");
                    None
                }
            },
            Kind::Model(spec) => self.model(spec, v).map(Value::Object),
            Kind::DictOf(inner) => match v {
                Value::Object(m) => {
                    let mut out = Map::new();
                    let mut ok = true;
                    for (k, item) in m {
                        match self.at(Loc::Key(k.clone()), |w| w.kind(*inner, item, strip)) {
                            Some(x) => {
                                out.insert(k.clone(), x);
                            }
                            None => ok = false,
                        }
                    }
                    ok.then_some(Value::Object(out))
                }
                _ => {
                    self.push("Input should be a valid dictionary");
                    None
                }
            },
            Kind::DictLitKeys(keys, inner) => match v {
                Value::Object(m) => {
                    let mut out = Map::new();
                    let mut ok = true;
                    for (k, item) in m {
                        if !keys.contains(&k.as_str()) {
                            self.at(Loc::Key(k.clone()), |w| {
                                w.at(Loc::Key("[key]".into()), |w| {
                                    w.push(Self::literal_message(keys))
                                })
                            });
                            ok = false;
                        }
                        match self.at(Loc::Key(k.clone()), |w| w.kind(*inner, item, strip)) {
                            Some(x) => {
                                out.insert(k.clone(), x);
                            }
                            None => ok = false,
                        }
                    }
                    ok.then_some(Value::Object(out))
                }
                _ => {
                    self.push("Input should be a valid dictionary");
                    None
                }
            },
        }
    }

    fn model(&mut self, spec: &Spec, v: &Value) -> Option<Map<String, Value>> {
        let Value::Object(input) = v else {
            self.push(format!(
                "Input should be a valid dictionary or instance of {}",
                spec.name
            ));
            return None;
        };
        let blank = |v: &Value| matches!(v, Value::String(s) if pyfmt::strip(s).is_empty());
        let mut out = Map::new();
        let mut ok = true;
        for f in spec.fields {
            let key = f.alias.filter(|a| input.contains_key(*a)).unwrap_or(f.name);
            let given = input.get(key).filter(|v| !(spec.strict && blank(v)));
            match given {
                None if f.required => {
                    self.at(Loc::Key(f.alias.unwrap_or(f.name).into()), |w| {
                        w.push("Field required")
                    });
                    ok = false;
                }
                None => {}
                Some(Value::Null) if f.nullable => {
                    out.insert(f.name.into(), Value::Null);
                }
                Some(value) => {
                    match self.at(Loc::Key(key.into()), |w| w.kind(f.kind, value, spec.strict)) {
                        Some(x) => {
                            out.insert(f.name.into(), x);
                        }
                        None => ok = false,
                    }
                }
            }
        }
        let known = |k: &str| {
            spec.fields
                .iter()
                .any(|f| f.name == k || f.alias == Some(k))
        };
        for (k, value) in input {
            if known(k) || (spec.strict && blank(value)) {
                continue;
            }
            if spec.strict {
                self.at(Loc::Key(k.clone()), |w| {
                    w.push("Extra inputs are not permitted")
                });
                ok = false;
            } else {
                out.insert(k.clone(), value.clone());
            }
        }
        if !ok {
            return None;
        }
        if let Some(after) = spec.after
            && let Err(msg) = after(&out)
        {
            self.push(msg);
            return None;
        }
        Some(out)
    }
}

// ------------------------------------------------------------------ typed document

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Location {
    pub address: Option<String>,
    #[serde(rename = "postalCode")]
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    #[serde(rename = "countryCode")]
    pub country_code: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub network: String,
    pub username: Option<String>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Basics {
    pub name: String,
    pub label: Option<String>,
    pub image: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub url: Option<String>,
    pub summary: Option<String>,
    pub location: Option<Location>,
    pub profiles: Vec<Profile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct TitleSpan {
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub position: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Work {
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub name: Option<String>,
    pub position: Option<String>,
    pub location: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub project: Option<String>,
    #[serde(rename = "titleHistory")]
    pub title_history: Vec<TitleSpan>,
    pub summary: Option<String>,
    pub highlights: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Volunteer {
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub organization: Option<String>,
    pub position: Option<String>,
    pub location: Option<String>,
    pub url: Option<String>,
    pub summary: Option<String>,
    pub highlights: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Education {
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub institution: Option<String>,
    pub url: Option<String>,
    pub area: Option<String>,
    #[serde(rename = "studyType")]
    pub study_type: Option<String>,
    pub score: Option<String>,
    pub courses: Vec<String>,
    pub location: Option<String>,
    pub highlights: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Award {
    pub title: String,
    pub date: Option<String>,
    pub awarder: Option<String>,
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Certificate {
    pub name: String,
    pub date: Option<String>,
    pub issuer: Option<String>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Publication {
    pub name: String,
    pub publisher: Option<String>,
    #[serde(rename = "releaseDate")]
    pub release_date: Option<String>,
    pub url: Option<String>,
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Skill {
    pub name: String,
    pub level: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Language {
    pub language: String,
    pub fluency: Option<String>,
    pub rating: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Interest {
    pub name: String,
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Reference {
    pub name: String,
    pub reference: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Project {
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub highlights: Vec<String>,
    pub keywords: Vec<String>,
    pub url: Option<String>,
    pub roles: Vec<String>,
    pub entity: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Achievement {
    pub title: String,
    pub summary: Option<String>,
    pub icon: Option<String>,
}

/// `layout.header` as written: `None` for a field the CV leaves out (the template's default
/// then applies; pydantic's `model_fields_set`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct HeaderSet {
    pub contacts: Option<Vec<String>>,
    pub profiles: Option<Vec<String>>,
    pub photo: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub template: Option<String>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<String>,
    #[serde(rename = "maxPages")]
    pub max_pages: Option<i64>,
    #[serde(rename = "accentColor")]
    pub accent_color: Option<String>,
    pub columns: Option<IndexMap<String, Vec<String>>>,
    pub titles: IndexMap<String, String>,
    pub header: HeaderSet,
    #[serde(rename = "documentTitle")]
    pub document_title: Option<String>,
}

/// A validated CV document.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Cv {
    pub basics: Basics,
    pub work: Vec<Work>,
    pub volunteer: Vec<Volunteer>,
    pub education: Vec<Education>,
    pub awards: Vec<Award>,
    pub certificates: Vec<Certificate>,
    pub publications: Vec<Publication>,
    pub skills: Vec<Skill>,
    pub languages: Vec<Language>,
    pub interests: Vec<Interest>,
    pub references: Vec<Reference>,
    pub projects: Vec<Project>,
    pub achievements: Vec<Achievement>,
    pub layout: Layout,
}

impl Cv {
    /// Validate a parsed JSON document; every problem, or the CV.
    pub fn from_value(value: &Value) -> Result<Cv, Vec<Problem>> {
        let mut walker = Walker {
            problems: Vec::new(),
            path: Vec::new(),
        };
        let cleaned = walker.model(&CV_SPEC, value);
        match cleaned {
            Some(map) if walker.problems.is_empty() => serde_json::from_value(Value::Object(map))
                .map_err(|e| {
                    vec![Problem {
                        loc: Vec::new(),
                        msg: e.to_string(),
                    }]
                }),
            _ => Err(walker.problems),
        }
    }

    /// Validate JSON text.
    pub fn from_json(text: &str) -> Result<Cv, LoadError> {
        let value = pyjson::parse(text).map_err(LoadError::Json)?;
        Cv::from_value(&value).map_err(LoadError::Invalid)
    }
}

/// Why a CV file could not be loaded.
#[derive(Debug)]
pub enum LoadError {
    NotFound,
    Io(std::io::Error),
    Json(pyjson::JsonError),
    Invalid(Vec<Problem>),
}

impl LoadError {
    /// The report cv-builder prints for `path` (`load_cv`'s ValueError text).
    pub fn report(&self, path: &str) -> String {
        match self {
            LoadError::NotFound => format!("{path}: file not found"),
            LoadError::Io(e) => format!("{path}: {}", io_message(e)),
            LoadError::Json(e) => {
                format!("{path}:{}:{}: invalid JSON: {}", e.line, e.column, e.msg)
            }
            LoadError::Invalid(problems) => {
                let mut out = format!("{path}: {} problem(s)", problems.len());
                for p in problems {
                    out.push_str(&format!("\n  {}: {}", p.location(), p.msg));
                }
                out
            }
        }
    }
}

/// A file error in a word or two, for the messages where cv-builder crashed (DECISIONS.md
/// ADR-015): an empty path is the current folder there, which Python cannot read or write.
pub fn io_message(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => "file not found".into(),
        std::io::ErrorKind::IsADirectory => "is a directory".into(),
        std::io::ErrorKind::PermissionDenied => "permission denied".into(),
        _ => e.to_string(),
    }
}

/// Read and validate a CV file (`load_cv`).
pub fn load_cv(path: &Path) -> Result<Cv, LoadError> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(LoadError::NotFound),
        Err(e) => return Err(LoadError::Io(e)),
    };
    Cv::from_json(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problems(json: &str) -> Vec<String> {
        match Cv::from_json(json) {
            Err(LoadError::Invalid(p)) => p
                .iter()
                .map(|p| format!("{}: {}", p.location(), p.msg))
                .collect(),
            Err(e) => vec![format!("{e:?}")],
            Ok(_) => vec![],
        }
    }

    #[test]
    fn reports_like_pydantic() {
        let bad = r#"{"basics": {"name": "A", "email": 5, "foo": 1}, "work": [{"startDate": "2020-13", "endDate": "2019"}],
            "layout": {"maxPages": 0, "accentColor": "blue", "pageSize": "A5", "columns": {"main": ["work", "work", "nope"]}}}"#;
        let p = problems(bad);
        assert_eq!(p[0], "basics.email: Input should be a valid string");
        assert_eq!(p[1], "basics.foo: Extra inputs are not permitted");
        assert_eq!(
            p[2],
            "work[0].startDate: invalid date '2020-13': use YYYY, YYYY-MM or YYYY-MM-DD"
        );
        assert_eq!(p[3], "layout.pageSize: Input should be 'A4' or 'Letter'");
        assert_eq!(
            p[4],
            "layout.maxPages: Input should be greater than or equal to 1"
        );
        assert_eq!(p.len(), 7);
        assert_eq!(
            problems("[]"),
            vec!["(root): Input should be a valid dictionary or instance of CV"]
        );
        assert_eq!(
            problems(r#"{"basics": {}}"#),
            vec!["basics.name: Field required"]
        );
        assert_eq!(
            problems(
                r#"{"basics":{"name":"A"},"education":[{"startDate":"2020-05","endDate":"2020-04"}]}"#
            ),
            vec!["education[0]: endDate 2020-04 is before startDate 2020-05"]
        );
    }

    #[test]
    fn cleans_like_the_validators() {
        let cv = Cv::from_json(r#"{"basics": {"name": " A ", "email": "  "}, "work": [{"name": "", "startDate": "2020", "endDate": " Present "}]}"#)
            .expect("valid");
        assert_eq!(cv.basics.name, "A");
        assert_eq!(cv.basics.email, None);
        assert_eq!(cv.work[0].end_date.as_deref(), Some("present"));
        assert!(cv.layout.header.photo.is_none());
    }
}
