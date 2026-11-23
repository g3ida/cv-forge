// Executive: an understated one-column page in forest green for senior roles, read in order by
// resume parsers. Port of cv-builder's templates/executive (cv.html.j2 + style.css), box for box.
//
// A display-serif name over a fine accent rule, the summary as an opening paragraph in italic
// serif, quiet serif headings under a short accent bar, the text in Noto Sans. Each entry's
// title, employer, dates and place sit on lines of their own, all on the left.

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#2b2d2c")
#let heading-ink = rgb("#161817")
#let muted = rgb("#616664")
#let sans = "Noto Sans"
#let display = "Noto Serif Display"
#let serif = "Noto Serif"
#let width = (data.page.width - m.left - m.right) * 1pt
#let lines(v, family: sans) = line-box(family, lh(v))

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: sans, size: 10pt, fill: ink, lang: data.lang, region: data.region, ligatures: false, hyphenate: false, ..lines(1.4))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
#show heading: it => it.body

// ------------------------------------------------------------------ header

#let contacts(items, room) = par(items.enumerate().map(((i, c)) => {
  let sep = if i < items.len() - 1 { [#" ·"] } else { [] }
  let space = if i < items.len() - 1 { [ ] } else { [] }
  nowrap(link-or([#c.text], c.href) + sep, w: c.w + 1, room: room) + space
}).join())

#let header-text(room) = {
  block(width: 100%, heading(level: 1, text(font: display, size: 29pt, weight: 400, fill: heading-ink, ..lines(1.12, family: display), data.header.name)))
  if not blank(data.header.label) {
    pad-top(8pt, text(size: 11.5pt, fill: accent, ..lines(1.3), smallcaps(rich(data.header.label))))
  }
  if data.header.contacts.len() > 0 {
    pad-top(if blank(data.header.label) { 8pt } else { 4pt }, text(fill: muted, contacts(data.header.contacts, room)))
  }
}

// A portrait-shaped photo to the right of the text, bottom-aligned with it; then the accent rule.
#let header() = block(width: 100%, inset: (bottom: 14pt), {
  block(width: 100%, inset: (bottom: 10pt), if data.header.photo == none { header-text(width) } else {
    grid(
      columns: (1fr, 60pt),
      column-gutter: 14pt,
      align: bottom,
      header-text(width - 74pt),
      box(width: 60pt, height: 74pt, clip: true, image(data.header.photo, width: 60pt, height: 74pt, fit: "cover")),
    )
  })
  rule(1pt, accent)
})

// ------------------------------------------------------------------ items

// A short accent bar over a quiet serif heading.
#let section-title(chunk) = block(width: 100%, inset: (bottom: 4pt), {
  block(width: 22pt, height: 1.5pt, fill: accent)
  block(width: 100%, height: 4pt)
  heading(level: 2, text(font: display, size: 15pt, weight: 400, fill: accent, ..lines(1.2, family: display), chunk.title))
})

#let entry(e, first) = pad-top(if first or e.continued { 0pt } else if e.same_employer { 5pt } else { 8pt }, {
  if not blank(e.title) {
    block(width: 100%, heading(level: 3, text(size: 10.5pt, weight: 700, fill: heading-ink, ..lines(1.35), link-or(rich(e.title), e.title_url))))
  }
  if not blank(e.subtitle) { block(width: 100%, text(fill: accent, link-or(rich(e.subtitle), e.subtitle_url))) }
  let meta = e.meta.flatten()
  let dates = meta.filter(x => x.icon == "calendar")
  let rest = meta.filter(x => x.icon == "pin") + meta.filter(x => x.icon not in ("pin", "calendar"))
  if dates.len() > 0 { block(width: 100%, text(fill: muted, dates.map(x => x.text).join(" | "))) }
  if rest.len() > 0 { block(width: 100%, text(fill: muted, rest.map(x => link-or([#x.text], x.href)).join([#" · "]))) }
  for p in e.paragraphs { pad-top(2pt, rich(p)) }
  for b in e.bullets { pad-top(1.5pt, par(hanging-indent: 11pt, box(width: 11pt, text(fill: accent, [•])) + rich(b))) }
  if not blank(e.tags) { pad-top(2pt, text(fill: muted, rich(e.tags))) }
})

#let text-item(paras) = for (i, p) in paras.enumerate() { pad-top(if i > 0 { 4pt } else { 0pt }, rich(p)) }

// The summary opens the page as a lead paragraph.
#let lead(paras) = {
  set text(font: serif, style: "italic", size: 11pt, fill: heading-ink, ..lines(1.45, family: serif))
  text-item(paras)
}

// "Group: keyword, keyword" on one line; the colon only when keywords follow.
#let tags(s) = pad-top(2pt, par({
  text(weight: 700, fill: heading-ink, {
    rich(s.name)
    if s.level != none { text(weight: 400, fill: muted, [ · #s.level]) }
    if not blank(s.text) { [:] }
  })
  if not blank(s.text) { [ ] + rich(s.text) }
}))

#let achievement(a, first) = pad-top(if first { 0pt } else { 5pt }, {
  block(width: 100%, heading(level: 3, text(weight: 700, fill: heading-ink, rich(a.title))))
  if not blank(a.summary) { block(width: 100%, rich(a.summary)) }
})

#let language(l, first) = pad-top(if first { 0pt } else { 1.5pt }, par({
  text(weight: 700, fill: heading-ink, l.name)
  if l.fluency != none { [: #l.fluency] } else if l.rating > 0 { [: #str(l.rating)/5] }
}))

#let profiles(items) = for (i, p) in items.enumerate() {
  pad-top(if i > 0 { 1.5pt } else { 0pt }, par({
    text(weight: 700, fill: heading-ink, p.network)
    [: ]
    link-or([#p.text], p.href)
  }))
}

#let reference(r, first) = pad-top(if first { 0pt } else { 5pt }, {
  for p in r.text { block(width: 100%, text(font: serif, style: "italic", ..lines(1.4, family: serif), [“] + rich(p) + [”])) }
  block(width: 100%, text(weight: 700, fill: heading-ink, r.name))
})

// Certificates and awards, one line each: "Name, Issuer, 2026".
#let line-item(l, first) = pad-top(if first { 0pt } else { 3pt }, {
  block(width: 100%, par({
    text(weight: 700, fill: heading-ink, link-or(rich(l.title), l.url))
    if not blank(l.detail) { [, ] + rich(l.detail) }
  }))
  if not blank(l.summary) { pad-top(1pt, rich(l.summary)) }
})

#let item(chunk, it) = block(width: 100%, breakable: it.overflow, {
  let d = it.data
  let k = chunk.kind
  if k == "entry" { entry(d, it.first) } else if k == "text" { if chunk.id == "summary" { lead(d) } else { text-item(d) } } else if k == "tags" {
    tags(d)
  } else if k == "achievement" { achievement(d, it.first) } else if k == "language" { language(d, it.first) } else if k == "profiles" {
    profiles(d)
  } else if k == "reference" { reference(d, it.first) } else if k == "line" { line-item(d, it.first) }
})

// CSS: `.section { margin-top: 13pt }`, none for the first in its column.
#let section(chunk, first) = {
  if not first { block(width: 100%, height: 13pt) }
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
  [#metadata((text: 10, line: lh(1.4) * 10, width: width / 1pt))<cvforge-measure>]
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
