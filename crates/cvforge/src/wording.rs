//! The words a CV prints that are not the CV's own: section headings, month names, "Present"
//! (port of `wording.py`).
//!
//! `layout.language` (a BCP-47 tag, `en` by default) picks them. English, French and German have
//! every word; any other language gets its month names from CLDR ([`crate::months`], what Babel
//! gives cv-builder) and English headings, which `layout.titles` replaces. `layout.dateStyle`
//! writes months as names ("Mar 2021") or numbers ("03/2021"); German CVs usually use numbers, so
//! that is its default.

use serde::Serialize;

use crate::dates::MONTHS;
use crate::months;

pub type Titles = [(&'static str, &'static str); 14];

pub const TITLES_EN: Titles = [
    ("summary", "Summary"),
    ("work", "Experience"),
    ("volunteer", "Volunteering"),
    ("education", "Education"),
    ("projects", "Projects"),
    ("skills", "Skills"),
    ("languages", "Languages"),
    ("achievements", "Key Achievements"),
    ("awards", "Awards"),
    ("certificates", "Certifications"),
    ("publications", "Publications"),
    ("interests", "Interests"),
    ("references", "References"),
    ("profiles", "Find Me Online"),
];

pub const TITLES_FR: Titles = [
    ("summary", "Profil"),
    ("work", "Expérience professionnelle"),
    ("volunteer", "Bénévolat"),
    ("education", "Formation"),
    ("projects", "Projets"),
    ("skills", "Compétences"),
    ("languages", "Langues"),
    ("achievements", "Réalisations clés"),
    ("awards", "Distinctions"),
    ("certificates", "Certifications"),
    ("publications", "Publications"),
    ("interests", "Centres d'intérêt"),
    ("references", "Références"),
    ("profiles", "En ligne"),
];

pub const TITLES_DE: Titles = [
    ("summary", "Profil"),
    ("work", "Berufserfahrung"),
    ("volunteer", "Ehrenamt"),
    ("education", "Ausbildung"),
    ("projects", "Projekte"),
    ("skills", "Kenntnisse"),
    ("languages", "Sprachen"),
    ("achievements", "Wichtige Erfolge"),
    ("awards", "Auszeichnungen"),
    ("certificates", "Zertifikate"),
    ("publications", "Veröffentlichungen"),
    ("interests", "Interessen"),
    ("references", "Referenzen"),
    ("profiles", "Online"),
];

/// present, separator, "{type} in {area}", "continued", "{n} of 5", the word after the name in
/// the PDF's title.
type Words = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);
const WORDS_EN: Words = (
    "Present",
    " - ",
    "{type} in {area}",
    "continued",
    "{n} of 5",
    "CV",
);
const WORDS_FR: Words = (
    "aujourd'hui",
    " – ",
    "{type} en {area}",
    "suite",
    "{n} sur 5",
    "CV",
);
const WORDS_DE: Words = (
    "heute",
    " – ",
    "{type} in {area}",
    "Fortsetzung",
    "{n} von 5",
    "Lebenslauf",
);
const NUMERIC_BY_DEFAULT: [&str; 1] = ["de"];

/// The languages with their own headings and words; any other gets CLDR's months only.
pub const FULL: [&str; 3] = ["en", "fr", "de"];

/// Languages Typst lays out by rules of its own that WeasyPrint (cv-builder) does not have: a
/// hyphen repeated after a line break (Lower Sorbian, Czech, Croatian, Polish, Portuguese,
/// Slovak, Spanish), Chinese and Japanese line breaking, right-to-left text. cv-builder draws
/// CVs in these.
pub const TYPST_OWN_RULES: [&str; 20] = [
    "dsb", "cs", "hr", "pl", "pt", "sk", "es", "zh", "ja", "ar", "dv", "fa", "he", "ks", "pa",
    "ps", "sd", "ug", "ur", "yi",
];

/// Languages whose uppercase WeasyPrint tailors (`i` to `İ` in Turkish and Azerbaijani, no
/// accents in Greek) where Typst's `upper` does not: headings would differ.
pub const WEASYPRINT_OWN_RULES: [&str; 3] = ["el", "tr", "az"];

