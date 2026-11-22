//! The validated CV turned into display-ready sections for templates (port of `view.py`).

use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use serde::Serialize;

use crate::dates;
use crate::model::{self, Cv, Profile};
use crate::pyfmt;
use crate::text::{self, Rich, href, inline, keep, keep_str, paragraphs, pretty_url};
use crate::wording::{CvLocale, TITLES_EN, Titles};

pub const DEFAULT_TITLES: Titles = TITLES_EN;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct MetaItem {
    pub icon: &'static str,
    pub text: String,
    pub href: Option<String>,
    /// An upper bound of the text's width in em (see `Engine::width_bound`).
    pub w: f64,
}

impl MetaItem {
    fn new(icon: &'static str, text: impl Into<String>, href: Option<String>) -> Self {
        let text = text.into();
        let w = crate::engine::Engine::get().width_bound(&text);
        MetaItem {
            icon,
            text,
            href,
            w,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Entry {
    pub title: Option<Rich>,
    pub title_url: Option<String>,
    pub subtitle: Option<Rich>,
    pub subtitle_url: Option<String>,
    pub meta: Vec<Vec<MetaItem>>,
    pub paragraphs: Vec<Rich>,
    pub bullets: Vec<Rich>,
    pub tags: Option<Rich>,
    /// A later piece of an entry cut up by `split_entries`.
    pub continued: bool,
    /// Still going on: no end date, or "present" (the timeline marks it, whatever the language).
    pub current: bool,
    /// A work entry at the same company as the one before it.
    pub same_employer: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Line {
    pub title: Rich,
    pub url: Option<String>,
    pub detail: Option<Rich>,
    pub summary: Option<Rich>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Tags {
    pub name: Rich,
    pub level: Option<String>,
    pub text: Rich,
    /// The keywords one by one (sidebar's chips): the CV's own list of the first skill or
    /// interest of this name, else the joined text split at ", " (as the original's template).
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct LanguageItem {
    pub name: String,
    pub fluency: Option<String>,
    pub rating: i64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AchievementItem {
    pub title: Rich,
    pub summary: Option<Rich>,
    pub icon: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ReferenceItem {
    pub name: String,
    pub text: Vec<Rich>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProfileItem {
    pub network: String,
    pub text: String,
    pub href: Option<String>,
    pub icon: &'static str,
}

/// One item of a section; the section's `kind` says which.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Item {
    Text(Vec<Rich>),
    Entry(Entry),
    Line(Line),
    Tags(Tags),
    Language(LanguageItem),
    Achievement(AchievementItem),
    Reference(ReferenceItem),
    Profiles(Vec<ProfileItem>),
}

impl Item {
    fn continued(&self) -> bool {
        matches!(self, Item::Entry(e) if e.continued)
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Section {
    pub id: &'static str,
    pub title: String,
    /// The template macro that draws each item.
    pub kind: &'static str,
    pub items: Vec<Item>,
}

impl Section {
    /// Index in the CV's own list of the entry `items[index]` belongs to.
    pub fn source_index(&self, index: usize) -> usize {
        self.items[..=index]
            .iter()
            .filter(|i| !i.continued())
            .count()
            .saturating_sub(1)
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Header {
    pub name: String,
    pub label: Option<Rich>,
    pub contacts: Vec<MetaItem>,
    /// Where the template finds the photo (a file of the Typst world), if any.
    pub photo: Option<String>,
}

/// `layout.header` with the template's defaults filled in.
#[derive(Clone, Debug, PartialEq)]
pub struct HeaderLayout {
    pub contacts: Vec<String>,
    pub profiles: Vec<String>,
    pub photo: bool,
}

// ------------------------------------------------------------------ builders

fn meta_line(items: [Option<MetaItem>; 3]) -> Vec<MetaItem> {
    items
        .into_iter()
        .flatten()
        .filter(|i| !i.text.is_empty())
        .collect()
}

fn date_item(text: String) -> Option<MetaItem> {
    (!text.is_empty()).then(|| MetaItem::new("calendar", text, None))
}

fn place(text: Option<&str>) -> Option<MetaItem> {
    text.filter(|t| !t.is_empty())
        .map(|t| MetaItem::new("pin", t, None))
}

fn same(a: Option<&str>, b: Option<&str>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => {
            caseless::default_case_fold_str(&pyfmt::collapse(a))
                == caseless::default_case_fold_str(&pyfmt::collapse(b))
        }
        _ => false,
    }
}

/// `Engineer → Senior Engineer` when a role held two titles or more (no dates on this line: a
/// parser takes a dated line for another role).
fn titles_line(w: &model::Work) -> Vec<Rich> {
    let mut titles: Vec<&str> = Vec::new();
    for span in &w.title_history {
        if titles
            .last()
            .is_none_or(|last| !same(Some(last), Some(&span.position)))
        {
            titles.push(&span.position);
        }
    }
    if titles.len() > 1 {
        vec![keep_str(&titles.join(" → "))]
    } else {
        Vec::new()
    }
}

/// Still going on: a start and no end, or the end `present`.
fn ongoing(start: Option<&str>, end: Option<&str>) -> bool {
    end == Some("present") || (end.is_none() && start.is_some_and(|s| !s.is_empty()))
}

fn inlines(items: &[String]) -> Vec<Rich> {
    items.iter().map(|h| inline(Some(h))).collect()
}

fn work(w: &model::Work, previous: Option<&model::Work>, words: &CvLocale) -> Entry {
    let same_employer = previous.is_some_and(|p| same(p.name.as_deref(), w.name.as_deref()));
    let description = if same_employer
        && previous.is_some_and(|p| same(p.description.as_deref(), w.description.as_deref()))
    {
        Vec::new()
    } else {
        paragraphs(w.description.as_deref())
    };
    let mut paras = description;
    if let Some(project) = &w.project {
        paras.push(text::project(project));
    }
    paras.extend(titles_line(w));
    paras.extend(paragraphs(w.summary.as_deref()));
    Entry {
        title: keep(w.position.as_deref()),
        subtitle: keep(w.name.as_deref()),
        subtitle_url: href(w.url.as_deref()),
        meta: vec![meta_line([
            date_item(dates::range(
                w.start_date.as_deref(),
                w.end_date.as_deref(),
                words,
            )),
            place(w.location.as_deref()),
            None,
        ])],
        paragraphs: paras,
        bullets: inlines(&w.highlights),
        same_employer,
        current: ongoing(w.start_date.as_deref(), w.end_date.as_deref()),
        ..Entry::default()
    }
}

fn volunteer(v: &model::Volunteer, words: &CvLocale) -> Entry {
    Entry {
        title: keep(v.position.as_deref()),
        subtitle: keep(v.organization.as_deref()),
        subtitle_url: href(v.url.as_deref()),
        meta: vec![meta_line([
            date_item(dates::range(
                v.start_date.as_deref(),
                v.end_date.as_deref(),
                words,
            )),
            place(v.location.as_deref()),
            None,
        ])],
        current: ongoing(v.start_date.as_deref(), v.end_date.as_deref()),
        paragraphs: paragraphs(v.summary.as_deref()),
        bullets: inlines(&v.highlights),
        ..Entry::default()
    }
}

fn education(e: &model::Education, words: &CvLocale) -> Entry {
    let title = match (e.study_type.as_deref(), e.area.as_deref()) {
        (Some(t), Some(a)) => Some(words.degree(t, a)),
        (t, a) => t.or(a).map(str::to_string),
    };
    let score = e.score.as_deref().map(|s| MetaItem::new("award", s, None));
    Entry {
        title: keep(title.as_deref()),
        subtitle: keep(e.institution.as_deref()),
        subtitle_url: href(e.url.as_deref()),
        meta: vec![meta_line([
            date_item(dates::range(
                e.start_date.as_deref(),
                e.end_date.as_deref(),
                words,
            )),
            place(e.location.as_deref()),
            score,
        ])],
        current: ongoing(e.start_date.as_deref(), e.end_date.as_deref()),
        bullets: inlines(&e.highlights),
        tags: (!e.courses.is_empty()).then(|| keep_str(&e.courses.join(", "))),
        ..Entry::default()
    }
}

fn project(p: &model::Project, words: &CvLocale) -> Entry {
    let roles = p.roles.join(", ");
    let subtitle: Vec<&str> = [Some(roles.as_str()), p.entity.as_deref()]
        .into_iter()
        .flatten()
        .filter(|x| !x.is_empty())
        .collect();
    let mut meta = vec![meta_line([
        date_item(dates::range(
            p.start_date.as_deref(),
            p.end_date.as_deref(),
            words,
        )),
        None,
        None,
    ])];
    if let Some(url) = p.url.as_deref() {
        meta.push(vec![MetaItem::new("link", url, href(Some(url)))]);
    }
    Entry {
        title: keep(Some(&p.name)),
        subtitle: (!subtitle.is_empty()).then(|| keep_str(&subtitle.join(" · "))),
        meta: meta.into_iter().filter(|line| !line.is_empty()).collect(),
        paragraphs: paragraphs(p.description.as_deref()),
        bullets: inlines(&p.highlights),
        tags: (!p.keywords.is_empty()).then(|| keep_str(&p.keywords.join(", "))),
        ..Entry::default()
    }
}

fn year(date: Option<&str>) -> Option<String> {
    date.filter(|d| !d.is_empty())
        .map(|d| d.chars().take(4).collect())
}

/// One line: the name in bold (linked with a URL), then `, detail, detail` (certificates, awards).
fn line(title: &str, url: Option<&str>, details: &[Option<&str>], summary: Option<&str>) -> Line {
    let detail: Vec<&str> = details
        .iter()
        .flatten()
        .copied()
        .filter(|d| !d.is_empty())
        .collect();
    Line {
        title: keep_str(title),
        url: href(url),
        detail: (!detail.is_empty()).then(|| keep_str(&detail.join(", "))),
        summary: summary.filter(|s| !s.is_empty()).map(|s| inline(Some(s))),
    }
}

fn publication(p: &model::Publication, words: &CvLocale) -> Entry {
    let mut meta = vec![meta_line([
        date_item(dates::date(p.release_date.as_deref(), words)),
        None,
        None,
    ])];
    if let Some(url) = p.url.as_deref() {
        meta.push(vec![MetaItem::new(
            "link",
            pretty_url(url),
            href(Some(url)),
        )]);
    }
    Entry {
        title: keep(Some(&p.name)),
        title_url: href(p.url.as_deref()),
        subtitle: keep(p.publisher.as_deref()),
        meta: meta.into_iter().filter(|m| !m.is_empty()).collect(),
        paragraphs: paragraphs(p.summary.as_deref()),
        ..Entry::default()
    }
}

pub fn network_icon(network: &str) -> &'static str {
    match network.to_lowercase().replace(' ', "").as_str() {
        "github" => "github",
        "linkedin" => "linkedin",
        _ => "globe",
    }
}

fn profile_item(p: &Profile) -> ProfileItem {
    let text = match (&p.url, &p.username) {
        (Some(url), _) => pretty_url(url),
        (None, Some(user)) => user.clone(),
        (None, None) => p.network.clone(),
    };
    ProfileItem {
        network: p.network.clone(),
        text,
        href: href(p.url.as_deref()),
        icon: network_icon(&p.network),
    }
}

static NOT_PHONE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\d+]").expect("static regex"));

pub fn build_header(cv: &Cv, layout: &HeaderLayout, photo: Option<String>) -> Header {
    let b = &cv.basics;
    let wanted: Vec<String> = layout.profiles.iter().map(|n| n.to_lowercase()).collect();
    let mut contacts = Vec::new();
    for key in &layout.contacts {
        match key.as_str() {
            "phone" => {
                if let Some(phone) = &b.phone {
                    contacts.push(MetaItem::new(
                        "phone",
                        phone.clone(),
                        Some(format!("tel:{}", NOT_PHONE.replace_all(phone, ""))),
                    ));
                }
            }
            "email" => {
                if let Some(email) = &b.email {
                    contacts.push(MetaItem::new(
                        "at",
                        email.clone(),
                        Some(format!("mailto:{email}")),
                    ));
                }
            }
            "url" => {
                if let Some(url) = &b.url {
                    contacts.push(MetaItem::new("link", pretty_url(url), href(Some(url))));
                }
            }
            "location" => {
                if let Some(loc) = &b.location {
                    let parts: Vec<&str> = [
                        loc.city.as_deref(),
                        loc.region.as_deref().or(loc.country_code.as_deref()),
                    ]
                    .into_iter()
                    .flatten()
                    .filter(|x| !x.is_empty())
                    .collect();
                    if !parts.is_empty() {
                        contacts.push(MetaItem::new("pin", parts.join(", "), None));
                    }
                }
            }
            _ => {}
        }
    }
    for p in &b.profiles {
        if wanted.contains(&p.network.to_lowercase()) {
            let item = profile_item(p);
            contacts.push(MetaItem::new("link", item.text, item.href));
        }
    }
    Header {
        name: b.name.clone(),
        label: keep(b.label.as_deref()),
        contacts,
        photo,
    }
}

/// Every non-empty section, by id, in the original's order, in `words`' language (headings,
/// dates): `titles` over the language's own headings.
pub fn build_sections(
    cv: &Cv,
    titles: &IndexMap<String, String>,
    header_profiles: &[String],
    words: &CvLocale,
) -> IndexMap<&'static str, Section> {
    let title = |id: &str| -> String {
        titles
            .get(id)
            .cloned()
            .unwrap_or_else(|| words.title(id).unwrap_or_default().to_string())
    };
    let in_header: Vec<String> = header_profiles.iter().map(|n| n.to_lowercase()).collect();
    let mut raw: Vec<(&'static str, &'static str, Vec<Item>)> = Vec::with_capacity(14);
    raw.push((
        "summary",
        "text",
        cv.basics
            .summary
            .as_deref()
            .map(|s| vec![Item::Text(paragraphs(Some(s)))])
            .unwrap_or_default(),
    ));
    raw.push((
        "work",
        "entry",
        cv.work
            .iter()
            .enumerate()
            .map(|(n, w)| Item::Entry(work(w, n.checked_sub(1).map(|p| &cv.work[p]), words)))
            .collect(),
    ));
    raw.push((
        "volunteer",
        "entry",
        cv.volunteer
            .iter()
            .map(|v| Item::Entry(volunteer(v, words)))
            .collect(),
    ));
    raw.push((
        "education",
        "entry",
        cv.education
            .iter()
            .map(|e| Item::Entry(education(e, words)))
            .collect(),
    ));
    raw.push((
        "projects",
        "entry",
        cv.projects
            .iter()
            .map(|p| Item::Entry(project(p, words)))
            .collect(),
    ));
    raw.push((
        "awards",
        "line",
        cv.awards
            .iter()
            .map(|a| {
                Item::Line(line(
                    &a.title,
                    None,
                    &[a.awarder.as_deref(), year(a.date.as_deref()).as_deref()],
                    a.summary.as_deref(),
                ))
            })
            .collect(),
    ));
    raw.push((
        "certificates",
        "line",
        cv.certificates
            .iter()
            .map(|c| {
                Item::Line(line(
                    &c.name,
                    c.url.as_deref(),
                    &[c.issuer.as_deref(), year(c.date.as_deref()).as_deref()],
                    None,
                ))
            })
            .collect(),
    ));
    raw.push((
        "publications",
        "entry",
        cv.publications
            .iter()
            .map(|p| Item::Entry(publication(p, words)))
            .collect(),
    ));
    // The original's sidebar finds a group by comparing its escaped name to the CV's names, so
    // a name with characters `keep` changes (`&<>"'`, "++") is never found.
    let chips = |name: &str, joined: &str| -> Vec<String> {
        let plain = !name.contains("++") && !name.contains(['&', '<', '>', '"', '\'']);
        let found = cv
            .skills
            .iter()
            .map(|s| (&s.name, &s.keywords))
            .chain(cv.interests.iter().map(|i| (&i.name, &i.keywords)))
            .find(|(n, _)| plain && n.as_str() == name);
        match found {
            Some((_, words)) if !words.is_empty() => words.clone(),
            _ if joined.is_empty() => Vec::new(),
            _ => joined.split(", ").map(str::to_string).collect(),
        }
    };
    let tags = |name: &str, level: Option<String>, words: &[String]| {
        let joined = words.join(", ");
        Item::Tags(Tags {
            name: keep_str(name),
            level,
            keywords: chips(name, &joined),
            text: keep_str(&joined),
        })
    };
    raw.push((
        "skills",
        "tags",
        cv.skills
            .iter()
            .map(|s| tags(&s.name, s.level.clone(), &s.keywords))
            .collect(),
    ));
    raw.push((
        "interests",
        "tags",
        cv.interests
            .iter()
            .map(|i| tags(&i.name, None, &i.keywords))
            .collect(),
    ));
    raw.push((
        "languages",
        "language",
        cv.languages
            .iter()
            .map(|l| {
                Item::Language(LanguageItem {
                    name: l.language.clone(),
                    fluency: l.fluency.clone(),
                    rating: text::language_rating(l.fluency.as_deref(), l.rating),
                })
            })
            .collect(),
    ));
    raw.push((
        "achievements",
        "achievement",
        cv.achievements
            .iter()
            .map(|a| {
                let summary = inline(a.summary.as_deref());
                Item::Achievement(AchievementItem {
                    title: keep_str(&a.title),
                    summary: (!summary.is_empty()).then_some(summary),
                    icon: a.icon.clone().unwrap_or_else(|| "gem".into()),
                })
            })
            .collect(),
    ));
    raw.push((
        "references",
        "reference",
        cv.references
            .iter()
            .map(|r| {
                Item::Reference(ReferenceItem {
                    name: r.name.clone(),
                    text: paragraphs(r.reference.as_deref()),
                })
            })
            .collect(),
    ));
    let online: Vec<ProfileItem> = cv
        .basics
        .profiles
        .iter()
        .filter(|p| !in_header.contains(&p.network.to_lowercase()))
        .map(profile_item)
        .collect();
    raw.push((
        "profiles",
        "profiles",
        if online.is_empty() {
            Vec::new()
        } else {
            vec![Item::Profiles(online)]
        },
    ));

    raw.into_iter()
        .filter(|(_, _, items)| !items.is_empty())
        .map(|(id, kind, items)| {
            (
                id,
                Section {
                    id,
                    title: title(id),
                    kind,
                    items,
                },
            )
        })
        .collect()
}

/// Cut a section's entries into pieces a page break may fall between: the heading, paragraphs
/// and first bullet stay together; each further bullet is a piece; tags go with the last one.
pub fn split_entries(section: Section) -> Section {
    if section.kind != "entry" {
        return section;
    }
    let mut pieces = Vec::with_capacity(section.items.len() * 3);
    for item in section.items {
        let Item::Entry(entry) = item else {
            pieces.push(item);
            continue;
        };
        if entry.bullets.len() < 2 {
            pieces.push(Item::Entry(entry));
            continue;
        }
        let mut bullets = entry.bullets.clone().into_iter();
        let first = bullets.next().into_iter().collect();
        let tags = entry.tags.clone();
        pieces.push(Item::Entry(Entry {
            bullets: first,
            tags: None,
            ..entry
        }));
        pieces.extend(bullets.map(|b| {
            Item::Entry(Entry {
                bullets: vec![b],
                continued: true,
                ..Entry::default()
            })
        }));
        if let Some(Item::Entry(last)) = pieces.last_mut() {
            last.tags = tags;
        }
    }
    Section {
        items: pieces,
        ..section
    }
}
