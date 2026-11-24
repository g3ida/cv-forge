//! Templates: a `template.json` (the original's, unchanged) and a Typst file `cv.typ` that
//! replaces `cv.html.j2` + `style.css`. The bundled ones are compiled into the binary.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::build::RenderError;
use crate::paginate::TOLERANCE;
use crate::pyfmt;

pub const DEFAULT_TEMPLATE: &str = "compact";

/// Shared helpers every template imports as `/lib.typ`.
pub const SHARED_LIB: &str = include_str!("../../../templates/_shared/lib.typ");

macro_rules! bundled {
    ($($name:literal => $typ:expr),* $(,)?) => {
        &[$(($name, include_str!(concat!("../../../templates/", $name, "/template.json")), $typ)),*]
    };
}

/// (name, template.json, cv.typ). In the original's order (sorted folder names).
const BUNDLED: &[(&str, &str, Option<&str>)] = bundled![
    "awesome" => Some(include_str!("../../../templates/awesome/cv.typ")),
    "classic" => Some(include_str!("../../../templates/classic/cv.typ")),
    "clear" => Some(include_str!("../../../templates/clear/cv.typ")),
    "compact" => Some(include_str!("../../../templates/compact/cv.typ")),
    "contrast" => Some(include_str!("../../../templates/contrast/cv.typ")),
    "engineer" => Some(include_str!("../../../templates/engineer/cv.typ")),
    "executive" => Some(include_str!("../../../templates/executive/cv.typ")),
    "grid" => Some(include_str!("../../../templates/grid/cv.typ")),
    "modern" => Some(include_str!("../../../templates/modern/cv.typ")),
    "sidebar" => Some(include_str!("../../../templates/sidebar/cv.typ")),
    "timeline" => Some(include_str!("../../../templates/timeline/cv.typ")),
];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Margins {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub name: Option<String>,
    pub label: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "readingOrder", default)]
    pub reading_order: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<String>,
    pub margins: Margins,
    #[serde(rename = "accentColor")]
    pub accent_color: Option<String>,
    pub columns: IndexMap<String, Vec<String>>,
    #[serde(default)]
    pub titles: IndexMap<String, String>,
    /// Defaults for `layout.header`, as raw JSON (`"profiles": "*"` means every profile).
    #[serde(default)]
    pub header: serde_json::Map<String, Value>,
    #[serde(rename = "repeatTitles", default = "yes")]
    pub repeat_titles: bool,
    #[serde(rename = "splitEntries", default)]
    pub split_entries: bool,
    #[serde(rename = "fitTolerance", default = "tolerance")]
    pub fit_tolerance: f64,
}

fn yes() -> bool {
    true
}
fn tolerance() -> f64 {
    TOLERANCE
}

#[derive(Debug)]
pub struct Template {
    pub name: String,
    /// Where it came from: `bundled:<name>` or the folder, for telling templates apart.
    pub origin: PathBuf,
    pub config: Config,
    pub typst: Option<String>,
    /// The parsed Typst source, made once and shared by every build with this template.
    pub source: OnceLock<typst::syntax::Source>,
}

impl Template {
    pub fn label(&self) -> String {
        self.config
            .label
            .clone()
            .unwrap_or_else(|| title_case(&self.name.replace('-', " ")))
    }

    pub fn column_names(&self) -> Vec<String> {
        self.config.columns.keys().cloned().collect()
    }

