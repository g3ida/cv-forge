//! Build a CV's PDF (port of `render.py`), with Typst in place of WeasyPrint.
//!
//! Two passes, as in the original:
//! 1. *Measure*: the template measures the header, every section title and every item at the
//!    column's width and reports their heights (`#metadata(..) <cvforge-measure>`).
//! 2. *Paginate & print*: items are distributed over real pages ([`paginate`]) and the final
//!    document is typeset to PDF.
//!
//! Fitting to a page limit tightens line spacing (and nothing else) by binary search, one
//! measure pass per step.

use std::collections::HashMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use base64::Engine as _;
use indexmap::IndexMap;
use serde::Serialize;
use typst::foundations::{Dict, Label, Selector, Smart, Value};
use typst::introspection::{Introspector, MetadataElem};
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use crate::engine::{self, Engine, Job};
use crate::letter::{Letter, LetterResult, LetterView, letter_notes};
use crate::model::{Cv, SECTION_IDS};
use crate::paginate::{Overflow, Unit, paginate_column};
use crate::pyfmt;
use crate::template::{Margins, Template};
use crate::view::{
    Header, HeaderLayout, Item, Section, build_header, build_sections, split_entries,
};
use crate::wording::{CvLocale, cv_locale};

pub const PT_TO_PX: f64 = 4.0 / 3.0;
/// Fitting never sets text line spacing below this (the low end of the usual 1.15 to 1.5).
pub const LINE_HEIGHT_FLOOR: f64 = 1.15;
/// Binary-search steps between the template's spacing and the tightest: to within 3%.
pub const FIT_STEPS: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderError(pub String);

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RenderError {}

pub fn page_size_pt(name: &str) -> (f64, f64) {
    match name {
        "Letter" => (612.0, 792.0),
        _ => (595.28, 841.89),
    }
}

/// Command-line overrides; `None` keeps the CV's layout or the template's default.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub template: Option<String>,
    pub accent: Option<String>,
    pub page_size: Option<String>,
    pub max_pages: Option<i64>,
}

#[derive(Debug)]
pub struct Settings {
    pub template: Arc<Template>,
    pub page_size: String,
    pub accent: String,
    pub max_pages: Option<i64>,
    pub columns: IndexMap<String, Vec<String>>,
    pub titles: IndexMap<String, String>,
    pub document_title: String,
    pub header: HeaderLayout,
    /// The CV's language: headings, months, "Present" (`layout.language`, `layout.dateStyle`).
    pub wording: CvLocale,
}

/// `layout.header`, with the template's defaults for the fields the CV leaves out.
fn resolve_header(cv: &Cv, template: &Template) -> HeaderLayout {
    let set = &cv.layout.header;
    let defaults = &template.config.header;
    let strings = |v: &serde_json::Value| -> Option<Vec<String>> {
        v.as_array().map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
    };
    let contacts = set
        .contacts
        .clone()
        .or_else(|| defaults.get("contacts").and_then(strings))
        .unwrap_or_else(|| vec!["phone".into(), "email".into(), "url".into()]);
    let profiles = set
        .profiles
        .clone()
        .or_else(|| match defaults.get("profiles") {
            Some(serde_json::Value::String(s)) if s == "*" => Some(
                cv.basics
                    .profiles
                    .iter()
                    .map(|p| p.network.clone())
                    .collect(),
            ),
            Some(v) => strings(v),
            None => None,
        })
        .unwrap_or_else(|| vec!["LinkedIn".into()]);
    let photo = set
        .photo
        .or_else(|| defaults.get("photo").and_then(serde_json::Value::as_bool))
        .unwrap_or(true);
    HeaderLayout {
        contacts,
        profiles,
        photo,
    }
}

/// Whether `-t` picked another template than the one the CV names (aliases count as the same).
fn other_template(chosen: Option<&str>, laid_out_for: Option<&str>, template: &Template) -> bool {
    if chosen.is_none() {
        return false;
    }
    match Template::load(laid_out_for) {
        Ok(t) => t.origin != template.origin,
        Err(_) => true,
    }
}

