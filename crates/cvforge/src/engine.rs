//! The Typst world a CV is typeset in: the standard library and the bundled fonts, made once
//! per process and shared by every build and thread; per build, the template, the shared
//! helpers, the data as JSON and the photo. Nothing else is readable, so a CV cannot make the
//! typesetter read other files (the original's `LocalFetcher`).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, OnceLock};

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::layout::Abs;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook, FontInstance, FontVariations};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use crate::template::{SHARED_LIB, Template};

macro_rules! fonts {
    ($($file:literal),* $(,)?) => {
        &[$(include_bytes!(concat!("../../../assets/fonts/", $file))),*]
    };
}

/// The original's bundled fonts (SIL Open Font License), embedded.
const FONT_FILES: &[&[u8]] = fonts![
    "Inter-Regular.ttf",
    "Inter-Medium.ttf",
    "Inter-SemiBold.ttf",
    "Inter-Bold.ttf",
    "NotoSerif-Regular.ttf",
    "NotoSerif-Italic.ttf",
    "NotoSerif-Bold.ttf",
    "NotoSerif-BoldItalic.ttf",
    "Rubik-Regular.ttf",
    "Rubik-Medium.ttf",
    "Rubik-Bold.ttf",
];

pub struct Engine {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    instances: Vec<FontInstance>,
    /// Ascender and descender (em, both positive) per family: CSS line boxes in the templates.
    pub metrics: Vec<(String, f64, f64)>,
}

impl Engine {
    /// An upper bound of `text`'s width in em: each character at the widest advance it has in
    /// any bundled font (1.5 em when none has it). Templates draw a no-break box without
    /// measuring it when this bound fits the column (`nowrap` in lib.typ).
    pub fn width_bound(&self, text: &str) -> f64 {
        text.chars()
            .map(|c| {
                let widest = self
                    .instances
                    .iter()
                    .filter_map(|i| {
                        let face = i.ttf();
                        let advance = face.glyph_hor_advance(face.glyph_index(c)?)?;
                        Some(f64::from(advance) / f64::from(face.units_per_em()))
                    })
                    .fold(None, |best: Option<f64>, a| {
                        Some(best.map_or(a, |b| b.max(a)))
                    });
                widest.unwrap_or(1.5)
            })
            .sum()
    }

    /// Whether every bundled font has a glyph for each character of `text` (else Typst would
    /// fall back to another bundled font, or draw nothing, where cv-builder takes a system font).
    pub fn covers(&self, text: &str) -> bool {
        text.chars().filter(|c| !c.is_whitespace()).all(|c| {
            self.instances
                .iter()
                .all(|i| i.ttf().glyph_index(c).is_some())
        })
    }

    /// The process-wide engine.
    pub fn get() -> &'static Engine {
        static ENGINE: OnceLock<Engine> = OnceLock::new();
        ENGINE.get_or_init(|| {
            let fonts: Vec<Font> = FONT_FILES
                .iter()
                .flat_map(|data| Font::iter(Bytes::new(*data)))
                .collect();
            let instances: Vec<FontInstance> = fonts
                .iter()
                .map(|f| {
                    f.clone().instantiate(
                        f.info().variant,
                        Abs::pt(10.0),
                        &FontVariations::default(),
                    )
                })
                .collect();
            let mut metrics: Vec<(String, f64, f64)> = Vec::new();
            for (font, instance) in fonts.iter().zip(&instances) {
                let family = font.info().family.clone();
                if !metrics.iter().any(|(f, _, _)| *f == family) {
                    let m = instance.metrics();
                    metrics.push((family, m.ascender.get(), -m.descender.get()));
                }
            }
            Engine {
                library: LazyHash::new(Library::builder().build()),
                book: LazyHash::new(FontBook::from_fonts(&fonts)),
                fonts,
                instances,
                metrics,
            }
        })
    }
}

fn file_id(path: &str) -> FileId {
    // INFALLIBLE: called with the constant absolute paths below.
    RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(path).expect("valid virtual path"),
    )
    .intern()
}

static MAIN: LazyLock<FileId> = LazyLock::new(|| file_id("/main.typ"));
static LIB: LazyLock<FileId> = LazyLock::new(|| file_id("/lib.typ"));
static DATA: LazyLock<FileId> = LazyLock::new(|| file_id("/data.json"));

/// The world's path for a photo with this extension.
pub fn photo_path(ext: &str) -> String {
    format!("/photo.{ext}")
}

fn shared_lib() -> Source {
    static LIB_SOURCE: OnceLock<Source> = OnceLock::new();
    LIB_SOURCE
        .get_or_init(|| Source::new(*LIB, SHARED_LIB.to_string()))
        .clone()
}

/// One document to typeset.
pub struct Job {
    engine: &'static Engine,
    main: Source,
    lib: Source,
    data: Bytes,
    photo: Option<(FileId, Bytes)>,
}

impl Job {
    pub fn new(template: &Template, data: Vec<u8>, photo: Option<(String, Arc<Vec<u8>>)>) -> Job {
        let main = template
            .source
            .get_or_init(|| Source::new(*MAIN, template.typst.clone().unwrap_or_default()))
            .clone();
        Job {
            engine: Engine::get(),
            main,
            lib: shared_lib(),
            data: Bytes::new(data),
            photo: photo.map(|(path, bytes)| (file_id(&path), Bytes::new(ArcBytes(bytes)))),
        }
    }
}

/// Shared photo bytes, without a copy per build.
struct ArcBytes(Arc<Vec<u8>>);

impl AsRef<[u8]> for ArcBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl World for Job {
    fn library(&self) -> &LazyHash<Library> {
        &self.engine.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.engine.book
    }

    fn main(&self) -> FileId {
        *MAIN
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == *MAIN {
            Ok(self.main.clone())
        } else if id == *LIB {
            Ok(self.lib.clone())
        } else {
            Err(FileError::NotFound(std::path::PathBuf::from(
                id.vpath().get_with_slash(),
            )))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == *DATA {
            return Ok(self.data.clone());
        }
        if let Some((photo, bytes)) = &self.photo
            && id == *photo
        {
            return Ok(bytes.clone());
        }
        if id == *MAIN || id == *LIB {
            return Ok(Bytes::new(self.source(id)?.text().as_bytes().to_vec()));
        }
        Err(FileError::NotFound(std::path::PathBuf::from(
            id.vpath().get_with_slash(),
        )))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.engine.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

/// Called after every build. Long-running callers (the Python module, servers) must not grow
/// Typst's caches forever, but evicting takes a global lock: doing it after every build
/// serializes parallel builds. So every 64th build drops what the last few hundred builds did
/// not use.
pub fn after_build() {
    static BUILDS: AtomicUsize = AtomicUsize::new(0);
    if BUILDS.fetch_add(1, Ordering::Relaxed) % 64 == 63 {
        comemo::evict(4);
    }
}