    /// Where a section goes when the template's defaults do not place it.
    pub fn main_column(&self) -> String {
        self.config
            .columns
            .iter()
            .find(|(_, sections)| sections.iter().any(|s| s == "work"))
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| self.column_names().first().cloned().unwrap_or_default())
    }

    /// Sections laid out for another template, moved into this one's columns: the CV picks the
    /// sections, this template where and in what order; unknown ones end the main column.
    pub fn fit_columns(
        &self,
        columns: &IndexMap<String, Vec<String>>,
    ) -> IndexMap<String, Vec<String>> {
        let chosen: Vec<&String> = columns.values().flatten().collect();
        let mut out: IndexMap<String, Vec<String>> = self
            .config
            .columns
            .iter()
            .map(|(name, sections)| {
                (
                    name.clone(),
                    sections
                        .iter()
                        .filter(|s| chosen.contains(s))
                        .cloned()
                        .collect(),
                )
            })
            .collect();
        let placed: Vec<String> = out.values().flatten().cloned().collect();
        let main = self.main_column();
        let rest: Vec<String> = chosen
            .iter()
            .filter(|s| !placed.contains(s))
            .map(|s| (*s).clone())
            .collect();
        out.entry(main).or_default().extend(rest);
        out
    }

    fn bundled(name: &str) -> Option<Arc<Template>> {
        static CACHE: OnceLock<Vec<Arc<Template>>> = OnceLock::new();
        let all = CACHE.get_or_init(|| {
            BUNDLED
                .iter()
                .map(|(name, json, typ)| {
                    // INFALLIBLE: the bundled template.json files are checked by the tests.
                    let config: Config = serde_json::from_str(json).expect("bundled template.json");
                    Arc::new(Template {
                        name: config.name.clone().unwrap_or_else(|| (*name).to_string()),
                        origin: PathBuf::from(format!("bundled:{name}")),
                        config,
                        typst: typ.map(str::to_string),
                        source: OnceLock::new(),
                    })
                })
                .collect()
        });
        all.iter()
            .find(|t| t.origin == Path::new(&format!("bundled:{name}")))
            .cloned()
    }

    /// Each bundled template's `template.json` as written (name, text): its label, description,
    /// columns and aliases, for callers that describe the templates.
    pub fn bundled_configs() -> Vec<(&'static str, &'static str)> {
        BUNDLED
            .iter()
            .map(|(name, json, _)| (*name, *json))
            .collect()
    }

    /// Every bundled template, in the original's order.
    pub fn available() -> Vec<Arc<Template>> {
        BUNDLED
            .iter()
            .filter_map(|(name, _, _)| Template::bundled(name))
            .collect()
    }

    /// A template by name, former name (alias) or folder.
    pub fn load(name_or_path: Option<&str>) -> Result<Arc<Template>, RenderError> {
        let name = name_or_path.unwrap_or(DEFAULT_TEMPLATE);
        let dir = Path::new(name);
        if dir.join("template.json").is_file() {
            return Template::from_dir(dir);
        }
        if let Some(t) = Template::bundled(name) {
            return Ok(t);
        }
        if let Some(t) = Template::available()
            .into_iter()
            .find(|t| t.config.aliases.iter().any(|a| a == name))
        {
            return Ok(t);
        }
        let known: Vec<String> = Template::available()
            .iter()
            .map(|t| t.name.clone())
            .collect();
        Err(RenderError(format!(
            "unknown template {} (available: {})",
            pyfmt::repr(name),
            known.join(", ")
        )))
    }

    fn from_dir(dir: &Path) -> Result<Arc<Template>, RenderError> {
        let read = |file: &str| {
            std::fs::read_to_string(dir.join(file))
                .map_err(|e| RenderError(format!("{}: {e}", dir.join(file).display())))
        };
        let config: Config = serde_json::from_str(&read("template.json")?)
            .map_err(|e| RenderError(format!("{}: {e}", dir.display())))?;
        let typst = dir
            .join("cv.typ")
            .is_file()
            .then(|| read("cv.typ"))
            .transpose()?;
        let folder = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Arc::new(Template {
            name: config.name.clone().unwrap_or(folder),
            origin: dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf()),
            config,
            typst,
            source: OnceLock::new(),
        }))
    }
}

/// Python's `str.title()` for template names.
fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut start = true;
    for c in s.chars() {
        if c.is_alphabetic() {
            if start {
                out.extend(c.to_uppercase())
            } else {
                out.extend(c.to_lowercase())
            }
            start = false;
        } else {
            out.push(c);
            start = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_templates_load() {
        let names: Vec<String> = Template::available()
            .iter()
            .map(|t| t.name.clone())
            .collect();
        assert_eq!(
            names,
            [
                "awesome",
                "classic",
                "clear",
                "compact",
                "contrast",
                "engineer",
                "executive",
                "grid",
                "modern",
                "sidebar",
                "timeline"
            ]
        );
        assert_eq!(
            Template::load(Some("two-column")).expect("alias").name,
            "compact"
        );
        let e = Template::load(Some("nope")).expect_err("unknown");
        assert_eq!(
            e.0,
            "unknown template 'nope' (available: awesome, classic, clear, compact, contrast, engineer, executive, grid, modern, sidebar, timeline)"
        );
    }
}