pub fn resolve_settings(
    cv: &Cv,
    options: &Options,
) -> Result<(Settings, Vec<String>), RenderError> {
    let layout = &cv.layout;
    // Python's `or`: an empty `-t ''` falls back to the CV's template.
    let template = Template::load(
        options
            .template
            .as_deref()
            .filter(|t| !t.is_empty())
            .or(layout.template.as_deref()),
    )?;
    let cfg = &template.config;
    let mut notes = Vec::new();
    let names = template.column_names();

    let mut columns = layout
        .columns
        .clone()
        .unwrap_or_else(|| cfg.columns.clone());
    if let Some(given) = &layout.columns {
        let mut given_keys: Vec<&String> = given.keys().collect();
        given_keys.sort();
        let mut own: Vec<&String> = names.iter().collect();
        own.sort();
        if given_keys != own {
            if !other_template(
                options.template.as_deref(),
                layout.template.as_deref(),
                &template,
            ) {
                let unknown: Vec<&String> = given_keys
                    .iter()
                    .filter(|k| !names.contains(k))
                    .copied()
                    .collect();
                if !unknown.is_empty() {
                    return Err(RenderError(format!(
                        "layout.columns uses {} but template {} has columns {}",
                        pyfmt::repr_list(&unknown),
                        pyfmt::repr(&template.name),
                        pyfmt::repr_list(&names)
                    )));
                }
            } else {
                // The command line picked another template than the CV's: keep the CV's choice
                // and order of sections, in this template's columns.
                columns = template.fit_columns(&columns);
                notes.push(format!(
                    "layout.columns names {} but template {} has columns {}; its sections were moved into them",
                    pyfmt::repr_list(&given_keys),
                    pyfmt::repr(&template.name),
                    pyfmt::repr_list(&names)
                ));
            }
        }
    }
    let header = resolve_header(cv, &template);
    let placed: Vec<&String> = columns.values().flatten().collect();
    let wording = cv_locale(layout.language.as_deref(), layout.date_style.as_deref());
    let sections = build_sections(cv, &IndexMap::new(), &header.profiles, &wording);
    for sid in SECTION_IDS {
        if sections.contains_key(sid) && !placed.iter().any(|p| *p == sid) {
            notes.push(format!(
                "section {} has content but is not in layout.columns; it is not rendered",
                pyfmt::repr(sid)
            ));
        }
    }
    let page_size = options
        .page_size
        .clone()
        .or_else(|| layout.page_size.clone())
        .or_else(|| cfg.page_size.clone())
        .unwrap_or_else(|| "A4".into());
    let accent = options
        .accent
        .clone()
        .filter(|a| !a.is_empty())
        .or_else(|| layout.accent_color.clone())
        .or_else(|| cfg.accent_color.clone())
        .unwrap_or_else(|| "#1E90FF".into());
    let mut titles = cfg.titles.clone();
    titles.extend(layout.titles.clone());
    let settings = Settings {
        page_size,
        accent,
        max_pages: options.max_pages.filter(|&n| n != 0).or(layout.max_pages),
        columns: names
            .iter()
            .map(|n| (n.clone(), columns.get(n).cloned().unwrap_or_default()))
            .collect(),
        titles,
        document_title: layout
            .document_title
            .clone()
            .unwrap_or_else(|| format!("{} - {}", cv.basics.name, wording.cv_word)),
        header,
        wording,
        template,
    };
    Ok((settings, notes))
}

// ------------------------------------------------------------------ page model

/// The part of a section that lands in one column of one page: item indexes.
#[derive(Clone, Debug)]
pub struct Chunk {
    pub id: &'static str,
    pub continued: bool,
    pub items: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Page {
    pub header: bool,
    pub columns: IndexMap<String, Vec<Chunk>>,
}

/// One measure pass, in CSS px.
#[derive(Clone, Debug)]
pub struct Measure {
    pub units: IndexMap<String, Vec<Unit>>,
    pub title_heights: HashMap<&'static str, f64>,
    pub header_height: f64,
    pub text_px: f64,
    pub line_px: f64,
    pub line_width_px: f64,
}

impl Measure {
    /// The measure at tightness `t`, on the line through the measures at 0 and 1.
    ///
    /// Fitting changes line spacing only: line breaks stay, so every position is the sum of
    /// line heights (each `v - t (v - 1.15)`, linear in `t`) and fixed space, and is linear in
    /// `t`. Lines whose height is set by something of fixed height (an icon box taller than the
    /// text) make it convex instead; `fit` checks one point against a real pass and falls back
    /// to real passes when it is off.
    fn between(loose: &Measure, tight: &Measure, t: f64) -> Measure {
        let mix = |a: f64, b: f64| a + t * (b - a);
        Measure {
            units: loose
                .units
                .iter()
                .map(|(col, units)| {
                    let other = &tight.units[col];
                    let list = units
                        .iter()
                        .zip(other)
                        .map(|(a, b)| Unit {
                            top: mix(a.top, b.top),
                            bottom: mix(a.bottom, b.bottom),
                            ..*a
                        })
                        .collect();
                    (col.clone(), list)
                })
                .collect(),
            title_heights: loose
                .title_heights
                .iter()
                .map(|(k, v)| (*k, mix(*v, tight.title_heights[k])))
                .collect(),
            header_height: mix(loose.header_height, tight.header_height),
            text_px: loose.text_px,
            line_px: mix(loose.line_px, tight.line_px),
            line_width_px: loose.line_width_px,
        }
    }

    /// Whether two measures agree to within a millionth of a px everywhere.
    fn same_as(&self, other: &Measure) -> bool {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
        close(self.header_height, other.header_height)
            && close(self.line_px, other.line_px)
            && self
                .title_heights
                .iter()
                .all(|(k, v)| other.title_heights.get(k).is_some_and(|w| close(*v, *w)))
            && self.units.iter().all(|(col, units)| {
                other.units.get(col).is_some_and(|o| {
                    o.len() == units.len()
                        && units
                            .iter()
                            .zip(o)
                            .all(|(a, b)| close(a.top, b.top) && close(a.bottom, b.bottom))
                })
            })
    }
}

/// Measured units distributed over pages, per column.
#[derive(Clone, Debug)]
pub struct Pagination {
    pub columns: IndexMap<String, Vec<Vec<Unit>>>,
    pub overflows: Vec<Overflow>,
    /// What a continued section repeats (zero when it repeats nothing).
    pub title_heights: HashMap<&'static str, f64>,
    pub header_height: f64,
    pub content_height: f64,
}

impl Pagination {
    fn available(&self, index: usize) -> f64 {
        self.content_height - if index == 0 { self.header_height } else { 0.0 }
    }

    fn used(&self, page: &[Unit]) -> f64 {
        let (Some(start), Some(end)) = (page.first(), page.last()) else {
            return 0.0;
        };
        end.bottom - start.top
            + if start.opens_section() {
                0.0
            } else {
                self.title_heights
                    .get(start.section)
                    .copied()
                    .unwrap_or(0.0)
            }
    }

