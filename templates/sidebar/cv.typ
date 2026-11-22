// Sidebar: a header across the page, then a side column on a tinted band and the main column
// beside it. Port of cv-builder's templates/sidebar (cv.html.j2 + style.css), box for box.
// Made for people, not resume parsers: the two columns interleave when the text is extracted.
//
// Horizontal geometry (pt from the paper's left edge): 0-26 margin, 26-188 side column, band to
// 214, main column from 238 to the 34pt right margin. The band is the page background, so it
// runs the full height of every page; pages after the first carry "n / total" bottom right.

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#33383f")
#let heading-ink = rgb("#16191d")
#let muted = rgb("#6c727b")
#let rule-ink = rgb("#d9dde2")
#let band-rule = rgb("#d3d8de")
#let chip-border = rgb("#dadee4")
#let seg-off = rgb("#d6dae0")
#let band = rgb("#eff1f4")
#let meta-ink = "#7a8089"
#let sans = "Inter"
#let display = "Rubik"
#let width = (data.page.width - m.left - m.right) * 1pt
#let side-width = 162pt
#let main-width = width - side-width - 50pt
#let lines(v, family: sans) = line-box(family, lh(v))
// A fixed CSS line height (in pt, never tightened) as a line box for text of `size` pt.
#let fixed-lines(height, size, family: sans) = line-box(family, height / size)

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: sans, size: 10pt, fill: ink, lang: data.lang, region: data.region, hyphenate: false, ..lines(1.4))
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
  nowrap(box(baseline: 1.5pt, icon(c.icon, data.accent, 8.4, stroke: 2.3)) + h(3.5pt) + link-or([#c.text], c.href) + h(10pt), w: c.w, extra: 21.9pt, room: main-width) + [ ]
).join())

#let initials() = {
  let names = data.header.name.split()
  let first = if names.len() > 0 { names.first().codepoints().slice(0, calc.min(1, names.first().codepoints().len())).join() } else { "" }
  let last = if names.len() > 1 { names.last().codepoints().slice(0, calc.min(1, names.last().codepoints().len())).join() } else { "" }
  upper((first, last).join())
}

#let header-side() = align(center, if data.header.photo != none {
  box(width: 108pt, height: 108pt, {
    place(circle(radius: 54pt, fill: white))
    place(dx: 3pt, dy: 3pt, box(width: 102pt, height: 102pt, radius: 51pt, clip: true,
      image(data.header.photo, width: 102pt, height: 102pt, fit: "cover")))
  })
} else if initials() != "" {
  box(width: 80pt, height: 80pt, radius: 40pt, fill: white, align(center + horizon,
    text(font: display, size: 29pt, weight: 500, tracking: 1.5pt, fill: accent, ..fixed-lines(80pt, 29pt, family: display), initials())))
})

#let header-main() = {
  block(width: 100%, heading(level: 1, text(font: display, size: 23pt, weight: 500, tracking: -0.2pt, fill: heading-ink, ..lines(1.12, family: display), data.header.name)))
  if not blank(data.header.label) {
    pad-top(4pt, text(size: 11.5pt, weight: 500, fill: accent, ..lines(1.32), rich(data.header.label)))
  }
  if data.header.contacts.len() > 0 {
    pad-top(8pt, text(size: 10pt, fill: ink, ..lines(1.75), contacts(data.header.contacts)))
  }
}

#let header() = block(width: 100%, inset: (bottom: 22pt),
  grid(columns: (side-width, 1fr), column-gutter: 50pt, align: horizon, header-side(), header-main()))

// ------------------------------------------------------------------ items

// Small spaced capitals with a hairline to the column's edge; a step smaller on the band.
#let section-title(chunk, side) = block(width: 100%, inset: (bottom: 7pt), grid(
  columns: (auto, 1fr),
  column-gutter: 8pt,
  align: horizon,
  {
    let size = if side { 10pt } else { 11pt }
    heading(level: 2, text(font: display, size: size, weight: 500, tracking: if side { 1.3pt } else { 1.4pt }, fill: heading-ink,
      ..fixed-lines(13pt, size, family: display), upper(chunk.title)))
  },
  block(width: 100%, height: 0.75pt, fill: if side { band-rule } else { rule-ink }),
))

