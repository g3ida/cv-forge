//! Python bindings for `src/cv_builder/`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use pyo3::prelude::*;

pub mod js;
pub mod port;
pub mod pyconv;
pub mod pynum;

#[allow(unused_imports)]
use pynum::Num;

fn error(message: String) -> PyErr {
    use pyo3::exceptions as e;
    let (kind, text) = message.split_once(": ").unwrap_or(("", message.as_str()));
    let text = text.to_string();
    match kind {
        "TypeError" => e::PyTypeError::new_err(text),
        "KeyError" => e::PyKeyError::new_err(text),
        "IndexError" => e::PyIndexError::new_err(text),
        "OverflowError" => e::PyOverflowError::new_err(text),
        "ZeroDivisionError" => e::PyZeroDivisionError::new_err(text),
        "AttributeError" => e::PyAttributeError::new_err(text),
        "RuntimeError" => e::PyRuntimeError::new_err(text),
        "NotImplementedError" => e::PyNotImplementedError::new_err(text),
        "LookupError" => e::PyLookupError::new_err(text),
        "ArithmeticError" => e::PyArithmeticError::new_err(text),
        "UnicodeError" => e::PyUnicodeError::new_err(text),
        "ValueError" => e::PyValueError::new_err(text),
        _ => e::PyValueError::new_err(message),
    }
}

fn format_date_inner(value: Option<String>) -> Result<String, String> {
    let out = port::format_date(value)?;
    Ok(out)
}

/// `format_date`.
#[pyfunction]
#[pyo3(name = "format_date", signature = (value))]
fn format_date(value: Option<String>) -> PyResult<String> {
    format_date_inner(value).map_err(error)
}

fn format_range_inner(
    start: Option<String>,
    end: Option<String>,
    ongoing_if_no_end: bool,
    present: String,
    separator: String,
) -> Result<String, String> {
    let out = port::format_range(start, end, ongoing_if_no_end, present, separator)?;
    Ok(out)
}

/// `format_range`.
#[pyfunction]
#[pyo3(name = "format_range", signature = (start, end, *, ongoing_if_no_end=true, present=String::from("Present"), separator=String::from(" - ")))]
fn format_range(
    start: Option<String>,
    end: Option<String>,
    ongoing_if_no_end: bool,
    present: String,
    separator: String,
) -> PyResult<String> {
    format_range_inner(start, end, ongoing_if_no_end, present, separator).map_err(error)
}

fn network_icon_inner(network: String) -> Result<String, String> {
    let out = port::network_icon(network)?;
    Ok(out)
}

/// `network_icon`.
#[pyfunction]
#[pyo3(name = "network_icon", signature = (network))]
fn network_icon(network: String) -> PyResult<String> {
    network_icon_inner(network).map_err(error)
}

fn keep_inner(text: Option<String>) -> Result<Option<serde_json::Value>, String> {
    let out = port::keep(text)?;
    Ok(out)
}

/// `keep`.
#[pyfunction]
#[pyo3(name = "keep", signature = (text))]
fn keep(py: Python<'_>, text: Option<String>) -> PyResult<Py<PyAny>> {
    keep_inner(text)
        .map_err(error)
        .and_then(|v| crate::pyconv::ToPy::to_py(&v, py))
}

fn inline_inner(text: Option<String>) -> Result<serde_json::Value, String> {
    let out = port::inline(text)?;
    Ok(out)
}

/// `inline`.
#[pyfunction]
#[pyo3(name = "inline", signature = (text))]
fn inline(py: Python<'_>, text: Option<String>) -> PyResult<Py<PyAny>> {
    inline_inner(text)
        .map_err(error)
        .and_then(|v| crate::pyconv::ToPy::to_py(&v, py))
}

fn paragraphs_inner(text: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let out = port::paragraphs(text)?;
    Ok(out)
}

/// `paragraphs`.
#[pyfunction]
#[pyo3(name = "paragraphs", signature = (text))]
fn paragraphs(py: Python<'_>, text: Option<String>) -> PyResult<Py<PyAny>> {
    paragraphs_inner(text)
        .map_err(error)
        .and_then(|v| crate::pyconv::ToPy::to_py(&v, py))
}

fn pretty_url_inner(url: String) -> Result<String, String> {
    let out = port::pretty_url(url)?;
    Ok(out)
}

/// `pretty_url`.
#[pyfunction]
#[pyo3(name = "pretty_url", signature = (url))]
fn pretty_url(url: String) -> PyResult<String> {
    pretty_url_inner(url).map_err(error)
}

fn href_inner(url: Option<String>) -> Result<Option<String>, String> {
    let out = port::href(url)?;
    Ok(out)
}

/// `href`.
#[pyfunction]
#[pyo3(name = "href", signature = (url))]
fn href(url: Option<String>) -> PyResult<Option<String>> {
    href_inner(url).map_err(error)
}

fn language_rating_inner(fluency: Option<String>, rating: Option<i64>) -> Result<i64, String> {
    let out = port::language_rating(fluency, rating)?;
    Ok(out)
}

/// `language_rating`.
#[pyfunction]
#[pyo3(name = "language_rating", signature = (fluency, rating))]
fn language_rating(fluency: Option<String>, rating: Option<i64>) -> PyResult<i64> {
    language_rating_inner(fluency, rating).map_err(error)
}

#[pymodule]
#[pyo3(name = "_native")]
fn native_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(format_date, m)?)?;
    m.add_function(wrap_pyfunction!(format_range, m)?)?;
    m.add_function(wrap_pyfunction!(network_icon, m)?)?;
    m.add_function(wrap_pyfunction!(keep, m)?)?;
    m.add_function(wrap_pyfunction!(inline, m)?)?;
    m.add_function(wrap_pyfunction!(paragraphs, m)?)?;
    m.add_function(wrap_pyfunction!(pretty_url, m)?)?;
    m.add_function(wrap_pyfunction!(href, m)?)?;
    m.add_function(wrap_pyfunction!(language_rating, m)?)?;
    Ok(())
}