/// Every language cv-forge prints as cv-builder does: English, French and German in full, then
/// every language CLDR has month names for that the bundled fonts print and Typst lays out as
/// WeasyPrint does (see [`TYPST_OWN_RULES`] and [`WEASYPRINT_OWN_RULES`]). cv-forge builds a CV in any language; auto-apply
/// sends it the ones listed here.
pub fn languages() -> Vec<&'static str> {
    let engine = crate::engine::Engine::get();
    let mut out: Vec<&'static str> = FULL.to_vec();
    out.extend(
        months::CODES
            .iter()
            .filter(|(c, i)| {
                !FULL.contains(c)
                    && !TYPST_OWN_RULES.contains(c)
                    && !WEASYPRINT_OWN_RULES.contains(c)
                    && months::TABLES[usize::from(*i)]
                        .iter()
                        .all(|m| engine.covers(m))
            })
            .map(|(c, _)| *c),
    );
    out
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CvLocale {
    /// The tag as written (`de-AT`).
    pub lang: String,
    pub months: [&'static str; 12],
    pub numeric: bool,
    pub present: &'static str,
    pub separator: &'static str,
    pub degree: &'static str,
    pub continued: &'static str,
    pub of_five: &'static str,
    pub cv_word: &'static str,
    #[serde(skip)]
    pub titles: Titles,
}

impl Default for CvLocale {
    fn default() -> Self {
        cv_locale(None, None)
    }
}

impl CvLocale {
    /// The primary language subtag (`de` for `de-AT`), lowercased.
    pub fn base(&self) -> String {
        base(&self.lang)
    }

    /// A two-letter region subtag (`AT` for `de-AT`), uppercased: what Typst takes as `region`.
    pub fn region(&self) -> Option<String> {
        self.lang
            .split('-')
            .skip(1)
            .find(|t| t.len() == 2 && t.chars().all(|c| c.is_ascii_alphabetic()))
            .map(str::to_uppercase)
    }

    /// The heading of section `id` in this language.
    pub fn title(&self, id: &str) -> Option<&'static str> {
        self.titles.iter().find(|(k, _)| *k == id).map(|(_, t)| *t)
    }

    /// "{type} in {area}" in this language.
    pub fn degree(&self, study_type: &str, area: &str) -> String {
        // One pass, as `str.format`: a placeholder inside the CV's own text stays as written.
        let mut out = String::new();
        let mut rest = self.degree;
        while let Some(at) = rest.find('{') {
            out.push_str(&rest[..at]);
            rest = &rest[at..];
            if let Some(r) = rest.strip_prefix("{type}") {
                out.push_str(study_type);
                rest = r;
            } else if let Some(r) = rest.strip_prefix("{area}") {
                out.push_str(area);
                rest = r;
            } else {
                out.push('{');
                rest = &rest[1..];
            }
        }
        out.push_str(rest);
        out
    }
}

fn base(lang: &str) -> String {
    lang.split('-').next().unwrap_or("").to_lowercase()
}

/// CLDR's abbreviated month names for `base`, English when CLDR has none.
pub fn month_names(base: &str) -> [&'static str; 12] {
    if base == "en" {
        return MONTHS;
    }
    months::CODES
        .binary_search_by(|(c, _)| (*c).cmp(base))
        .map_or(MONTHS, |i| months::TABLES[usize::from(months::CODES[i].1)])
}

/// The words for a CV in `language`, months as names or numbers (`date_style`: month | numeric).
pub fn cv_locale(language: Option<&str>, date_style: Option<&str>) -> CvLocale {
    let lang = language
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .unwrap_or("en");
    let base = base(lang);
    let ((present, separator, degree, continued, of_five, cv_word), titles) = match base.as_str() {
        "fr" => (WORDS_FR, TITLES_FR),
        "de" => (WORDS_DE, TITLES_DE),
        _ => (WORDS_EN, TITLES_EN),
    };
    let numeric = match date_style {
        Some("month") => false,
        Some("numeric") => true,
        _ => NUMERIC_BY_DEFAULT.contains(&base.as_str()),
    };
    CvLocale {
        lang: lang.to_string(),
        months: month_names(&base),
        numeric,
        present,
        separator,
        degree,
        continued,
        of_five,
        cv_word,
        titles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::format_range;

    #[test]
    fn months_and_present_follow_the_language() {
        let fr = cv_locale(Some("fr"), None);
        assert_eq!(
            format_range(
                Some("2021-03"),
                None,
                true,
                fr.present,
                fr.separator,
                &fr.months,
                false
            )
            .as_deref(),
            Ok("mars 2021 – aujourd'hui")
        );
        let de = cv_locale(Some("de"), None);
        assert!(de.numeric, "German CVs write months as numbers by default");
        assert_eq!(
            format_range(
                Some("2018-09"),
                Some("2021-02"),
                true,
                de.present,
                de.separator,
                &de.months,
                true
            )
            .as_deref(),
            Ok("09/2018 – 02/2021")
        );
        assert!(!cv_locale(Some("de"), Some("month")).numeric);
        assert_eq!(cv_locale(Some("de"), Some("month")).months[2], "März");
        assert!(
            cv_locale(Some("es"), None).months[0]
                .to_lowercase()
                .starts_with("ene")
        );
        assert_eq!(
            cv_locale(Some("es"), None).title("work"),
            Some("Experience")
        );
        assert_eq!(cv_locale(Some("xx-unknown"), None).months[0], "Jan");
        assert_eq!(
            cv_locale(Some("de-AT"), None).title("work"),
            Some("Berufserfahrung")
        );
        assert_eq!(
            cv_locale(Some("de-AT"), None).region().as_deref(),
            Some("AT")
        );
        assert_eq!(cv_locale(Some("zh-Hant"), None).region(), None);
        assert_eq!(
            cv_locale(None, None).degree("BSc", "Physics"),
            "BSc in Physics"
        );
        assert_eq!(
            cv_locale(Some("fr"), None).degree("Master", "Informatique"),
            "Master en Informatique"
        );
        assert_eq!(cv_locale(None, None).degree("{area}", "X"), "{area} in X");
    }

    #[test]
    fn languages_start_with_the_full_ones() {
        let all = languages();
        assert_eq!(&all[..3], &["en", "fr", "de"]);
        assert!(all.contains(&"it") && all.contains(&"nl") && all.contains(&"ru"));
        // Typst's own hyphen and line-breaking rules; months the bundled fonts lack
        assert!(!all.contains(&"es") && !all.contains(&"ja") && !all.contains(&"ar"));
        assert!(!all.contains(&"th") && !all.contains(&"hi"));
        assert!(!all.contains(&"tr") && !all.contains(&"el"));
        assert_eq!(all.iter().filter(|c| **c == "fr").count(), 1);
    }
}