#let meta-item(x) = {
  let label = if x.icon == "link" { x.text.replace("https://", "").replace("http://", "") } else { x.text }
  nowrap(box(baseline: 1.3pt, icon(x.icon, meta-ink, 8, stroke: 2.2)) + h(3pt) + link-or([#label], x.href), w: x.w, extra: 11pt, room: side-width)
}
#let meta-items(list) = list.enumerate().map(((i, x)) => meta-item(x) + if i < list.len() - 1 { h(9pt) } else { [] }).join()

#let entry-body(e) = {
  for p in e.paragraphs { pad-top(3pt, rich(p)) }
  if e.bullets.len() > 0 {
    pad-top(if e.continued { 0pt } else { 2.5pt }, for b in e.bullets {
      block(width: 100%, inset: (top: 1pt, left: 10pt), {
        place(dx: -8.5pt, dy: 5.6pt, circle(radius: 1.5pt, fill: accent))
        rich(b)
      })
    })
  }
  if not blank(e.tags) { pad-top(3pt, text(size: 10pt, fill: muted, rich(e.tags))) }
}

#let title-text(e, size, v) = text(size: size, weight: 600, fill: heading-ink, ..lines(v), link-or(rich(e.title), e.title_url))
#let subtitle-text(e) = text(size: 10pt, weight: 600, fill: accent, ..lines(1.35), link-or(rich(e.subtitle), e.subtitle_url))

// Main column: the dates on the right of the title, the place on the right of the employer.
#let entry-main(e) = {
  let meta = e.meta.flatten()
  let when = meta.filter(x => x.icon == "calendar")
  let place-items = meta.filter(x => x.icon == "pin")
  let other = meta.filter(x => x.icon not in ("calendar", "pin"))
  let aside(list) = pad(left: 10pt, text(size: 10pt, fill: muted, ..lines(1.4), meta-items(list)))
  if not blank(e.title) or when.len() > 0 {
    baseline-row(
      if blank(e.title) { [] } else { block(width: 100%, heading(level: 3, title-text(e, 11pt, 1.3))) }, 1.3, 11,
      if when.len() > 0 { aside(when) } else { [] }, 1.4, 10,
    )
  }
  if not blank(e.subtitle) or place-items.len() > 0 {
    pad-top(0.5pt, baseline-row(
      if blank(e.subtitle) { [] } else { block(width: 100%, subtitle-text(e)) }, 1.35, 10,
      if place-items.len() > 0 { aside(place-items) } else { [] }, 1.4, 10,
    ))
  }
  if other.len() > 0 { pad-top(1pt, text(size: 10pt, fill: muted, meta-items(other))) }
  entry-body(e)
}

// Side column: everything stacked, a step smaller.
#let entry-side(e) = {
  if not blank(e.title) { block(width: 100%, heading(level: 3, title-text(e, 10pt, 1.32))) }
  if not blank(e.subtitle) { block(width: 100%, subtitle-text(e)) }
  for line in e.meta { pad-top(1.5pt, text(size: 10pt, fill: muted, meta-items(line))) }
  entry-body(e)
}

#let text-item(paras) = for (i, p) in paras.enumerate() { pad-top(if i > 0 { 4pt } else { 0pt }, rich(p)) }

// Skills as chips; the keywords come one by one from the CV.
#let tags(s) = {
  block(width: 100%, heading(level: 3, text(size: 10pt, weight: 600, fill: heading-ink, ..lines(1.3), {
    rich(s.name)
    if s.level != none { text(weight: 500, fill: muted, [ · #s.level]) }
  })))
  if s.keywords.len() > 0 {
    pad-top(3.5pt, par(s.keywords.map(k => box(inset: (right: 2.6pt, bottom: 2.6pt),
      box(stroke: 0.6pt + chip-border, radius: 2.5pt, fill: white, inset: (top: 1.9pt, bottom: 2.1pt, x: 5.1pt),
        text(size: 10pt, fill: ink, ..lines(1.2), k)))).join()))
  }
}

// An icon (11pt, 1pt down) in a line box of its own beside a block.
#let icon-row(name, body) = {
  let (asc, desc) = data.metrics.at(sans)
  let half = (lh(1.4) - asc - desc) / 2 * 10pt
  let above = calc.max(11pt, asc * 10pt + half)
  grid(columns: (17pt, 1fr),
    block(width: 17pt, height: 1pt + above + desc * 10pt + half, place(dy: 1pt + above - 11pt, icon(name, data.accent, 11))),
    body)
}

#let strong-title(body) = text(weight: 600, fill: heading-ink, body)

#let achievement(a) = icon-row(a.icon, {
  block(width: 100%, heading(level: 3, strong-title(rich(a.title))))
  if not blank(a.summary) { pad-top(1.5pt, rich(a.summary)) }
})

