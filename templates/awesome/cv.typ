// Awesome: one column in red and charcoal, read in order by resume parsers. Port of
// cv-builder's templates/awesome (cv.html.j2 + style.css), box for box.
//
// A centred header with the first names light and the surname bold, headings whose first three
// letters carry the accent with a hairline on to the margin, and entries headed "Title,
// Employer"; the dates follow alone on their own line, on the left.

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#333333")
#let heading-ink = rgb("#1f1f1f")
#let light = rgb("#7a7a7a")
#let muted = rgb("#666666")
#let rule-ink = rgb("#b5b5b5")
#let sans = "Noto Sans"
#let width = (data.page.width - m.left - m.right) * 1pt
#let lines(v) = line-box(sans, lh(v))

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: sans, size: 10pt, fill: ink, lang: data.lang, region: data.region, ligatures: false, hyphenate: false, ..lines(1.36))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
#show heading: it => it.body

// ------------------------------------------------------------------ header

#let contacts(items, room) = par(items.enumerate().map(((i, c)) => {
  let sep = if i < items.len() - 1 { [#" |"] } else { [] }
  let space = if i < items.len() - 1 { [ ] } else { [] }
  nowrap(link-or([#c.text], c.href) + sep, w: c.w + 1, room: room) + space
}).join())

// The name split at its last space (Python's rsplit(" ", 1)): first names light, surname bold.
#let name() = {
  let n = data.header.name
  let parts = n.split(" ")
  if parts.len() > 1 {
    text(weight: 400, fill: light, parts.slice(0, -1).join(" ")) + [ ] + text(weight: 700, fill: heading-ink, parts.last())
  } else { text(weight: 700, fill: heading-ink, n) }
}

#let header-text(room) = {
  set align(center)
  block(width: 100%, heading(level: 1, text(size: 25pt, ..lines(1.15), name())))
  if not blank(data.header.label) {
    pad-top(6pt, text(size: 11pt, fill: accent, ..lines(1.3), smallcaps(rich(data.header.label))))
  }
  if data.header.contacts.len() > 0 {
    pad-top(if blank(data.header.label) { 6pt } else { 3pt }, text(fill: muted, contacts(data.header.contacts, room)))
  }
}

// With a photo the text stays centred on the page: an empty box the photo's width balances it.
#let header() = block(width: 100%, inset: (bottom: 10pt), if data.header.photo == none { header-text(width) } else {
  grid(
    columns: (66pt, 1fr, 66pt),
    align: horizon,
    box(width: 66pt, height: 66pt, radius: 33pt, clip: true, image(data.header.photo, width: 66pt, height: 66pt, fit: "cover")),
    pad(x: 10pt, header-text(width - 152pt)),
    box(width: 66pt, height: 66pt),
  )
})

// ------------------------------------------------------------------ items

// "Experience" with "Exp" in the accent, then a hairline to the margin.
#let section-title(chunk) = block(width: 100%, inset: (bottom: 4pt), grid(
  columns: (auto, 1fr),
  column-gutter: 7pt,
  align: horizon,
  heading(level: 2, text(size: 15pt, weight: 700, fill: heading-ink, ..lines(1.2), {
    let c = chunk.title.codepoints()
    text(fill: accent, c.slice(0, calc.min(3, c.len())).join())
    c.slice(calc.min(3, c.len())).join()
  })),
  block(width: 100%, height: 0.75pt, fill: rule-ink),
))

#let entry(e, first) = pad-top(if first or e.continued { 0pt } else if e.same_employer { 4pt } else { 7pt }, {
  let meta = e.meta.flatten()
  let dates = meta.filter(x => x.icon == "calendar")
  let rest = meta.filter(x => x.icon == "pin") + meta.filter(x => x.icon not in ("pin", "calendar"))
  if not e.continued and (not blank(e.title) or not blank(e.subtitle) or dates.len() > 0) {
    if not blank(e.title) or not blank(e.subtitle) {
      block(width: 100%, heading(level: 3, text(size: 10.5pt, weight: 400, ..lines(1.32), par({
        if not blank(e.title) { text(weight: 700, fill: heading-ink, link-or(rich(e.title), e.title_url)) }
        if not blank(e.title) and not blank(e.subtitle) { [, ] }
        if not blank(e.subtitle) { text(fill: accent, link-or(rich(e.subtitle), e.subtitle_url)) }
      }))))
    }
    if dates.len() > 0 { block(width: 100%, text(style: "italic", fill: light, dates.map(x => x.text).join(" | "))) }
    if rest.len() > 0 { block(width: 100%, text(style: "italic", fill: light, rest.map(x => link-or([#x.text], x.href)).join([#" | "]))) }
  }
  for p in e.paragraphs { pad-top(2pt, rich(p)) }
  for b in e.bullets { pad-top(1.5pt, par(hanging-indent: 10pt, box(width: 10pt, text(fill: accent, [•])) + rich(b))) }
  if not blank(e.tags) { pad-top(2pt, text(fill: muted, rich(e.tags))) }
})

#let text-item(paras) = for (i, p) in paras.enumerate() { pad-top(if i > 0 { 3pt } else { 0pt }, rich(p)) }

// "Group: keyword, keyword" on one line; the colon only when keywords follow.
#let tags(s) = pad-top(2pt, par({
  text(weight: 700, fill: heading-ink, {
    rich(s.name)
    if s.level != none { text(weight: 400, fill: muted, [ · #s.level]) }
    if not blank(s.text) { [:] }
  })
  if not blank(s.text) { [ ] + rich(s.text) }
}))

#let achievement(a, first) = pad-top(if first { 0pt } else { 4pt }, {
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

#let reference(r, first) = pad-top(if first { 0pt } else { 4pt }, {
  for p in r.text { block(width: 100%, text(style: "italic", [“] + rich(p) + [”])) }
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
  if k == "entry" { entry(d, it.first) } else if k == "text" { text-item(d) } else if k == "tags" { tags(d) } else if k == "achievement" {
    achievement(d, it.first)
  } else if k == "language" { language(d, it.first) } else if k == "profiles" { profiles(d) } else if k == "reference" {
    reference(d, it.first)
  } else if k == "line" { line-item(d, it.first) }
})

// CSS: `.section { margin-top: 11pt }`, none for the first in its column.
#let section(chunk, first) = {
  if not first { block(width: 100%, height: 11pt) }
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
  // A cover letter: this page's band and header, then the letter. The letter's 26pt top margin
  // meets .header's 10pt margin.
  if lettering {
    header()
    letter-body(sans, ink, heading-ink, muted, accent, top: 16pt)
  }
  for (pi, p) in data.pages.enumerate() {
    if pi > 0 { pagebreak() }
    if p.header { header() }
    for (ci, chunk) in p.columns.at("main").enumerate() { section(chunk, ci == 0) }
  }
}
