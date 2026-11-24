// Timeline: the dates in a left gutter, a thin line with a dot per role, and all text on one
// edge. Port of cv-builder's templates/timeline (cv.html.j2 + style.css), box for box.
//
// Horizontal grid from the left margin: 0-112pt gutter (dates and place, right-aligned; skill
// groups, languages, networks), 112-128pt rail (line and dots on its axis at 120pt), 128pt the
// content edge. Made for people: pdftotext reads the gutter apart from the roles beside it.

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#2b2e33")
#let heading-ink = rgb("#15181c")
#let muted = rgb("#646a72")
#let faint = rgb("#8a9098")
#let line-ink = rgb("#c3c8ce")
#let rule-ink = rgb("#d3d7dc")
#let sans = "Inter"
#let display = "Noto Serif Display"
#let width = (data.page.width - m.left - m.right) * 1pt
#let lines(v, family: sans) = line-box(family, lh(v))

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: sans, size: 10pt, fill: ink, lang: data.lang, region: data.region, ligatures: false, hyphenate: false, ..lines(1.38))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
#show heading: it => it.body

// The gutter (112pt) and the rail (16pt) before the content edge at 128pt.
#let gutter(label, body) = grid(columns: (112pt, 1fr), column-gutter: 16pt, label, body)
#let content-edge(body) = block(width: 100%, inset: (left: 128pt), body)

// ------------------------------------------------------------------ header

#let contacts(items) = par(items.enumerate().map(((i, c)) => {
  let last = i == items.len() - 1
  nowrap(link-or([#c.text], c.href) + if last { [] } else { text(fill: faint)[#" ·"] }, w: c.w + 1, room: width - 80pt) + if last { [] } else { [ ] }
}).join())

#let header-text() = {
  block(width: 100%, heading(level: 1, text(font: display, size: 26.5pt, weight: 400, fill: heading-ink, ..lines(1.15, family: display), data.header.name)))
  if not blank(data.header.label) {
    pad-top(5pt, text(size: 11.5pt, weight: 500, fill: accent, ..lines(1.3), rich(data.header.label)))
  }
  if data.header.contacts.len() > 0 {
    pad-top(6pt, text(size: 10pt, fill: muted, contacts(data.header.contacts)))
  }
}

#let header() = block(width: 100%, inset: (bottom: 16pt), {
  block(width: 100%, inset: (bottom: 13pt), if data.header.photo == none { header-text() } else {
    grid(columns: (1fr, 64pt), column-gutter: 16pt, header-text(),
      box(width: 64pt, height: 64pt, radius: 32pt, clip: true, image(data.header.photo, width: 64pt, height: 64pt, fit: "cover")))
  })
  rule(0.6pt, rule-ink)
})

// ------------------------------------------------------------------ items

// A small square on the axis side, the title, and a hairline to the right margin.
#let section-title(chunk) = block(width: 100%, inset: (bottom: 7pt), grid(
  columns: (91.5pt, 5pt, auto, 1fr),
  column-gutter: (0pt, 5.5pt, 9pt),
  align: horizon,
  [],
  box(width: 5pt, height: 5pt, fill: accent),
  heading(level: 2, text(font: display, size: 13pt, weight: 400, fill: accent, ..lines(1.2, family: display), chunk.title)),
  block(width: 100%, height: 0.6pt, fill: rule-ink),
))

