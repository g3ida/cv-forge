// Clear (formerly single-column): one plain column in reading order, for job portals and resume
// parsers. Port of cv-builder's templates/clear (cv.html.j2 + style.css), box for box.
//
// Built for text extraction: one block after another in reading order; separators and bullets
// are real text; no letter-spacing, hyphenation or ligatures. The dates and location lines stay
// at the body's size (smaller meta text made pdftotext lose titles and employers).

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#2a2a2a")
#let heading-ink = rgb("#000000")
#let muted = rgb("#55595e")
#let rule-ink = rgb("#b9bdc2")
#let sans = "Inter"
#let display = "Rubik"
#let width = (data.page.width - m.left - m.right) * 1pt
#let lines(v, family: sans) = line-box(family, lh(v))

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: sans, size: 10pt, fill: ink, lang: "en", ligatures: false, hyphenate: false, ..lines(1.36))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
// Headings carry the PDF structure; their look is set inside each body.
#show heading: it => it.body

// ------------------------------------------------------------------ header

// One paragraph; each contact keeps its " |" and never breaks inside.
#let contacts(items) = par(items.enumerate().map(((i, c)) => {
  let sep = if i < items.len() - 1 { [#" |"] } else { [] }
  let space = if i < items.len() - 1 { [ ] } else { [] }
  nowrap(link-or([#c.text], c.href) + sep, w: c.w + 1, room: width - 76pt) + space
}).join())

#let header-text() = {
  block(width: 100%, heading(level: 1, text(font: display, size: 22pt, weight: 500, fill: heading-ink, ..lines(1.2, family: display), data.header.name)))
  if not blank(data.header.label) {
    pad-top(4pt, text(size: 11.5pt, weight: 600, fill: accent, ..lines(1.3), rich(data.header.label)))
  }
  if data.header.contacts.len() > 0 {
    pad-top(if blank(data.header.label) { 4pt } else { 3pt }, text(fill: muted, contacts(data.header.contacts)))
  }
}

// The photo sits to the right of the text, top-aligned: a 62pt circle.
#let header() = block(width: 100%, inset: (bottom: 9pt), if data.header.photo == none { header-text() } else {
  grid(
    columns: (1fr, 62pt),
    column-gutter: 14pt,
    header-text(),
    box(width: 62pt, height: 62pt, radius: 31pt, clip: true, image(data.header.photo, width: 62pt, height: 62pt, fit: "cover")),
  )
})

// ------------------------------------------------------------------ items

#let section-title(chunk) = block(width: 100%, inset: (bottom: 3.5pt), {
  block(width: 100%, inset: (bottom: 1.5pt), heading(level: 2, text(font: display, size: 13pt, weight: 500, fill: accent, ..lines(1.2, family: display), chunk.title)))
  rule(0.75pt, rule-ink)
})

#let project = (entry-project: it => text(weight: 500, fill: heading-ink, it))