#let language(l) = {
  grid(columns: (1fr, auto), block(width: 100%, heading(level: 3, strong-title(l.name))),
    if l.fluency != none { text(size: 10pt, fill: muted, l.fluency) } else { [] })
  if l.rating > 0 {
    pad-top(3pt, grid(columns: (1fr,) * 5, column-gutter: 2.5pt,
      ..range(5).map(i => block(width: 100%, height: 3pt, radius: 1.5pt, fill: if i < l.rating { accent } else { seg-off }))))
  }
}

#let profiles(items) = for (i, p) in items.enumerate() {
  pad-top(if i > 0 { 5pt } else { 0pt }, icon-row(p.icon, {
    block(width: 100%, heading(level: 3, strong-title(p.network)))
    block(width: 100%, text(size: 10pt, fill: muted, link-or([#p.text], p.href)))
  }))
}

#let reference(r) = {
  for p in r.text { block(width: 100%, text(font: "Noto Serif", style: "italic", size: 10pt, [“] + rich(p) + [”])) }
  pad-top(2pt, strong-title(r.name))
}

#let line-item(l) = {
  block(width: 100%, par({
    text(weight: 600, fill: heading-ink, link-or(rich(l.title), l.url))
    if not blank(l.detail) { [, ] + rich(l.detail) }
  }))
  if not blank(l.summary) { pad-top(1pt, rich(l.summary)) }
}

// The space above an item, as the original's selectors decide it (their specificity included:
// on the band, `.col--side .item + .item` beats the per-kind gaps).
#let item-gap(kind, it, titled, side) = {
  if it.first { return 0pt }
  if kind == "entry" { return 0pt }
  if side { return 7pt }
  if kind == "tags" or kind == "line" { 4pt } else if kind == "language" { 6pt } else { 9pt }
}

#let item(chunk, it, titled, side) = block(width: 100%, breakable: it.overflow, {
  let d = it.data
  let k = chunk.kind
  pad-top(item-gap(k, it, titled, side), {
    if k == "entry" {
      let gap = if (it.first and titled) or d.continued { 0pt } else if side { 8pt } else { 9pt }
      pad-top(gap, {
        // "continued" sits at the end of a repeated title, over its hairline; no height.
        if d.continued and it.first and titled {
          place(top + right, dy: -19pt, box(fill: if side { band } else { white }, inset: (left: 6pt),
            text(size: 10pt, fill: muted, ..fixed-lines(12pt, 10pt), data.words.continued)))
        }
        if side { entry-side(d) } else { entry-main(d) }
      })
    } else if k == "text" { text-item(d) } else if k == "tags" { tags(d) } else if k == "achievement" { achievement(d) } else if k == "language" {
      language(d)
    } else if k == "profiles" { profiles(d) } else if k == "reference" { reference(d) } else if k == "line" { line-item(d) }
  })
})

// CSS: `.section { margin-top: 17pt }`, none for the first in its column.
#let section(chunk, first, side) = {
  if not first { block(width: 100%, height: 17pt) }
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

#let columns(page) = grid(columns: (side-width, 1fr), column-gutter: 50pt,
  block(width: side-width, column(page.columns.at("side"), true)), block(width: main-width, column(page.columns.at("main"), false)))

// ------------------------------------------------------------------ document

#if measuring {
  set page(width: data.page.width * 1pt, height: auto, margin: 0pt)
  block(width: width, {
    header()
    mark("c")
    columns(data.pages.at(0))
  })
  [#metadata((text: 10, line: lh(1.4) * 10, width: main-width / 1pt))<cvforge-measure>]
} else {
  set page(
    width: data.page.width * 1pt,
    height: data.page.height * 1pt,
    margin: (top: m.top * 1pt, right: m.right * 1pt, bottom: m.bottom * 1pt, left: m.left * 1pt),
    background: place(top + left, rect(width: 214pt, height: 100%, fill: band, stroke: none)),
  )
  for (pi, p) in data.pages.enumerate() {
    if pi > 0 { pagebreak() }
    // "n / total" centred in the bottom margin, right-aligned; none on the first page. The
    // builder knows the page count, so no counter (a counter makes Typst lay out twice).
    if pi > 0 {
      place(bottom + right, dy: m.bottom * 0.5pt + 6.05pt, text(size: 10pt, fill: rgb("#8a9099"), top-edge: "ascender", bottom-edge: "descender",
        [#(pi + 1) / #data.pages.len()]))
    }
    if p.header { header() }
    columns(p)
  }
}