// An entry: dates and place in the gutter, the dot on the axis, title, employer and body on the
// content edge. The line is the left border of the content side, through the space above the
// entry too, so pieces of a split entry join; under a section title it starts at the dot.
#let entry(e, first, under-title) = {
  let pad = if first or e.continued { 0pt } else { 11pt }
  let meta = e.meta.flatten()
  let dates = meta.filter(x => x.icon == "calendar")
  let places = meta.filter(x => x.icon == "pin")
  let rest = meta.filter(x => x.icon not in ("pin", "calendar"))
  let current = e.current
  let when = block(width: 112pt, inset: (top: pad), align(right, {
    if dates.len() > 0 {
      pad-top(2.2pt, text(size: 10pt, weight: 500, number-width: "tabular", fill: muted, ..lines(1.3),
        box(dates.map(x => x.text.replace(" - ", " – ")).join(" | "))))
    }
    if places.len() > 0 { pad-top(1pt, text(size: 10pt, fill: faint, ..lines(1.3), places.map(x => x.text).join(" | "))) }
  }))
  let body = {
    if rest.len() > 0 {
      block(width: 100%, text(fill: muted, rest.enumerate().map(((i, x)) => {
        let sep = if i > 0 { [ ] + h(8pt) } else { [] }
        sep + nowrap(box(baseline: 0.9pt, icon(x.icon, "#8a9098", 7.4, stroke: 2.2)) + h(3pt) + link-or([#x.text], x.href), w: x.w, extra: 10.4pt, room: width - 128pt)
      }).join()))
    }
    for p in e.paragraphs { pad-top(2.5pt, rich(p)) }
    for b in e.bullets { pad-top(2pt, par(hanging-indent: 9pt, box(width: 9pt, text(fill: accent)[•]) + rich(b))) }
    if not blank(e.tags) { pad-top(2.5pt, text(fill: muted, rich(e.tags))) }
  }
  let side = block(width: 100%, stroke: (left: 0.75pt + line-ink), inset: (top: pad, left: 8pt), {
    if under-title { place(top + left, dx: -8.5pt, rect(width: 1pt, height: 6pt, fill: white, stroke: none)) }
    if not e.continued {
      place(top + left, dx: -11.5pt, dy: 3.4pt, circle(radius: 2.75pt, stroke: 1.5pt + accent, fill: if current { accent } else { white }))
      if not blank(e.title) {
        block(width: 100%, heading(level: 3, text(size: 11pt, weight: 600, fill: heading-ink, ..lines(1.3), link-or(rich(e.title), e.title_url))))
      }
      if not blank(e.subtitle) { block(width: 100%, text(weight: 500, fill: accent, link-or(rich(e.subtitle), e.subtitle_url))) }
    }
    body
  })
  grid(columns: (112pt, 8pt, 1fr), if e.continued { [] } else { when }, [], side)
}

#let text-item(paras) = content-edge(for (i, p) in paras.enumerate() { pad-top(if i > 0 { 4pt } else { 0pt }, rich(p)) })

// Group name in the gutter, keywords on the content edge; without keywords the name sits on
// the content edge.
#let tags(s, first) = pad-top(if first { 0pt } else { 4pt }, {
  let name = text(weight: 600, fill: heading-ink, {
    rich(s.name)
    if s.level != none { text(weight: 400, fill: muted, [ · #s.level]) }
  })
  if blank(s.text) { content-edge(name) } else { gutter(align(right, name), rich(s.text)) }
})

#let achievement(a, first) = pad-top(if first { 0pt } else { 5pt }, grid(
  columns: (115.25pt, 9.5pt, 1fr),
  column-gutter: (0pt, 3.25pt),
  [],
  pad(top: 1pt, icon(a.icon, data.accent, 9.5)),
  {
    block(width: 100%, heading(level: 3, text(weight: 600, fill: heading-ink, rich(a.title))))
    if not blank(a.summary) { pad-top(1pt, rich(a.summary)) }
  },
))

#let dots(n) = box(range(5).map(i => box(width: 6pt, height: 6pt, radius: 3pt, fill: if i < n { accent } else { rule-ink })).join(h(2.5pt)) + h(2.5pt))

#let language(l, first) = pad-top(if first { 0pt } else { 2pt }, gutter(
  align(right, text(weight: 600, fill: heading-ink, l.name)),
  if l.fluency != none { [#l.fluency] } else { dots(l.rating) },
))

#let profiles(items) = for (i, p) in items.enumerate() {
  pad-top(if i > 0 { 2pt } else { 0pt }, gutter(align(right, text(weight: 600, fill: heading-ink, p.network)), link-or([#p.text], p.href)))
}

#let reference(r, first) = pad-top(if first { 0pt } else { 5pt }, content-edge({
  for (i, p) in r.text.enumerate() { pad-top(if i > 0 { 2pt } else { 0pt }, [“] + rich(p) + [”]) }
  pad-top(2pt, {
    if r.text.len() > 0 { text(fill: muted)[– ] }
    text(weight: 600, fill: heading-ink, r.name)
  })
}))

#let line-item(l, first) = pad-top(if first { 0pt } else { 3pt }, content-edge({
  block(width: 100%, par({
    text(weight: 600, fill: heading-ink, link-or(rich(l.title), l.url))
    if not blank(l.detail) { [, ] + rich(l.detail) }
  }))
  if not blank(l.summary) { pad-top(1pt, rich(l.summary)) }
}))

#let item(chunk, it, titled) = block(width: 100%, breakable: it.overflow, {
  let d = it.data
  let k = chunk.kind
  if k == "entry" { entry(d, it.first, it.first and titled) } else if k == "text" { text-item(d) } else if k == "tags" {
    tags(d, it.first)
  } else if k == "achievement" { achievement(d, it.first) } else if k == "language" { language(d, it.first) } else if k == "profiles" {
    profiles(d)
  } else if k == "reference" { reference(d, it.first) } else if k == "line" { line-item(d, it.first) }
})

// CSS: `.section { margin-top: 16pt }`, none for the first in its column.
#let section(chunk, first) = {
  if not first { block(width: 100%, height: 16pt) }
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
  [#metadata((text: 10, line: lh(1.38) * 10, width: width / 1pt))<cvforge-measure>]
} else {
  set page(
    width: data.page.width * 1pt,
    height: data.page.height * 1pt,
    margin: (top: m.top * 1pt, right: m.right * 1pt, bottom: m.bottom * 1pt, left: m.left * 1pt),
  )
  // A cover letter: this page's band and header, then the letter. The letter's 26pt top margin
  // meets .header's 16pt margin.
  if lettering {
    header()
    letter-body(sans, ink, heading-ink, muted, accent, top: 10pt)
  }
  for (pi, p) in data.pages.enumerate() {
    if pi > 0 { pagebreak() }
    if p.header { header() }
    for (ci, chunk) in p.columns.at("main").enumerate() { section(chunk, ci == 0) }
  }
}