    /// Pages the PDF will have. An item taller than a page runs on over the next ones.
    pub fn pages(&self) -> usize {
        let mut most = 1;
        for pages in self.columns.values() {
            let spill: usize = pages
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    ((self.used(p) / self.available(i) - 1e-6).ceil() as i64 - 1).max(0) as usize
                })
                .sum();
            most = most.max(pages.len() + spill);
        }
        most
    }

    /// Height of the content that does not fit on the first `limit` pages, in the column with
    /// the most: what runs past the bottom of those pages, and every page after them.
    pub fn beyond(&self, limit: usize) -> f64 {
        let mut worst: f64 = 0.0;
        for pages in self.columns.values() {
            let over: f64 = pages
                .iter()
                .take(limit)
                .enumerate()
                .map(|(i, p)| (self.used(p) - self.available(i)).max(0.0))
                .sum();
            let after: f64 = pages.iter().skip(limit).map(|p| self.used(p)).sum();
            worst = worst.max(over + after);
        }
        worst
    }

    /// How full the last page is (0 to 1), in the column that fills it most.
    pub fn last_page_fill(&self) -> f64 {
        let n = self
            .columns
            .values()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .max(1);
        let used = self
            .columns
            .values()
            .filter(|p| p.len() == n)
            .map(|p| self.used(&p[n - 1]))
            .fold(0.0_f64, f64::max);
        pyfmt::round((used / self.available(n - 1)).min(1.0), 3)
    }
}

/// What fitting to a page limit did (`layout.maxPages` / `--max-pages`).
///
/// `tightness`: how far line spacing went from the template's own (0) toward the floor of 1.15
/// (1). Text size and margins never change. When the content does not fit even at the
/// tightest, it is drawn as designed and `over_lines` says about how many lines of body text
/// (of about `line_chars` characters) have to go.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Fit {
    pub max_pages: i64,
    pub fits: bool,
    pub tightness: f64,
    pub line_height: f64,
    pub designed_line_height: f64,
    pub text_pt: f64,
    pub over_lines: i64,
    pub line_chars: i64,
}

/// Where a build's time went (milliseconds).
#[derive(Clone, Debug, Default, Serialize)]
pub struct Timings {
    pub thumbnail_ms: f64,
    pub measure_passes: usize,
    pub measure_ms: f64,
    pub typeset_ms: f64,
    pub pdf_ms: f64,
}

#[derive(Clone, Debug)]
pub struct BuildResult {
    /// Page 1 as PNG bytes, when asked for with [`Builder::set_thumbnail`].
    pub thumbnail: Option<Vec<u8>>,
    pub timings: Timings,
    pub pages: usize,
    pub notes: Vec<String>,
    pub overflows: Vec<Overflow>,
    pub fit: Option<Fit>,
    pub last_page_fill: f64,
}

// ------------------------------------------------------------------ data for the template

#[derive(Serialize)]
struct Geometry {
    width: f64,
    height: f64,
    margins: Margins,
}

impl Serialize for Margins {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut m = s.serialize_map(Some(4))?;
        m.serialize_entry("top", &self.top)?;
        m.serialize_entry("right", &self.right)?;
        m.serialize_entry("bottom", &self.bottom)?;
        m.serialize_entry("left", &self.left)?;
        m.end()
    }
}

#[derive(Serialize)]
struct ItemData<'a> {
    idx: usize,
    /// First item of its chunk: no sibling item above it on this page.
    first: bool,
    /// Taller than a page: may break.
    overflow: bool,
    data: &'a Item,
}

#[derive(Serialize)]
struct ChunkData<'a> {
    id: &'a str,
    title: &'a str,
    kind: &'a str,
    continued: bool,
    items: Vec<ItemData<'a>>,
}

#[derive(Serialize)]
struct PageData<'a> {
    header: bool,
    columns: IndexMap<&'a str, Vec<ChunkData<'a>>>,
}

#[derive(Serialize)]
struct DocData<'a> {
    mode: &'static str,
    tightness: f64,
    page: Geometry,
    accent: &'a str,
    title: String,
    /// Typst's `lang` (the primary subtag) and `region` (a two-letter region subtag, if any).
    lang: String,
    region: Option<String>,
    /// The CV's words the templates print themselves ("continued").
    words: &'a CvLocale,
    author: &'a str,
    description: Option<String>,
    metrics: IndexMap<&'a str, [f64; 2]>,
    header: &'a Header,
    columns: Vec<&'a str>,
    repeat_titles: bool,
    pages: Vec<PageData<'a>>,
    /// In letter mode: the letter to draw below the header.
    #[serde(skip_serializing_if = "Option::is_none")]
    letter: Option<LetterView>,
}

// ------------------------------------------------------------------ builder

pub struct Builder {
    pub cv: Cv,
    pub base_dir: PathBuf,
    pub settings: Settings,
    pub notes: Vec<String>,
    pub template: Arc<Template>,
    /// Line spacing: 0 the template's own, 1 the floor (see [`Builder::fit`]).
    pub tightness: f64,
    measures: HashMap<u64, Arc<Measure>>,
    thumbnail_dpi: Option<f64>,
    /// While fitting: take measures between 0 and 1 from the line through the two (see `fit`).
    interpolate: bool,
    timings: Timings,
    pub sections: IndexMap<&'static str, Section>,
    pub header: Header,
    photo: Option<(String, Arc<Vec<u8>>)>,
}

/// `urlparse(url).scheme`: letters, digits, `+-.` before a colon, starting with a letter.
fn url_scheme(url: &str) -> Option<String> {
    let (scheme, _) = url.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic() && chars.all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)))
        .then(|| scheme.to_ascii_lowercase())
}

