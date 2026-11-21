//! Split measured column content into pages (port of `paginate.py`).
//!
//! Each column flows from page to page on its own. Items are never split; a section that
//! continues on a new page repeats its title (a template that does not repeat titles passes a
//! title height of zero).

use std::collections::HashMap;

/// px of slack for float rounding (the default; a template may set its own).
pub const TOLERANCE: f64 = 0.5;

/// One item measured in a continuous single-page layout of its column (CSS px).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit {
    pub section: &'static str,
    pub index: usize,
    /// Border-box top; for an item opening its section, the section's top.
    pub top: f64,
    pub bottom: f64,
}

impl Unit {
    pub fn opens_section(&self) -> bool {
        self.index == 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Overflow {
    pub column: String,
    pub unit: Unit,
    pub height: f64,
    pub available: f64,
}

/// Greedily fill pages; the units per page and the items too tall for a page.
///
/// `tolerance` is how far (px) a page's content may exceed the page. An overfilled page, however
/// little, makes the typesetter move what sticks out to an extra page; pass zero or less to rule
/// that out.
pub fn paginate_column(
    column: &str,
    units: &[Unit],
    title_heights: &HashMap<&'static str, f64>,
    first_page: f64,
    other_pages: f64,
    tolerance: f64,
) -> (Vec<Vec<Unit>>, Vec<Overflow>) {
    let mut pages: Vec<Vec<Unit>> = Vec::new();
    let mut overflows = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let available = if pages.is_empty() {
            first_page
        } else {
            other_pages
        };
        let start = units[i];
        let repeat_title = if start.opens_section() {
            0.0
        } else {
            title_heights.get(start.section).copied().unwrap_or(0.0)
        };
        let mut j = i;
        while j < units.len() && units[j].bottom - start.top + repeat_title <= available + tolerance
        {
            j += 1;
        }
        if j == i {
            let height = start.bottom - start.top + repeat_title;
            if pages.is_empty() && height <= other_pages + tolerance {
                pages.push(Vec::new()); // does not fit under the header: start on page 2
                continue;
            }
            overflows.push(Overflow {
                column: column.to_string(),
                unit: start,
                height,
                available,
            });
            j = i + 1;
        }
        pages.push(units[i..j].to_vec());
        i = j;
    }
    (pages, overflows)
}

#[cfg(test)]
mod tests {
    //! The original's tests/test_paginate.py, case for case.
    use super::*;

    /// Units of one section laid out back to back, the first one including the title.
    fn stack(section: &'static str, heights: &[f64], start: f64, title: f64) -> Vec<Unit> {
        let mut y = start;
        heights
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let top = y;
                let bottom = y + if i == 0 { title } else { 0.0 } + h;
                y = bottom;
                Unit {
                    section,
                    index: i,
                    top,
                    bottom,
                }
            })
            .collect()
    }

    fn titles(pairs: &[(&'static str, f64)]) -> HashMap<&'static str, f64> {
        pairs.iter().copied().collect()
    }

    fn indexes(pages: &[Vec<Unit>]) -> Vec<Vec<usize>> {
        pages
            .iter()
            .map(|p| p.iter().map(|u| u.index).collect())
            .collect()
    }

    #[test]
    fn everything_fits_on_one_page() {
        let units = stack("work", &[30.0, 30.0, 30.0], 0.0, 10.0);
        let (pages, overflows) = paginate_column(
            "left",
            &units,
            &titles(&[("work", 10.0)]),
            200.0,
            300.0,
            TOLERANCE,
        );
        assert_eq!(pages.iter().map(Vec::len).collect::<Vec<_>>(), [3]);
        assert!(overflows.is_empty());
    }

    #[test]
    fn items_flow_to_next_page_with_repeated_title() {
        let units = stack("work", &[40.0, 40.0, 40.0, 40.0], 0.0, 10.0);
        let (pages, _) = paginate_column(
            "left",
            &units,
            &titles(&[("work", 10.0)]),
            100.0,
            100.0,
            TOLERANCE,
        );
        assert_eq!(indexes(&pages), [vec![0, 1], vec![2, 3]]);
        assert!(!pages[1][0].opens_section());
    }

    #[test]
    fn repeated_title_height_is_accounted_for() {
        let units = stack("work", &[40.0, 45.0, 45.0], 0.0, 10.0);
        let (pages, _) = paginate_column(
            "left",
            &units,
            &titles(&[("work", 10.0)]),
            50.0,
            100.0,
            TOLERANCE,
        );
        assert_eq!(indexes(&pages), [vec![0], vec![1, 2]]);
    }

    #[test]
    fn first_item_skips_short_first_page() {
        let units = stack("work", &[80.0], 0.0, 10.0);
        let (pages, overflows) = paginate_column(
            "left",
            &units,
            &titles(&[("work", 10.0)]),
            50.0,
            100.0,
            TOLERANCE,
        );
        assert_eq!(pages.iter().map(Vec::len).collect::<Vec<_>>(), [0, 1]);
        assert!(overflows.is_empty());
    }

    #[test]
    fn item_taller_than_a_page_is_reported() {
        let units = stack("work", &[500.0, 20.0], 0.0, 10.0);
        let (pages, overflows) = paginate_column(
            "left",
            &units,
            &titles(&[("work", 10.0)]),
            100.0,
            100.0,
            TOLERANCE,
        );
        assert_eq!(overflows.len(), 1);
        assert_eq!(overflows[0].unit.index, 0);
        assert_eq!(indexes(&pages), [vec![0], vec![1]]);
    }

    #[test]
    fn sections_share_pages() {
        let mut units = stack("summary", &[20.0], 0.0, 10.0);
        units.extend(stack("skills", &[20.0, 20.0], 35.0, 10.0));
        let (pages, _) = paginate_column(
            "right",
            &units,
            &titles(&[("summary", 10.0), ("skills", 10.0)]),
            1000.0,
            1000.0,
            TOLERANCE,
        );
        let got: Vec<(&str, usize)> = pages[0].iter().map(|u| (u.section, u.index)).collect();
        assert_eq!(got, [("summary", 0), ("skills", 0), ("skills", 1)]);
    }

    #[test]
    fn tolerance_decides_whether_a_hair_too_much_still_fits() {
        let units = stack("work", &[40.0, 50.3], 0.0, 10.0);
        let (pages, _) = paginate_column(
            "left",
            &units,
            &titles(&[("work", 10.0)]),
            100.0,
            100.0,
            TOLERANCE,
        );
        assert_eq!(indexes(&pages), [vec![0, 1]]);
        let (pages, _) =
            paginate_column("main", &units, &titles(&[("work", 0.0)]), 100.0, 100.0, 0.0);
        assert_eq!(indexes(&pages), [vec![0], vec![1]]);
        let exact = stack("work", &[40.0, 50.0], 0.0, 10.0);
        let lens = |t: f64| {
            paginate_column("main", &exact, &titles(&[("work", 0.0)]), 100.0, 100.0, t)
                .0
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>()
        };
        assert_eq!(lens(0.0), [2]);
        assert_eq!(lens(-0.1), [1, 1]);
    }
}
