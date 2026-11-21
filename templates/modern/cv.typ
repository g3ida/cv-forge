// Modern: one column under a full-width accent band with the name and a round photo, with
// accent-marked section titles. Port of cv-builder's templates/modern (cv.html.j2 + style.css).
//
// Reads in order for resume parsers: no floats or positioned boxes; separators and bullets are
// real text; no letter-spacing, hyphenation or ligatures. The band reaches the paper's edges
// (negative padding equal to the page margins); the measured header height is what the band
// takes below the page margin, as in the original.

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#2c3038")
#let heading-ink = rgb("#111419")
#let muted = rgb("#5c636e")
#let faint = rgb("#a3a9b2")
#let rule-ink = rgb("#dde1e6")
// The contacts strip: a 92% white veil over the accent colour.
#let tint = color.mix((accent, 8%), (white, 92%), space: rgb)
#let sans = "Inter"
#let width = (data.page.width - m.left - m.right) * 1pt
#let lines(v) = line-box(sans, lh(v))

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: sans, size: 10pt, fill: ink, lang: "en", ligatures: false, hyphenate: false, ..lines(1.37))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
#show heading: it => it.body

// Separators are real text (" | "); the 2pt on each side only widens the gap on the page.
#let sep(t) = h(2pt) + text(fill: faint, t) + h(2pt)

// ------------------------------------------------------------------ header

#let photo() = box(width: 64pt, height: 64pt, {
  place(circle(radius: 32pt, fill: white))
  place(dx: 2pt, dy: 2pt, box(width: 60pt, height: 60pt, radius: 30pt, clip: true,
    image(data.header.photo, width: 60pt, height: 60pt, fit: "cover")))
})

#let band-text() = {
  block(width: 100%, heading(level: 1, text(size: 28pt, weight: 700, fill: white, ..lines(1.15), data.header.name)))
  if not blank(data.header.label) {
    pad-top(6pt, text(size: 13pt, fill: white.transparentize(10%), ..lines(1.3), rich(data.header.label)))
  }
}

#let contacts(items) = par(items.enumerate().map(((i, c)) => {
  let last = i == items.len() - 1
  nowrap(link-or([#c.text], c.href) + if last { [] } else { sep[#" |"] }, w: c.w + 1, extra: 4pt, room: width) + if last { [] } else { [ ] }
}).join())

#let header() = block(width: 100%, inset: (bottom: 16pt), pad(top: -m.top * 1pt, left: -m.left * 1pt, right: -m.right * 1pt, {
  block(width: 100%, fill: accent, inset: (top: 24pt, right: m.right * 1pt, bottom: 21pt, left: m.left * 1pt),
    if data.header.photo == none { band-text() } else {
      grid(columns: (1fr, 64pt), column-gutter: 18pt, align: horizon, band-text(), photo())
    })
  if data.header.contacts.len() > 0 {
    block(width: 100%, fill: tint, inset: (y: 7pt, right: m.right * 1pt, left: m.left * 1pt),
      text(size: 10pt, fill: ink, ..lines(1.5), contacts(data.header.contacts)))
  }
}))

// ------------------------------------------------------------------ items

// A small square, the title, and a hairline to the right edge (no text but the title).
#let section-title(chunk) = block(width: 100%, inset: (bottom: 6pt), grid(
  columns: (7pt, auto, 1fr),
  column-gutter: (7pt, 9pt),
  align: horizon,
  box(width: 7pt, height: 7pt, radius: 1.5pt, fill: accent),
  heading(level: 2, text(size: 13pt, weight: 700, fill: accent, ..lines(1.25), chunk.title)),
  block(width: 100%, height: 0.75pt, fill: rule-ink),
))