/// `Path.resolve()`: absolute, symlinks resolved where the path exists, `..` folded otherwise.
fn resolve(path: &Path) -> PathBuf {
    if let Ok(p) = path.canonicalize() {
        return p;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let mut out = PathBuf::new();
    for c in absolute.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn image_ext(media: &str) -> &'static str {
    match media.trim().to_ascii_lowercase().as_str() {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        _ => "jpg",
    }
}

impl Builder {
    pub fn new(cv: Cv, base_dir: &Path, options: &Options) -> Result<Builder, RenderError> {
        let (settings, notes) = resolve_settings(&cv, options)?;
        let template = settings.template.clone();
        if template.typst.is_none() {
            return Err(RenderError(format!(
                "template {} has no cv.typ (not ported to Typst yet)",
                pyfmt::repr(&template.name)
            )));
        }
        let mut sections = build_sections(
            &cv,
            &settings.titles,
            &settings.header.profiles,
            &settings.wording,
        );
        if template.config.split_entries {
            sections = sections
                .into_iter()
                .map(|(id, s)| (id, split_entries(s)))
                .collect();
        }
        let mut builder = Builder {
            header: build_header(&cv, &settings.header, None),
            cv,
            base_dir: base_dir.to_path_buf(),
            settings,
            notes,
            template,
            tightness: 0.0,
            measures: HashMap::new(),
            thumbnail_dpi: None,
            interpolate: false,
            timings: Timings::default(),
            sections,
            photo: None,
        };
        builder.photo = builder.load_photo();
        builder.header.photo = builder.photo.as_ref().map(|(path, _)| path.clone());
        Ok(builder)
    }

    /// The photo's bytes when it is a `data:` URL or a file inside the CV's folder; a note
    /// otherwise, as the original gives.
    fn load_photo(&mut self) -> Option<(String, Arc<Vec<u8>>)> {
        let image = self.cv.basics.image.clone()?;
        if !self.settings.header.photo {
            return None;
        }
        match url_scheme(&image).as_deref() {
            Some("data") => {
                let rest = &image[5..];
                let (meta, payload) = rest.split_once(',')?;
                let media = meta.split(';').next().unwrap_or("");
                let bytes = if meta.ends_with(";base64") {
                    base64::engine::general_purpose::STANDARD
                        .decode(payload.trim())
                        .ok()?
                } else {
                    payload.as_bytes().to_vec()
                };
                return Some((engine::photo_path(image_ext(media)), Arc::new(bytes)));
            }
            Some(_) => {
                self.notes.push(format!(
                    "photo {} is not a file in the CV's folder; rendering without it",
                    pyfmt::repr(&image)
                ));
                return None;
            }
            None => {}
        }
        let base = resolve(&self.base_dir);
        let path = resolve(&base.join(&image));
        if !path.starts_with(&base) || path == base {
            self.notes.push(format!(
                "photo {} is outside the CV's folder; rendering without it",
                pyfmt::repr(&image)
            ));
            return None;
        }
        if !path.is_file() {
            let parent = path
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            self.notes.push(format!(
                "photo {} not found (looked in {parent}); rendering without it",
                pyfmt::repr(&image)
            ));
            return None;
        }
        let bytes = std::fs::read(&path).ok()?;
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let ext = if ext == "jpeg" || ext.is_empty() {
            "jpg".to_string()
        } else {
            ext
        };
        Some((engine::photo_path(&ext), Arc::new(bytes)))
    }

    // -------------------------------------------------------------- typesetting

    fn data<'a>(
        &'a self,
        mode: &'static str,
        pages: &'a [Page],
        overflows: &[Overflow],
    ) -> DocData<'a> {
        let (width, height) = page_size_pt(&self.settings.page_size);
        let pages = pages
            .iter()
            .map(|page| PageData {
                header: page.header,
                columns: page
                    .columns
                    .iter()
                    .map(|(col, chunks)| {
                        let chunks = chunks
                            .iter()
                            .filter_map(|chunk| {
                                let section = self.sections.get(chunk.id)?;
                                let items = chunk
                                    .items
                                    .iter()
                                    .enumerate()
                                    .map(|(n, &idx)| ItemData {
                                        idx,
                                        first: n == 0,
                                        overflow: overflows.iter().any(|o| {
                                            o.unit.section == chunk.id && o.unit.index == idx
                                        }),
                                        data: &section.items[idx],
                                    })
                                    .collect();
                                Some(ChunkData {
                                    id: section.id,
                                    title: &section.title,
                                    kind: section.kind,
                                    continued: chunk.continued,
                                    items,
                                })
                            })
                            .collect();
                        (col.as_str(), chunks)
                    })
                    .collect(),
            })
            .collect();
        DocData {
            mode,
            tightness: self.tightness,
            page: Geometry {
                width,
                height,
                margins: self.template.config.margins,
            },
            accent: &self.settings.accent,
            title: self.settings.document_title.clone(),
            lang: self.settings.wording.base(),
            region: self.settings.wording.region(),
            words: &self.settings.wording,
            author: &self.cv.basics.name,
            description: self.cv.basics.label.clone(),
            metrics: Engine::get()
                .metrics
                .iter()
                .map(|(f, a, d)| (f.as_str(), [*a, *d]))
                .collect(),
            header: &self.header,
            columns: self.settings.columns.keys().map(String::as_str).collect(),
            repeat_titles: self.template.config.repeat_titles,
            pages,
            letter: None,
        }
    }

    fn compile(&self, data: &DocData<'_>) -> Result<PagedDocument, RenderError> {
        let bytes = serde_json::to_vec(data).map_err(|e| RenderError(e.to_string()))?;
        let job = Job::new(&self.template, bytes, self.photo.clone());
        typst::compile::<PagedDocument>(&job)
            .output
            .map_err(|errors| {
                // Where in the template, for whoever writes one: `cv.typ:12: message`.
                let messages: Vec<String> = errors
                    .iter()
                    .map(|e| {
                        let place = e.span.id().and_then(|id| {
                            let source = typst::World::source(&job, id).ok()?;
                            let at = typst::WorldExt::range(&job, e.span)?.start;
                            let line = source.lines().byte_to_line(at)? + 1;
                            let file = id.vpath().get_without_slash().to_string();
                            Some(format!("{file}:{line}: "))
                        });
                        format!("{}{}", place.unwrap_or_default(), e.message)
                    })
                    .collect();
                RenderError(format!(
                    "template {}: {}",
                    pyfmt::repr(&self.template.name),
                    messages.join("; ")
                ))
            })
    }

    fn single_page(&self) -> Page {
        let columns = self
            .settings
            .columns
            .iter()
            .map(|(col, ids)| {
                let chunks = ids
                    .iter()
                    .filter_map(|sid| self.sections.get(sid.as_str()))
                    .map(|s| Chunk {
                        id: s.id,
                        continued: false,
                        items: (0..s.items.len()).collect(),
                    })
                    .collect();
                (col.clone(), chunks)
            })
            .collect();
        Page {
            header: true,
            columns,
        }
    }

    /// Lay every column out at full length and read each item's height (px).
    pub fn measure(&mut self) -> Result<Arc<Measure>, RenderError> {
        let key = self.tightness.to_bits();
        if let Some(m) = self.measures.get(&key) {
            return Ok(m.clone());
        }
        if self.interpolate
            && let (Some(loose), Some(tight)) = (
                self.measures.get(&0f64.to_bits()),
                self.measures.get(&1f64.to_bits()),
            )
        {
            return Ok(Arc::new(Measure::between(loose, tight, self.tightness)));
        }
        self.measure_exact()
    }

    /// A measure pass at the current tightness, never interpolated.
    fn measure_exact(&mut self) -> Result<Arc<Measure>, RenderError> {
        let key = self.tightness.to_bits();
        if let Some(m) = self.measures.get(&key) {
            return Ok(m.clone());
        }
        let started = std::time::Instant::now();
        let doc = self.compile(&self.data("measure", &[self.single_page()], &[]))?;
        let measure = Arc::new(self.read_measure(&doc)?);
        self.timings.measure_passes += 1;
        self.timings.measure_ms += started.elapsed().as_secs_f64() * 1e3;
        self.measures.insert(key, measure.clone());
        Ok(measure)
    }

    fn read_measure(&self, doc: &PagedDocument) -> Result<Measure, RenderError> {
        let bad = |what: &str| {
            RenderError(format!(
                "template {} measured no {what}",
                pyfmt::repr(&self.template.name)
            ))
        };
        let introspector = doc.introspector();
        // INFALLIBLE: the labels are non-empty constants.
        let root_label = Label::new(PicoStr::intern("cvforge-measure")).expect("non-empty label");
        let mark_label = Label::new(PicoStr::intern("cvforge-m")).expect("non-empty label");
        let content = introspector
            .query_label(root_label)
            .map_err(|e| RenderError(e.to_string()))?;
        let Some(Value::Dict(root)) = content.to_packed::<MetadataElem>().map(|m| &m.value) else {
            return Err(bad("text metrics"));
        };
        let num = |d: &Dict, k: &str| -> Result<f64, RenderError> {
            match d.get(k) {
                Ok(Value::Float(f)) => Ok(*f),
                Ok(Value::Int(i)) => Ok(*i as f64),
                _ => Err(bad(k)),
            }
        };

        // The markers in document order: (kind, y in pt).
        let mut marks: Vec<(char, f64)> = Vec::new();
        for elem in introspector.query(&Selector::Label(mark_label)).iter() {
            let (Some(meta), Some(loc)) = (elem.to_packed::<MetadataElem>(), elem.location())
            else {
                continue;
            };
            let kind = match &meta.value {
                Value::Array(a) => match a.iter().next() {
                    Some(Value::Str(k)) => k.chars().next().unwrap_or('?'),
                    _ => '?',
                },
                _ => '?',
            };
            let y = introspector
                .position(loc)
                .map(|p| p.point.y.to_pt())
                .ok_or_else(|| bad("marker position"))?;
            marks.push((kind, y));
        }
        let mut marks = marks.into_iter();
        let columns_top = match marks.next() {
            Some(('c', y)) => y,
            _ => return Err(bad("columns")),
        };
        let mut units = IndexMap::new();
        let mut title_heights = HashMap::new();
        for (col, ids) in &self.settings.columns {
            let mut list = Vec::new();
            for sid in ids {
                let Some(section) = self.sections.get(sid.as_str()) else {
                    continue;
                };
                let Some(('s', top)) = marks.next() else {
                    return Err(bad("section"));
                };
                for index in 0..section.items.len() {
                    let (Some(('i', start)), Some(('j', end))) = (marks.next(), marks.next())
                    else {
                        return Err(bad("item"));
                    };
                    if index == 0 {
                        title_heights.insert(section.id, (start - top) * PT_TO_PX);
                    }
                    let top = if index == 0 { top } else { start };
                    list.push(Unit {
                        section: section.id,
                        index,
                        top: (top - columns_top) * PT_TO_PX,
                        bottom: (end - columns_top) * PT_TO_PX,
                    });
                }
            }
            units.insert(col.clone(), list);
        }
        Ok(Measure {
            units,
            title_heights,
            header_height: columns_top * PT_TO_PX,
            text_px: num(root, "text")? * PT_TO_PX,
            line_px: num(root, "line")? * PT_TO_PX,
            line_width_px: num(root, "width")? * PT_TO_PX,
        })
    }

    fn paginate_units(&mut self) -> Result<Pagination, RenderError> {
        let measure = self.measure()?;
        let m = self.template.config.margins;
        let content_height =
            (page_size_pt(&self.settings.page_size).1 - m.top - m.bottom) * PT_TO_PX;
        let title_heights: HashMap<&'static str, f64> = if self.template.config.repeat_titles {
            measure.title_heights.clone()
        } else {
            measure.title_heights.keys().map(|k| (*k, 0.0)).collect()
        };
        let mut columns = IndexMap::new();
        let mut overflows = Vec::new();
        for (col, units) in &measure.units {
            let (pages, over) = paginate_column(
                col,
                units,
                &title_heights,
                content_height - measure.header_height,
                content_height,
                self.template.config.fit_tolerance,
            );
            columns.insert(col.clone(), pages);
            overflows.extend(over);
        }
        Ok(Pagination {
            columns,
            overflows,
            title_heights,
            header_height: measure.header_height,
            content_height,
        })
    }

    pub fn paginate(&mut self) -> Result<(Vec<Page>, Vec<Overflow>), RenderError> {
        let laid_out = self.paginate_units()?;
        let mut pages = Vec::with_capacity(laid_out.pages());
        for n in 0..laid_out.pages() {
            let mut page = Page {
                header: n == 0,
                columns: IndexMap::new(),
            };
            for col in self.settings.columns.keys() {
                let mut chunks: Vec<Chunk> = Vec::new();
                if let Some(units) = laid_out.columns.get(col).and_then(|p| p.get(n)) {
                    for unit in units {
                        match chunks.last_mut() {
                            Some(chunk) if chunk.id == unit.section => chunk.items.push(unit.index),
                            _ => chunks.push(Chunk {
                                id: unit.section,
                                continued: !unit.opens_section(),
                                items: vec![unit.index],
                            }),
                        }
                    }
                }
                page.columns.insert(col.clone(), chunks);
            }
            pages.push(page);
        }
        Ok((pages, laid_out.overflows))
    }

    /// Whether the CV fits `limit` pages at `tightness`. An item taller than a page never fits.
    fn fits_at(&mut self, tightness: f64, limit: usize) -> Result<bool, RenderError> {
        self.tightness = tightness;
        let laid_out = self.paginate_units()?;
        Ok(laid_out.overflows.is_empty() && laid_out.pages() <= limit)
    }

    /// The original's search: halve the interval between too loose (0) and fitting (1)
    /// `FIT_STEPS` times. The tightest spacing that fits, and the last point tried.
    fn binary_search(&mut self, pages: usize) -> Result<(f64, f64), RenderError> {
        let (mut too_loose, mut fits, mut mid) = (0.0, 1.0, 1.0);
        for _ in 0..FIT_STEPS {
            mid = (too_loose + fits) / 2.0;
            if self.fits_at(mid, pages)? {
                fits = mid;
            } else {
                too_loose = mid;
            }
        }
        Ok((fits, mid))
    }

    fn line_height(&mut self) -> Result<f64, RenderError> {
        let m = self.measure()?;
        Ok(pyfmt::round(m.line_px / m.text_px, 3))
    }

    /// The fewest pages this content can take under any limit: at the tightest spacing.
    pub fn fewest_pages(&mut self) -> Result<usize, RenderError> {
        self.tightness = 1.0;
        let pages = self.paginate_units()?.pages();
        self.tightness = 0.0;
        Ok(pages)
    }

    /// Pick the line spacing for the page limit: the template's own when the CV fits as it is,
    /// else the least tightening that fits, down to [`LINE_HEIGHT_FLOOR`].
    pub fn fit(&mut self) -> Result<Option<Fit>, RenderError> {
        let Some(limit) = self.settings.max_pages.filter(|&n| n > 0) else {
            return Ok(None);
        };
        let pages = limit as usize;
        self.tightness = 0.0;
        let designed = self.line_height()?;
        let text_pt = pyfmt::round(self.measure()?.text_px / PT_TO_PX, 2);
        let fit = |fits, tightness, line_height| Fit {
            max_pages: limit,
            fits,
            tightness,
            line_height,
            designed_line_height: designed,
            text_pt,
            over_lines: 0,
            line_chars: 0,
        };
        if self.fits_at(0.0, pages)? {
            return Ok(Some(fit(true, 0.0, designed)));
        }
        if self.fits_at(1.0, pages)? {
            // The original's binary search, on measures interpolated between the passes at 0
            // and 1 (3 passes in all instead of up to 7). One real pass inside the range checks
            // the interpolation: positions are convex in the tightness, so agreeing with the
            // line at one inner point means agreeing everywhere. Otherwise: real passes.
            self.interpolate = true;
            let (mut fits, last_mid) = self.binary_search(pages)?;
            self.interpolate = false;
            let probe = if fits < 1.0 { fits } else { last_mid };
            self.tightness = probe;
            let predicted = Measure::between(
                &self.measures[&0f64.to_bits()],
                &self.measures[&1f64.to_bits()],
                probe,
            );
            if !self.measure_exact()?.same_as(&predicted) {
                fits = self.binary_search(pages)?.0;
            }
            self.tightness = fits;
            let line_height = self.line_height()?;
            return Ok(Some(fit(true, pyfmt::round(fits, 4), line_height)));
        }
        // It does not fit: say how much has to go at the tightest, and draw it as designed.
        self.tightness = 1.0;
        let measure = self.measure()?;
        let beyond = self.paginate_units()?.beyond(pages);
        self.tightness = 0.0;
        Ok(Some(Fit {
            over_lines: ((beyond / measure.line_px).ceil() as i64).max(1),
            line_chars: pyfmt::round_int(measure.line_width_px / (measure.text_px * 0.5)),
            ..fit(false, 0.0, designed)
        }))
    }

    // -------------------------------------------------------------- output

    /// Also draw page 1 as a PNG at `dpi` (auto-apply's thumbnail), straight from the layout:
    /// no PDF to parse. The image has the size pypdfium2 gives (`ceil(points × dpi / 72)`).
    pub fn set_thumbnail(&mut self, dpi: f64) {
        self.thumbnail_dpi = Some(dpi);
    }

    /// Typeset the CV and write the PDF. `first_pages` draws only that many pages (a preview):
    /// the layout is worked out for the whole CV. `debug_target` also writes the data the
    /// template got (the original's `--html`).
    pub fn write_pdf(
        &mut self,
        target: &Path,
        debug_target: Option<&Path>,
        first_pages: Option<usize>,
    ) -> Result<BuildResult, RenderError> {
        let (pdf, result) = self.pdf(debug_target, first_pages)?;
        std::fs::write(target, pdf).map_err(|e| {
            RenderError(format!(
                "{}: {}",
                target.display(),
                crate::model::io_message(&e)
            ))
        })?;
        Ok(result)
    }

    fn write_pdf_bytes(&mut self, doc: &PagedDocument) -> Result<Vec<u8>, RenderError> {
        let started = std::time::Instant::now();
        let options = typst_pdf::PdfOptions {
            creator: Smart::Custom(Some("cv-forge".into())),
            ..typst_pdf::PdfOptions::default()
        };
        let pdf = typst_pdf::pdf(doc, &options).map_err(|errors| {
            RenderError(
                errors
                    .iter()
                    .map(|e| e.message.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        self.timings.pdf_ms += started.elapsed().as_secs_f64() * 1e3;
        Ok(pdf)
    }

    fn write_debug(path: &Path, data: &DocData<'_>) -> Result<(), RenderError> {
        let text = serde_json::to_string_pretty(data).map_err(|e| RenderError(e.to_string()))?;
        std::fs::write(path, text).map_err(|e| {
            RenderError(format!(
                "{}: {}",
                path.display(),
                crate::model::io_message(&e)
            ))
        })
    }

    /// A cover letter in this CV's design: the header its template draws on page one, then the
    /// letter, in the template's fonts, colours and accent (`letter.py`'s `LetterBuilder`).
    /// `debug_target` also writes the data the template got.
    pub fn letter_pdf(
        &mut self,
        letter: &Letter,
        debug_target: Option<&Path>,
    ) -> Result<(Vec<u8>, LetterResult), RenderError> {
        let doc = self.letter_document(letter, debug_target)?;
        let pdf = self.write_pdf_bytes(&doc)?;
        let pages = doc.pages().len();
        let mut notes = letter_notes(&self.notes);
        if pages > 1 {
            notes.push(format!("the letter takes {pages} pages; a letter has one"));
        }
        crate::engine::after_build();
        Ok((pdf, LetterResult { pages, notes }))
    }

    /// The letter typeset, before it becomes a PDF.
    pub fn letter_document(
        &mut self,
        letter: &Letter,
        debug_target: Option<&Path>,
    ) -> Result<PagedDocument, RenderError> {
        let mut data = self.data("letter", &[], &[]);
        let lang = letter.lang(&self.settings.wording.lang);
        data.title = letter.title(&self.cv.basics.name);
        data.lang = crate::wording::base(lang);
        data.region = crate::wording::region(lang);
        data.letter = Some(letter.view());
        if let Some(path) = debug_target {
            Self::write_debug(path, &data)?;
        }
        let started = std::time::Instant::now();
        let doc = self.compile(&data)?;
        self.timings.typeset_ms += started.elapsed().as_secs_f64() * 1e3;
        Ok(doc)
    }

    /// [`Builder::letter_pdf`], written to `target`.
    pub fn write_letter_pdf(
        &mut self,
        letter: &Letter,
        target: &Path,
        debug_target: Option<&Path>,
    ) -> Result<LetterResult, RenderError> {
        let (pdf, result) = self.letter_pdf(letter, debug_target)?;
        std::fs::write(target, pdf).map_err(|e| {
            RenderError(format!(
                "{}: {}",
                target.display(),
                crate::model::io_message(&e)
            ))
        })?;
        Ok(result)
    }

    /// The PDF's bytes, without writing a file.
    pub fn pdf(
        &mut self,
        debug_target: Option<&Path>,
        first_pages: Option<usize>,
    ) -> Result<(Vec<u8>, BuildResult), RenderError> {
        let fit = self.fit()?;
        let (mut pages, overflows) = self.paginate()?;
        if let Some(n) = first_pages.filter(|&n| n > 0) {
            pages.truncate(n);
        }
        let data = self.data("render", &pages, &overflows);
        if let Some(path) = debug_target {
            Self::write_debug(path, &data)?;
        }
        let started = std::time::Instant::now();
        let doc = self.compile(&data)?;
        self.timings.typeset_ms += started.elapsed().as_secs_f64() * 1e3;
        let pdf = self.write_pdf_bytes(&doc)?;
        let mut notes = self.notes.clone();
        for o in &overflows {
            let index = self
                .sections
                .get(o.unit.section)
                .map_or(o.unit.index, |s| s.source_index(o.unit.index));
            notes.push(format!(
                "{}[{index}] is taller than a page ({:.0}pt > {:.0}pt) and overflows; shorten it",
                o.unit.section,
                o.height / PT_TO_PX,
                o.available / PT_TO_PX
            ));
        }
        if let Some(f) = fit.as_ref().filter(|f| !f.fits) {
            let word = if f.max_pages == 1 { "page" } else { "pages" };
            notes.push(format!(
                "does not fit on {} {word}: about {} lines too long, even with the line spacing at {}",
                f.max_pages,
                f.over_lines,
                pyfmt::fmt_g(LINE_HEIGHT_FLOOR)
            ));
        }
        let thumbnail = match self.thumbnail_dpi {
            Some(dpi) => {
                let started = std::time::Instant::now();
                let png = thumbnail_png(&doc, dpi)?;
                self.timings.thumbnail_ms += started.elapsed().as_secs_f64() * 1e3;
                Some(png)
            }
            None => None,
        };
        let produced = doc.pages().len();
        if produced != pages.len() {
            notes.push(format!(
                "expected {} pages but Typst produced {produced}",
                pages.len()
            ));
        }
        let last_page_fill = self.paginate_units()?.last_page_fill();
        crate::engine::after_build();
        Ok((
            pdf,
            BuildResult {
                thumbnail,
                timings: self.timings.clone(),
                pages: produced,
                notes,
                overflows,
                fit,
                last_page_fill,
            },
        ))
    }
}

/// Page 1 of a typeset document as PNG bytes (8-bit RGB), drawn by typst-render at `dpi`,
/// padded with white to `ceil(points × dpi / 72)` pixels as pypdfium2 sizes its bitmaps.
pub fn thumbnail_png(doc: &PagedDocument, dpi: f64) -> Result<Vec<u8>, RenderError> {
    let page = doc
        .pages()
        .first()
        .ok_or_else(|| RenderError("no page to draw".into()))?;
    let scale = dpi / 72.0;
    let size = page.frame.size();
    let (w, h) = (
        (size.x.to_pt() * scale).ceil(),
        (size.y.to_pt() * scale).ceil(),
    );
    if w < 1.0 || h < 1.0 {
        return Err(RenderError("Crop exceeds page dimensions".into()));
    }
    let options = typst_render::RenderOptions {
        pixel_per_pt: typst::utils::Scalar::new(scale),
        render_bleed: false,
    };
    let pixmap = typst_render::render(page, &options);
    let (w, h) = (w as usize, h as usize);
    let (pw, ph) = (pixmap.width() as usize, pixmap.height() as usize);
    let data = pixmap.data();
    let mut rgb = vec![255u8; w * h * 3];
    for y in 0..h.min(ph) {
        for x in 0..w.min(pw) {
            let (i, o) = ((y * pw + x) * 4, (y * w + x) * 3);
            rgb[o..o + 3].copy_from_slice(&data[i..i + 3]);
        }
    }
    let mut out = Vec::with_capacity(rgb.len() / 4);
    let mut encoder = png::Encoder::new(&mut out, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder
        .write_header()
        .map_err(|e| RenderError(e.to_string()))?;
    writer
        .write_image_data(&rgb)
        .map_err(|e| RenderError(e.to_string()))?;
    writer.finish().map_err(|e| RenderError(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod letter_tests {
    use super::*;
    use crate::model::Cv;
    use crate::template::Template;
    use typst::layout::{Frame, FrameItem};

    const CV: &str = include_str!("../../../examples/android-hmi-architect.json");
    const LETTER: &str = include_str!("../../../examples/letter-de.json");

    fn text(frame: &Frame, out: &mut String) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(g) => text(&g.frame, out),
                FrameItem::Text(t) => {
                    out.push_str(&t.text);
                    out.push(' ');
                }
                _ => {}
            }
        }
    }

    fn examples() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
    }

    fn letter_in(template: &str, letter: &Letter) -> (usize, String, Vec<String>) {
        let cv = Cv::from_json(CV).expect("the example CV is valid");
        let options = Options {
            template: Some(template.into()),
            ..Options::default()
        };
        let mut b = Builder::new(cv, &examples(), &options).expect("builder");
        let doc = b.letter_document(letter, None).expect("typeset");
        let mut words = String::new();
        for page in doc.pages() {
            text(&page.frame, &mut words);
        }
        let (_, result) = b.letter_pdf(letter, None).expect("pdf");
        (doc.pages().len(), words, result.notes)
    }

    /// Every template draws a one-page letter: its header, then the letter's parts in order.
    /// The CV is laid out for another template; notes about its columns say nothing here.
    #[test]
    fn a_letter_in_each_template() {
        let letter = Letter::from_json(LETTER).expect("valid letter");
        for t in Template::available() {
            let (pages, words, notes) = letter_in(&t.name, &letter);
            assert_eq!(pages, 1, "{}", t.name);
            assert!(notes.is_empty(), "{}: {notes:?}", t.name);
            let at = |s: &str| {
                words
                    .find(s)
                    .unwrap_or_else(|| panic!("{}: {s:?} not in {words:?}", t.name))
            };
            let name = words.to_lowercase().find("duval").expect("the name");
            let order = [
                at("Kontor Pay GmbH"),
                at("Bewerbung als Android HMI Architect"),
                at("Sehr geehrte Frau Muster,"),
                at("seit zwölf Jahren"),
                at("Mit freundlichen Grüßen"),
                at("Samuel Duval"),
                at("Anlagen"),
                at("Arbeitszeugnisse"),
            ];
            assert!(name < order[0], "{}: the header comes first", t.name);
            assert!(
                order.windows(2).all(|w| w[0] < w[1]),
                "{}: {order:?}",
                t.name
            );
        }
    }

    #[test]
    fn a_long_letter_says_so() {
        let mut letter = Letter::from_json(LETTER).expect("valid letter");
        letter.paragraphs = vec!["Ein sehr langer Absatz über die Arbeit. ".repeat(60); 4];
        let (pages, _, notes) = letter_in("clear", &letter);
        assert!(pages > 1);
        assert_eq!(
            notes,
            [format!("the letter takes {pages} pages; a letter has one")]
        );
    }
}