// Title, employer, the dates alone on their line, then location, score and link: a parser
// attaches whatever shares a line with a date range to the role. A continued piece has bullets
// only. A role at the same company as the one above sits closer to it.
#let entry(e, first) = pad-top(if first or e.continued { 0pt } else if e.same_employer { 4pt } else { 7pt }, {
  if not blank(e.title) {
    block(width: 100%, heading(level: 3, text(size: 11pt, weight: 600, fill: heading-ink, ..lines(1.3), link-or(rich(e.title), e.title_url))))
  }
  if not blank(e.subtitle) {
    block(width: 100%, text(weight: 500, fill: heading-ink, link-or(rich(e.subtitle), e.subtitle_url)))
  }
  let meta = e.meta.flatten()
  let dates = meta.filter(x => x.icon == "calendar")
  let rest = meta.filter(x => x.icon == "pin") + meta.filter(x => x.icon not in ("pin", "calendar"))
  if dates.len() > 0 { block(width: 100%, text(fill: muted, dates.map(x => x.text).join(" | "))) }
  if rest.len() > 0 { block(width: 100%, text(fill: muted, rest.map(x => link-or([#x.text], x.href)).join([#" | "]))) }
  for p in e.paragraphs { pad-top(2pt, rich(p, classes: project)) }
  for b in e.bullets { pad-top(1.5pt, par(hanging-indent: 9pt, box(width: 9pt, [•]) + rich(b))) }
  if not blank(e.tags) { pad-top(2pt, text(fill: muted, rich(e.tags))) }
})

#let text-item(paras) = for (i, p) in paras.enumerate() { pad-top(if i > 0 { 3pt } else { 0pt }, rich(p)) }

// "Group: keyword, keyword" on one line; the colon only when keywords follow.
#let tags(s) = pad-top(1.5pt, par({
  text(weight: 600, fill: heading-ink, {
    rich(s.name)
    if s.level != none { text(weight: 400, fill: ink, [ · #s.level]) }
    if not blank(s.text) { [:] }
  })
  if not blank(s.text) { [ ] + rich(s.text) }
}))

#let achievement(a, first) = pad-top(if first { 0pt } else { 4pt }, {
  block(width: 100%, heading(level: 3, text(weight: 600, fill: heading-ink, rich(a.title))))
  if not blank(a.summary) { block(width: 100%, rich(a.summary)) }
})

#let language(l, first) = pad-top(if first { 0pt } else { 1.5pt }, par({
  text(weight: 600, fill: heading-ink, l.name)
  if l.fluency != none { [: #l.fluency] } else if l.rating > 0 { [: #str(l.rating)/5] }
}))

#let profiles(items) = for (i, p) in items.enumerate() {
  pad-top(if i > 0 { 1.5pt } else { 0pt }, par({
    text(weight: 600, fill: heading-ink, p.network)
    [: ]
    link-or([#p.text], p.href)
  }))
}

#let reference(r, first) = pad-top(if first { 0pt } else { 4pt }, {
  for p in r.text { block(width: 100%, [“] + rich(p) + [”]) }
  block(width: 100%, text(weight: 600, fill: heading-ink, r.name))
})

// Certificates and awards, one line each: "Name, Issuer, 2026".
#let line-item(l, first) = pad-top(if first { 0pt } else { 3pt }, {
  block(width: 100%, par({
    text(weight: 600, fill: heading-ink, link-or(rich(l.title), l.url))
    if not blank(l.detail) { [, ] + rich(l.detail) }
  }))
  if not blank(l.summary) { pad-top(1pt, rich(l.summary)) }
})

#let item(chunk, it) = block(width: 100%, breakable: it.overflow, {
  let d = it.data
  let k = chunk.kind
  if k == "entry" { entry(d, it.first) } else if k == "text" { text-item(d) } else if k == "tags" { tags(d) } else if k == "achievement" {
    achievement(d, it.first)
  } else if k == "language" { language(d, it.first) } else if k == "profiles" { profiles(d) } else if k == "reference" {
    reference(d, it.first)
  } else if k == "line" { line-item(d, it.first) }
})

// CSS: `.section { margin-top: 10pt }`, none for the first in its column.
#let section(chunk, first) = {
  if not first { block(width: 100%, height: 10pt) }
  block(width: 100%, {
    if measuring { mark("s", chunk.id) }
    if data.repeat_titles or not chunk.continued { section-title(chunk) }
    for it in chunk.items {
      if measuring { mark("i") }
      item(chunk, it)
      if measuring { mark("j") }
    }
  })
}

// ------------------------------------------------------------------ document

#if measuring {
  set page(width: data.page.width * 1pt, height: auto, margin: 0pt)
  block(width: width, {
    header()
    mark("c")
    for (ci, chunk) in data.pages.at(0).columns.at("main").enumerate() { section(chunk, ci == 0) }
  })
  [#metadata((text: 10, line: lh(1.36) * 10, width: width / 1pt))<cvforge-measure>]
} else {
  set page(
    width: data.page.width * 1pt,
    height: data.page.height * 1pt,
    margin: (top: m.top * 1pt, right: m.right * 1pt, bottom: m.bottom * 1pt, left: m.left * 1pt),
  )
  for (pi, p) in data.pages.enumerate() {
    if pi > 0 { pagebreak() }
    if p.header { header() }
    for (ci, chunk) in p.columns.at("main").enumerate() { section(chunk, ci == 0) }
  }
}
