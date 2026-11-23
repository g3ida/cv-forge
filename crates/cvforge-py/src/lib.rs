//! The `cvforge` Python module (`cvforge._cvforge`): what auto-apply calls in place of
//! cv-builder's `Builder`. The CV comes in as JSON text and is validated by cv-forge itself.

use std::path::Path;

use cvforge::{BuildResult, Builder, Cv, Fit, Options};
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

pyo3::create_exception!(
    _cvforge,
    RenderError,
    pyo3::exceptions::PyRuntimeError,
    "The CV could not be built (cv-builder's RenderError)."
);

fn parse(cv_json: &str) -> PyResult<Cv> {
    Cv::from_json(cv_json).map_err(|e| PyValueError::new_err(e.report("cv")))
}

fn options(
    template: Option<String>,
    accent: Option<String>,
    page_size: Option<String>,
    max_pages: Option<i64>,
) -> Options {
    Options {
        template,
        accent,
        page_size,
        max_pages,
    }
}

fn fit_dict<'py>(py: Python<'py>, fit: Option<&Fit>) -> PyResult<Bound<'py, PyAny>> {
    let Some(f) = fit else {
        return Ok(py.None().into_bound(py));
    };
    let d = PyDict::new(py);
    d.set_item("max_pages", f.max_pages)?;
    d.set_item("fits", f.fits)?;
    d.set_item("tightness", f.tightness)?;
    d.set_item("line_height", f.line_height)?;
    d.set_item("designed_line_height", f.designed_line_height)?;
    d.set_item("text_pt", f.text_pt)?;
    d.set_item("over_lines", f.over_lines)?;
    d.set_item("line_chars", f.line_chars)?;
    Ok(d.into_any())
}

fn result_dict<'py>(
    py: Python<'py>,
    r: &BuildResult,
    pdf: Option<Vec<u8>>,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("pages", r.pages)?;
    d.set_item("notes", PyList::new(py, &r.notes)?)?;
    d.set_item("fit", fit_dict(py, r.fit.as_ref())?)?;
    d.set_item("last_page_fill", r.last_page_fill)?;
    d.set_item("overflows", r.overflows.len())?;
    let t = PyDict::new(py);
    t.set_item("measure_passes", r.timings.measure_passes)?;
    t.set_item("measure_ms", r.timings.measure_ms)?;
    t.set_item("typeset_ms", r.timings.typeset_ms)?;
    t.set_item("pdf_ms", r.timings.pdf_ms)?;
    t.set_item("thumbnail_ms", r.timings.thumbnail_ms)?;
    d.set_item("timings", t)?;
    if let Some(png) = &r.thumbnail {
        d.set_item("thumbnail", PyBytes::new(py, png))?;
    }
    if let Some(bytes) = pdf {
        d.set_item("pdf", PyBytes::new(py, &bytes))?;
    }
    Ok(d)
}

/// Build a CV: the PDF written to `pdf` (or returned as bytes), and what fitting did.
#[pyfunction]
#[pyo3(signature = (cv, base_dir, *, template=None, accent=None, page_size=None, max_pages=None, pdf=None, first_pages=None, thumbnail_dpi=None))]
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors cv-builder's options as keyword arguments"
)]
fn build<'py>(
    py: Python<'py>,
    cv: &str,
    base_dir: &str,
    template: Option<String>,
    accent: Option<String>,
    page_size: Option<String>,
    max_pages: Option<i64>,
    pdf: Option<String>,
    first_pages: Option<usize>,
    thumbnail_dpi: Option<f64>,
) -> PyResult<Bound<'py, PyDict>> {
    let cv = parse(cv)?;
    let opts = options(template, accent, page_size, max_pages);
    let base = base_dir.to_string();
    let (bytes, result) = py
        .detach(move || {
            let mut b = Builder::new(cv, Path::new(&base), &opts)?;
            if let Some(dpi) = thumbnail_dpi {
                b.set_thumbnail(dpi);
            }
            b.pdf(None, first_pages)
        })
        .map_err(|e| RenderError::new_err(e.0))?;
    let returned = match pdf {
        Some(path) => {
            std::fs::write(&path, &bytes)
                .map_err(|e| PyOSError::new_err(format!("{path}: {e}")))?;
            None
        }
        None => Some(bytes),
    };
    result_dict(py, &result, returned)
}

/// The fewest pages the CV can take under any page limit (at the tightest line spacing).
#[pyfunction]
#[pyo3(signature = (cv, base_dir, *, template=None))]
fn fewest_pages(
    py: Python<'_>,
    cv: &str,
    base_dir: &str,
    template: Option<String>,
) -> PyResult<usize> {
    let cv = parse(cv)?;
    let base = base_dir.to_string();
    py.detach(move || {
        Builder::new(cv, Path::new(&base), &options(template, None, None, None))?.fewest_pages()
    })
    .map_err(|e| RenderError::new_err(e.0))
}

/// Whether the CV fits `max_pages`, without typesetting the PDF.
#[pyfunction]
#[pyo3(signature = (cv, base_dir, *, template=None, max_pages))]
fn fit<'py>(
    py: Python<'py>,
    cv: &str,
    base_dir: &str,
    template: Option<String>,
    max_pages: i64,
) -> PyResult<Bound<'py, PyAny>> {
    let cv = parse(cv)?;
    let base = base_dir.to_string();
    let fit = py
        .detach(move || {
            Builder::new(
                cv,
                Path::new(&base),
                &options(template, None, None, Some(max_pages)),
            )?
            .fit()
        })
        .map_err(|e| RenderError::new_err(e.0))?;
    fit_dict(py, fit.as_ref())
}

/// The CV's validation problems (`location: message`), as cv-builder reports them; empty when valid.
#[pyfunction]
fn problems(cv: &str) -> Vec<String> {
    match Cv::from_json(cv) {
        Ok(_) => Vec::new(),
        Err(cvforge::LoadError::Invalid(ps)) => ps
            .iter()
            .map(|p| format!("{}: {}", p.location(), p.msg))
            .collect(),
        Err(e) => vec![e.report("cv")],
    }
}

#[pymodule]
fn _cvforge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add(
        "ENGINE",
        concat!("cv-forge ", env!("CARGO_PKG_VERSION"), " (Typst 0.15)"),
    )?;
    // The languages a CV may be in (layout.language's primary subtag) for cv-forge to print it as
    // cv-builder does: English, French and German in full, then every one CLDR has months for.
    m.add("LANGUAGES", cvforge::wording::languages())?;
    // The bundled templates (their names, sorted); career-ledger draws any other with cv-builder.
    m.add(
        "TEMPLATES",
        cvforge::template::Template::available()
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>(),
    )?;
    m.add("RenderError", m.py().get_type::<RenderError>())?;
    m.add_function(wrap_pyfunction!(build, m)?)?;
    m.add_function(wrap_pyfunction!(fewest_pages, m)?)?;
    m.add_function(wrap_pyfunction!(fit, m)?)?;
    m.add_function(wrap_pyfunction!(problems, m)?)?;
    Ok(())
}
