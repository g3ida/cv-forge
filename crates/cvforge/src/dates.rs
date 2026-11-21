//! Display formatting for the partial ISO dates of the CV format (port of `dates.py`).

use crate::pyfmt;

pub const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `2019` -> `2019`; `2019-07` and `2019-07-15` -> `Jul 2019`. Errors as Python raises them
/// (`ValueError`, `IndexError`) for months a validated CV never has.
pub fn format_date(value: Option<&str>) -> Result<String, String> {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return Ok(String::new());
    };
    let mut parts = value.split('-');
    let year = parts.next().unwrap_or("");
    let Some(month) = parts.next() else {
        return Ok(year.to_string());
    };
    let index = pyfmt::int(month)? - 1;
    // Python indexes a tuple from its end for -12..=-1.
    let index = if index < 0 { index + 12 } else { index };
    match usize::try_from(index).ok().and_then(|i| MONTHS.get(i)) {
        Some(name) => Ok(format!("{name} {year}")),
        None => Err("IndexError: tuple index out of range".into()),
    }
}

/// A start/end pair. A missing end means "ongoing" (JSON Resume) unless `ongoing_if_no_end` is
/// false; an end equal to the start collapses to one date.
pub fn format_range(
    start: Option<&str>,
    end: Option<&str>,
    ongoing_if_no_end: bool,
    present: &str,
    separator: &str,
) -> Result<String, String> {
    let first = format_date(start)?;
    let started = start.is_some_and(|s| !s.is_empty());
    let last = if end == Some("present") || (end.is_none() && started && ongoing_if_no_end) {
        present.to_string()
    } else {
        format_date(end)?
    };
    if first.is_empty() {
        return Ok(if last != present { last } else { String::new() });
    }
    if last.is_empty() || last == first {
        return Ok(first);
    }
    Ok(format!("{first}{separator}{last}"))
}

/// `format_range` with the defaults every caller uses.
pub fn range(start: Option<&str>, end: Option<&str>) -> String {
    // INFALLIBLE: validated CV dates always have a month in 01..=12.
    format_range(start, end, true, "Present", " - ").unwrap_or_default()
}

/// `format_date` for validated dates.
pub fn date(value: Option<&str>) -> String {
    // INFALLIBLE: validated CV dates always have a month in 01..=12.
    format_date(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(format_date(Some("2019")).as_deref(), Ok("2019"));
        assert_eq!(format_date(Some("2019-07-15")).as_deref(), Ok("Jul 2019"));
        assert_eq!(format_date(Some("2019-00")).as_deref(), Ok("Dec 2019"));
        assert!(format_date(Some("2019-13")).is_err());
        assert_eq!(range(Some("2019-07"), None), "Jul 2019 - Present");
        assert_eq!(range(Some("2019-07"), Some("2019-07")), "Jul 2019");
        assert_eq!(range(None, Some("present")), "");
        assert_eq!(range(None, Some("2020")), "2020");
        assert_eq!(range(None, None), "");
    }
}
