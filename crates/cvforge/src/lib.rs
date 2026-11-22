//! cv-forge: PDF CVs from JSON Resume files, fast. A Rust port of cv-builder (Python +
//! WeasyPrint) with Typst as the typesetter.
//!
//! ```text
//! cv.json ──validate──▶ Cv ──view──▶ sections ──measure (Typst)──▶ paginate ──typeset──▶ PDF
//! ```
//!
//! The pipeline, names and messages follow the original module by module (see `GLOSSARY.md`):
//! [`model`] (`model.py`), [`view`] and [`text`] (`view.py`), [`dates`], [`paginate`],
//! [`build`] (`render.py`), [`template`] and [`wording`]. [`engine`] is the Typst world.

pub mod build;
pub mod dates;
pub mod engine;
pub mod model;
#[rustfmt::skip]
pub mod months;
pub mod paginate;
pub mod pyfmt;
pub mod pyjson;
pub mod template;
pub mod text;
pub mod view;
pub mod wording;

pub use build::{BuildResult, Builder, Fit, Options, RenderError};
pub use model::{Cv, LoadError, load_cv};
pub use template::Template;
