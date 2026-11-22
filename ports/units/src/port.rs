//! The Rust port of `src/cv_builder/`: each function is the one cv-forge ships, called with the
//! original's arguments, so the recorded cases check the code the builder runs.
//!
//! `keep`, `inline` and `paragraphs` return runs of text in cv-forge (the Typst templates draw
//! them); here they are turned back into the original's markup with `text::to_html`.

#![forbid(unsafe_code)]

use cvforge::{dates, text, view};

/// Port of `format_date` (`src/cv_builder/dates.py:8`).
pub fn format_date(value: Option<String>) -> Result<String, String> {
    dates::format_date(value.as_deref(), &dates::MONTHS, false)
}

/// Port of `format_range` (`src/cv_builder/dates.py:18`).
pub fn format_range(
    start: Option<String>,
    end: Option<String>,
    ongoing_if_no_end: bool,
    present: String,
    separator: String,
) -> Result<String, String> {
    dates::format_range(
        start.as_deref(),
        end.as_deref(),
        ongoing_if_no_end,
        &present,
        &separator,
        &dates::MONTHS,
        false,
    )
}

/// Port of `network_icon` (`src/cv_builder/icons.py:42`).
pub fn network_icon(network: String) -> Result<String, String> {
    Ok(view::network_icon(&network).to_string())
}

/// Port of `keep` (`src/cv_builder/view.py:48`).
pub fn keep(text: Option<String>) -> Result<Option<serde_json::Value>, String> {
    Ok(text::keep(text.as_deref()).map(|r| serde_json::Value::String(text::to_html(&r))))
}

/// Port of `inline` (`src/cv_builder/view.py:55`).
pub fn inline(text: Option<String>) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::String(text::to_html(&text::inline(
        text.as_deref(),
    ))))
}

/// Port of `paragraphs` (`src/cv_builder/view.py:65`).
pub fn paragraphs(text: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    Ok(text::paragraphs(text.as_deref())
        .iter()
        .map(|p| serde_json::Value::String(text::to_html(p)))
        .collect())
}

/// Port of `pretty_url` (`src/cv_builder/view.py:71`).
pub fn pretty_url(url: String) -> Result<String, String> {
    Ok(text::pretty_url(&url))
}

/// Port of `href` (`src/cv_builder/view.py:75`).
pub fn href(url: Option<String>) -> Result<Option<String>, String> {
    Ok(text::href(url.as_deref()))
}

/// Port of `language_rating` (`src/cv_builder/view.py:267`).
pub fn language_rating(fluency: Option<String>, rating: Option<i64>) -> Result<i64, String> {
    Ok(text::language_rating(fluency.as_deref(), rating))
}