#let entry(e, first) = pad-top(if first or e.continued { 0pt } else { 9pt }, {
  if not blank(e.title) {
    block(width: 100%, heading(level: 3, text(size: 11pt, weight: 600, fill: heading-ink, ..lines(1.3), link-or(rich(e.title), e.title_url))))
  }
  if not blank(e.subtitle) {
    block(width: 100%, text(weight: 600, fill: accent, link-or(rich(e.subtitle), e.subtitle_url)))
  }
  let meta = e.meta.flatten()
  let dates = meta.filter(x => x.icon == "calendar")
  let rest = meta.filter(x => x.icon == "pin") + meta.filter(x => x.icon not in ("pin", "calendar"))
  if dates.len() > 0 {
    pad-top(0.5pt, text(size: 10pt, weight: 500, fill: muted, ..lines(1.4), dates.map(x => x.text).join(" | ")))
  }
  if rest.len() > 0 {
    block(width: 100%, text(size: 10pt, fill: muted, ..lines(1.4), rest.map(x => link-or([#x.text], x.href)).join(sep[#" | "])))
  }
  for p in e.paragraphs { pad-top(3pt, rich(p)) }
  for b in e.bullets { pad-top(2pt, par(hanging-indent: 10pt, box(width: 10pt, text(fill: accent)[•]) + rich(b))) }
  if not blank(e.tags) { pad-top(2pt, text(fill: muted, rich(e.tags))) }
})

#let text-item(paras) = for (i, p) in paras.enumerate() { pad-top(if i > 0 { 3pt } else { 0pt }, rich(p)) }

// "Group: keyword, keyword"; no gap right under the section title.
#let tags(s, under-title) = pad-top(if under-title { 0pt } else { 2.5pt }, par({
  text(weight: 600, fill: heading-ink, {
    rich(s.name)
    if s.level != none { text(weight: 400, fill: muted, [ · #s.level]) }
    if not blank(s.text) { [:] }
  })
  if not blank(s.text) { [ ] + rich(s.text) }
}))

#let achievement(a, first) = pad-top(if first { 0pt } else { 5pt }, {
  block(width: 100%, heading(level: 3, text(weight: 600, fill: heading-ink, rich(a.title))))
  if not blank(a.summary) { block(width: 100%, rich(a.summary)) }
})

#let language(l, first) = pad-top(if first { 0pt } else { 2.5pt }, par({
  text(weight: 600, fill: heading-ink, l.name)
  if l.fluency != none { [: #l.fluency] } else if l.rating > 0 { [: #str(l.rating)/5] }
}))

#let profiles(items) = for (i, p) in items.enumerate() {
  pad-top(if i > 0 { 2.5pt } else { 0pt }, par({
    text(weight: 600, fill: heading-ink, p.network)
    [: ]
    link-or([#p.text], p.href)
  }))
}

#let reference(r, first) = pad-top(if first { 0pt } else { 5pt }, {
  for p in r.text { block(width: 100%, text(fill: ink, [“] + rich(p) + [”])) }
  pad-top(2pt, text(weight: 600, fill: heading-ink, r.name))
})

#let line-item(l, first) = pad-top(if first { 0pt } else { 3pt }, {
  block(width: 100%, par({
    text(weight: 600, fill: heading-ink, link-or(rich(l.title), l.url))
    if not blank(l.detail) { [, ] + rich(l.detail) }
  }))
  if not blank(l.summary) { pad-top(1pt, rich(l.summary)) }
})

#let item(chunk, it, titled) = block(width: 100%, breakable: it.overflow, {
  let d = it.data
  let k = chunk.kind
  if k == "entry" { entry(d, it.first) } else if k == "text" { text-item(d) } else if k == "tags" {
    tags(d, it.first and titled)
  } else if k == "achievement" { achievement(d, it.first) } else if k == "language" { language(d, it.first) } else if k == "profiles" {
    profiles(d)
  } else if k == "reference" { reference(d, it.first) } else if k == "line" { line-item(d, it.first) }
})

// CSS: `.section { margin-top: 14pt }`, none for the first in its column.
#let section(chunk, first) = {
  if not first { block(width: 100%, height: 14pt) }
  let titled = data.repeat_titles or not chunk.continued
  block(width: 100%, {
    if measuring { mark("s", chunk.id) }
    if titled { section-title(chunk) }
    for it in chunk.items {
      if measuring { mark("i") }
      item(chunk, it, titled)
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
  [#metadata((text: 10, line: lh(1.37) * 10, width: width / 1pt))<cvforge-measure>]
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
