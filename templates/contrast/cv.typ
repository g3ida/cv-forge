// Contrast: the main column on white and a side column on a dark band down the right edge of
// every page. Port of cv-builder's templates/contrast (cv.html.j2 + style.css), box for box.
// Made for people, not resume parsers: the two columns interleave when the text is extracted.
//
// Horizontal geometry (pt from the paper's right edge): 0-30 margin, 30-180 side column, band to
// 198, main column from 222 to the left margin. The band is the page background, so it runs the
// full height of every page.

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let band = rgb("#222d36")
#let ink = rgb("#30363b")
#let heading-ink = rgb("#14191d")
#let muted = rgb("#6f777e")
#let rule-ink = rgb("#dfe3e6")
#let side-text = rgb("#dde2e6")
#let side-soft = rgb("#a8b2ba")
#let side-heading = white
#let chip-border = rgb("#46535e")
#let seg-off = rgb("#3d4954")
#let subtitle-ink = rgb("#4a5661")
#let meta-ink = "#7d858c"
#let sans = "Rubik"
#let width = (data.page.width - m.left - m.right) * 1pt
#let side-width = 150pt
#let gap = 42pt
#let main-width = width - side-width - gap
#let lines(v, family: sans) = line-box(family, lh(v))
#let fixed-lines(height, size, family: sans) = line-box(family, height / size)

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: sans, size: 10pt, fill: ink, lang: data.lang, region: data.region, hyphenate: false, ..lines(1.42))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
#show heading: it => it.body

// Where the baseline sits in a CSS line box of line height `v` for `size` pt text.
#let baseline(v, size) = {
  let (asc, desc) = data.metrics.at(sans)
  ((v - asc - desc) / 2 + asc) * size * 1pt
}

// `display: flex; align-items: baseline; justify-content: space-between`: the left part takes
// the room, the right part keeps to one line; their first baselines line up.
#let baseline-row(left, lv, ls, right, rv, rs) = {
  let d = baseline(lh(lv), ls) - baseline(lh(rv), rs)
  grid(columns: (1fr, auto), pad(top: calc.max(-d, 0pt), left), pad(top: calc.max(d, 0pt), right))
}

// ------------------------------------------------------------------ header

#let contacts(items) = par(items.map(c =>
  nowrap(box(baseline: 1.5pt, icon(c.icon, data.accent, 8.4, stroke: 2.3)) + h(4pt) + link-or([#c.text], c.href) + h(10pt), w: c.w, extra: 22.4pt, room: main-width) + [ ]
).join())

#let initials() = {
  let names = data.header.name.split()
  let first = if names.len() > 0 { names.first().codepoints().slice(0, calc.min(1, names.first().codepoints().len())).join() } else { "" }
  let last = if names.len() > 1 { names.last().codepoints().slice(0, calc.min(1, names.last().codepoints().len())).join() } else { "" }
  upper((first, last).join())
}

#let header-main() = pad-top(6pt, {
  block(width: 100%, heading(level: 1, text(size: 26pt, weight: 700, fill: heading-ink, ..lines(1.1), data.header.name)))
  block(width: 100%, height: 8pt)
  block(width: 34pt, height: 3pt, fill: accent)
  if not blank(data.header.label) {
    pad-top(9pt, text(size: 12pt, weight: 500, fill: heading-ink, ..lines(1.3), rich(data.header.label)))
  }
  if data.header.contacts.len() > 0 {
    pad-top(8pt, text(..line-box(sans, 1.75), contacts(data.header.contacts)))
  }
})

#let header-side() = if data.header.photo != none {
  box(width: side-width, height: side-width, clip: true, image(data.header.photo, width: side-width, height: side-width, fit: "cover"))
} else if initials() != "" {
  block(width: side-width, {
    box(width: side-width, height: 96pt, align(center + horizon,
      text(size: 40pt, weight: 500, fill: side-heading, ..fixed-lines(96pt, 40pt), initials())))
    block(width: side-width, height: 3pt, fill: accent)
  })
}

#let header() = block(width: 100%, inset: (bottom: 22pt),
  grid(columns: (1fr, side-width), column-gutter: gap, align: top, header-main(), header-side()))

// ------------------------------------------------------------------ items

// Main column: a heading with an accent bar on its left; on the band, the heading in the accent
// over a hairline.
#let section-title(chunk, side) = if side {
  block(width: 100%, inset: (bottom: 6pt), {
    block(width: 100%, inset: (bottom: 4pt), heading(level: 2, text(size: 11.5pt, weight: 500, fill: accent, ..lines(1.2), chunk.title)))
    rule(0.75pt, chip-border)
  })
} else {
  block(width: 100%, inset: (bottom: 6pt), grid(
    columns: (3pt, 1fr),
    column-gutter: 8pt,
    block(width: 3pt, height: lh(1.2) * 13pt, fill: accent),
    heading(level: 2, text(size: 13pt, weight: 500, fill: heading-ink, ..lines(1.2), chunk.title)),
  ))
}

#let meta-item(x) = {
  let label = x.text.replace("https://", "")
  nowrap(box(baseline: 1.3pt, icon(x.icon, meta-ink, 8, stroke: 2.2)) + h(3pt) + link-or([#label], x.href) + h(9pt), w: x.w, extra: 20pt, room: main-width)
}

#let entry-body(e, side) = {
  for p in e.paragraphs { pad-top(3pt, rich(p)) }
  if e.bullets.len() > 0 {
    pad-top(2pt, for b in e.bullets {
      block(width: 100%, inset: (top: 1pt, left: 11pt), {
        place(dx: -10pt, dy: 6pt, rect(width: 5pt, height: 1.5pt, fill: accent, stroke: none))
        rich(b)
      })
    })
  }
  if not blank(e.tags) { pad-top(3pt, text(fill: if side { side-soft } else { muted }, rich(e.tags))) }
}

// Main column: the dates on the right of the title, the place on the right of the employer.
#let entry-main(e) = {
  let meta = e.meta.flatten()
  let when = meta.filter(x => x.icon == "calendar")
  let where = meta.filter(x => x.icon == "pin")
  let other = meta.filter(x => x.icon not in ("calendar", "pin"))
  let aside(list) = pad(left: 10pt, text(fill: muted, ..lines(1.42), list.map(x => nowrap([#x.text.replace(" - ", " – ")], w: x.w, room: main-width)).join()))
  if not e.continued {
    if not blank(e.title) or when.len() > 0 {
      baseline-row(
        if blank(e.title) { [] } else { block(width: 100%, heading(level: 3, text(size: 11pt, weight: 500, fill: heading-ink, ..lines(1.3), link-or(rich(e.title), e.title_url)))) }, 1.3, 11,
        if when.len() > 0 { aside(when) } else { [] }, 1.42, 10,
      )
    }
    if not blank(e.subtitle) or where.len() > 0 {
      baseline-row(
        if blank(e.subtitle) { [] } else { block(width: 100%, text(fill: subtitle-ink, link-or(rich(e.subtitle), e.subtitle_url))) }, 1.42, 10,
        if where.len() > 0 { aside(where) } else { [] }, 1.42, 10,
      )
    }
    if other.len() > 0 { block(width: 100%, text(fill: muted, par(other.map(meta-item).join()))) }
  }
  entry-body(e, false)
}

// Side column: stacked, light on the band.
#let entry-side(e) = {
  if not e.continued {
    if not blank(e.title) { block(width: 100%, heading(level: 3, text(size: 10pt, weight: 500, fill: side-heading, ..lines(1.3), link-or(rich(e.title), e.title_url)))) }
    if not blank(e.subtitle) { block(width: 100%, text(fill: side-soft, link-or(rich(e.subtitle), e.subtitle_url))) }
    for row in e.meta { block(width: 100%, text(fill: side-soft, row.map(x => link-or([#x.text], x.href)).join([#" · "]))) }
  }
  entry-body(e, true)
}

#let text-item(paras) = for (i, p) in paras.enumerate() { pad-top(if i > 0 { 4pt } else { 0pt }, rich(p)) }

#let strong-title(body, side) = text(weight: 500, fill: if side { side-heading } else { heading-ink }, body)
#let soft(body, side) = text(fill: if side { side-soft } else { muted }, body)

// Skills as chips; the keywords come one by one from the CV.
#let tags(s, side) = {
  block(width: 100%, heading(level: 3, strong-title({
    rich(s.name)
    if s.level != none { text(weight: 400, fill: if side { side-soft } else { muted }, [ · #s.level]) }
  }, side)))
  if s.keywords.len() > 0 {
    pad-top(3pt, par(s.keywords.map(k => box(inset: (right: 2.6pt, bottom: 2.6pt),
      box(stroke: 0.6pt + (if side { chip-border } else { rule-ink }), radius: 2pt, inset: (top: 1.8pt, bottom: 2pt, x: 5.1pt),
        text(size: 10pt, fill: if side { side-text } else { ink }, ..lines(1.2), k)))).join()))
  }
}

#let achievement(a, side) = {
  block(width: 100%, heading(level: 3, strong-title(rich(a.title), side)))
  if not blank(a.summary) { block(width: 100%, soft(rich(a.summary), side)) }
}

#let language(l, side) = {
  grid(columns: (1fr, auto), block(width: 100%, heading(level: 3, strong-title(l.name, side))),
    if l.fluency != none { soft(l.fluency, side) } else { [] })
  if l.rating > 0 {
    pad-top(3pt, grid(columns: (1fr,) * 5, column-gutter: 2.5pt,
      ..range(5).map(i => block(width: 100%, height: 3pt, fill: if i < l.rating { accent } else if side { seg-off } else { rule-ink }))))
  }
}

#let profiles(items, side) = for (i, p) in items.enumerate() {
  pad-top(if i > 0 { 5pt } else { 0pt }, {
    block(width: 100%, heading(level: 3, strong-title(p.network, side)))
    block(width: 100%, soft(link-or([#p.text], p.href), side))
  })
}

#let reference(r, side) = {
  for p in r.text { block(width: 100%, text(font: "Noto Serif", style: "italic", ..line-box("Noto Serif", lh(1.42)), [“] + rich(p) + [”])) }
  pad-top(2pt, strong-title(r.name, side))
}

// Certificates and awards, one line each: "Name, Issuer, 2026".
#let line-item(l, side) = {
  block(width: 100%, par({
    strong-title(link-or(rich(l.title), l.url), side)
    if not blank(l.detail) { [, ] + rich(l.detail) }
  }))
  if not blank(l.summary) { pad-top(1pt, soft(rich(l.summary), side)) }
}

// The space above an item: `.item + .item` 7pt, none between entries, and later rules for tags
// (5pt), languages (6pt) and lines (4pt).
#let item-gap(kind, first) = if first or kind == "entry" { 0pt } else if kind == "tags" { 5pt } else if kind == "language" { 6pt } else if kind == "line" { 4pt } else { 7pt }

#let item(chunk, it, titled, side) = block(width: 100%, breakable: it.overflow, pad-top(item-gap(chunk.kind, it.first), {
  let d = it.data
  let k = chunk.kind
  if k == "entry" {
    let gap = if (it.first and titled) or d.continued { 0pt } else if d.same_employer and not side { 6pt } else { 10pt }
    pad-top(gap, if side { entry-side(d) } else { entry-main(d) })
  } else if k == "text" { text-item(d) } else if k == "tags" { tags(d, side) } else if k == "achievement" { achievement(d, side) } else if k == "language" {
    language(d, side)
  } else if k == "profiles" { profiles(d, side) } else if k == "reference" { reference(d, side) } else if k == "line" { line-item(d, side) }
}))

// CSS: `.section { margin-top: 16pt }`, none for the first in its column.
#let section(chunk, first, side) = {
  if not first { block(width: 100%, height: 16pt) }
  let titled = data.repeat_titles or not chunk.continued
  block(width: 100%, {
    if measuring { mark("s", chunk.id) }
    if titled { section-title(chunk, side) }
    for it in chunk.items {
      if measuring { mark("i") }
      item(chunk, it, titled, side)
      if measuring { mark("j") }
    }
  })
}

#let column(chunks, side) = for (ci, chunk) in chunks.enumerate() { section(chunk, ci == 0, side) }

#let columns(page) = grid(columns: (main-width, side-width), column-gutter: gap,
  block(width: main-width, column(page.columns.at("main"), false)),
  block(width: side-width, { set text(fill: side-text); column(page.columns.at("side"), true) }))

// ------------------------------------------------------------------ document

#if measuring {
  set page(width: data.page.width * 1pt, height: auto, margin: 0pt)
  block(width: width, {
    header()
    mark("c")
    columns(data.pages.at(0))
  })
  [#metadata((text: 10, line: lh(1.42) * 10, width: main-width / 1pt))<cvforge-measure>]
} else {
  set page(
    width: data.page.width * 1pt,
    height: data.page.height * 1pt,
    margin: (top: m.top * 1pt, right: m.right * 1pt, bottom: m.bottom * 1pt, left: m.left * 1pt),
    background: place(top + right, rect(width: 198pt, height: 100%, fill: band, stroke: none)),
  )
  for (pi, p) in data.pages.enumerate() {
    if pi > 0 { pagebreak() }
    if p.header { header() }
    columns(p)
  }
}
