// Compact (formerly two-column): dense two columns with icons, rating dots and a round photo;
// fits the most on a page. Port of cv-builder's templates/compact (cv.html.j2 + style.css), box
// for box. For people: its two columns interleave when a parser reads the text.

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#3e3e3e")
#let heading-ink = rgb("#000000")
#let separator = rgb("#cccccc")
#let dot-off = rgb("#e4e4e4")
#let muted = "#65696d"
#let sans = "Inter"
#let display = "Rubik"
#let width = (data.page.width - m.left - m.right) * 1pt
#let left-width = width * 57.45%
#let right-width = width * 38.39%
#let lines(v, family: sans) = line-box(family, lh(v))

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: sans, size: 10pt, fill: ink, lang: data.lang, region: data.region, hyphenate: false, ..lines(1.23))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
#show heading: it => it.body

// An inline icon: `size` pt, lowered by `drop` (CSS vertical-align), `gap` before the text.
#let inline-icon(name, color, size, stroke, drop, gap) = box(baseline: drop, icon(name, color, size, stroke: stroke)) + h(gap)

// ------------------------------------------------------------------ header

#let contacts(items) = par(items.map(c =>
  nowrap(inline-icon(c.icon, data.accent, 8.4, 2.4, 1.6pt, 1.7pt) + link-or([#c.text], c.href) + h(9.4pt), w: c.w, extra: 19.5pt, room: width - 99.7pt)
).join())

#let header-main() = pad(top: -2pt, {
  block(width: 100%, heading(level: 1, text(font: display, size: 24pt, weight: 500, fill: heading-ink, ..lines(1.15, family: display), upper(data.header.name))))
  if not blank(data.header.label) {
    block(width: 100%, text(font: display, size: 14.5pt, weight: 500, fill: accent, ..lines(1.185, family: display), rich(data.header.label)))
  }
  if data.header.contacts.len() > 0 {
    pad-top(5.2pt, text(size: 10pt, weight: 700, ..lines(1.42), contacts(data.header.contacts)))
  }
})

#let header() = block(width: 100%, inset: (bottom: 9pt), if data.header.photo == none { header-main() } else {
  grid(columns: (1fr, 85.7pt), column-gutter: 14pt, header-main(),
    box(width: 85.7pt, height: 85.7pt, radius: 42.85pt, clip: true, image(data.header.photo, width: 85.7pt, height: 85.7pt, fit: "cover")))
})

// ------------------------------------------------------------------ items

#let section-title(chunk) = block(width: 100%, {
  block(width: 100%, heading(level: 2, text(font: display, size: 15.5pt, weight: 500, fill: heading-ink, ..lines(1.185, family: display), upper(chunk.title))))
  rule(1.9pt, heading-ink)
})

#let strong-title(body) = text(size: 11.5pt, weight: 700, fill: heading-ink, ..lines(1.21), body)

// The entry's blocks one after another, each with its top margin. An empty meta line has no
// height, and its margin collapses with the next block's, as in CSS.
#let stack(parts) = {
  let carry = 0pt
  for (gap, body) in parts {
    if body == none { carry = calc.max(carry, gap) } else {
      pad-top(calc.max(carry, gap), body)
      carry = 0pt
    }
  }
}

#let entry(e) = stack((
  (0pt, if blank(e.title) { none } else {
    block(width: 100%, heading(level: 3, text(font: display, size: 13pt, weight: 500, fill: heading-ink, ..lines(1.185, family: display), link-or(rich(e.title), e.title_url))))
  }),
  (2.1pt, if blank(e.subtitle) { none } else {
    block(width: 100%, text(size: 11.5pt, weight: 700, fill: accent, ..lines(1.21), link-or(rich(e.subtitle), e.subtitle_url)))
  }),
  ..e.meta.map(line => (2.75pt, if line.len() == 0 { none } else {
    block(width: 100%, text(size: 10pt, ..lines(1.21), par(line.map(x =>
      nowrap(inline-icon(x.icon, muted, 8.7, 2.4, 2pt, 2.7pt) + link-or([#x.text], x.href) + h(10.5pt), w: x.w, extra: 21.9pt, room: right-width)
    ).join())))
  })),
  ..e.paragraphs.map(p => (2.55pt, block(width: 100%, rich(p)))),
  (1.9pt, if e.bullets.len() == 0 { none } else {
    block(width: 100%, for b in e.bullets {
      grid(columns: (11.6pt, 1fr), pad(left: 2.5pt)[•], rich(b))
    })
  }),
  (2.55pt, if blank(e.tags) { none } else { block(width: 100%, text(style: "italic", rich(e.tags))) }),
))

#let text-item(paras) = for p in paras { block(width: 100%, rich(p)) }

#let tags(s) = {
  block(width: 100%, heading(level: 3, text(font: display, size: 11.5pt, weight: 700, fill: accent, ..lines(1.185, family: display), {
    rich(s.name)
    if s.level != none { text(weight: 500, fill: ink, [ · #s.level]) }
  })))
  if not blank(s.text) {
    block(width: 100%, inset: (top: 3.6pt, bottom: 2.7pt), text(size: 11pt, ..lines(1.216), rich(s.text)))
  }
}

// An icon beside a block: the icon (15.4pt, nudged 1.8pt right and 1.2pt up) sits in a line
// box of its own, as tall as the icon above the baseline and the body text's descent below.
#let icon-row(name, body) = {
  let v = lh(1.23)
  let below = (v - 1.2109375) / 2 * 10pt + 2.421875pt
  grid(columns: (25.4pt, 1fr),
    block(width: 25.4pt, height: 14.2pt + below, place(dx: 1.8pt, dy: -1.2pt, icon(name, data.accent, 15.4))),
    body)
}

#let achievement(a) = icon-row(a.icon, {
  block(width: 100%, heading(level: 3, strong-title(rich(a.title))))
  if not blank(a.summary) { pad-top(3.7pt, rich(a.summary)) }
})

#let dots(n) = box(range(5).map(i => h(2.6pt) + box(width: 10.1pt, height: 10.1pt, radius: 5.05pt, fill: if i < n { accent } else { dot-off })).join())

#let language(l) = grid(columns: (1fr, auto), align: (left + horizon, right + horizon), {
  block(width: 100%, heading(level: 3, strong-title(l.name)))
  if l.fluency != none { pad-top(0.5pt, [#l.fluency]) }
}, dots(l.rating))

// Two profiles per row (52.3% and 47.7% of the column).
#let profiles(items) = grid(columns: (52.3%, 47.7%), ..items.map(p => block(width: 100%, inset: (bottom: 2pt), icon-row(p.icon, {
  block(width: 100%, heading(level: 3, strong-title(p.network)))
  pad-top(3.7pt, link-or([#p.text], p.href))
}))))

#let reference(r) = {
  for p in r.text { block(width: 100%, text(style: "italic", [“] + rich(p) + [”])) }
  pad-top(2pt, text(weight: 700, fill: heading-ink, r.name))
}

#let line-item(l) = {
  block(width: 100%, par({
    text(weight: 700, fill: heading-ink, link-or(rich(l.title), l.url))
    if not blank(l.detail) { [, ] + rich(l.detail) }
  }))
  if not blank(l.summary) { pad-top(1pt, rich(l.summary)) }
}

// An item: 3.1pt above and below (2.85pt for a language; 3pt above a line after a line), and a
// dashed rule above every item but the first of its section on the page.
#let item(chunk, it) = block(width: 100%, breakable: it.overflow, {
  let d = it.data
  let k = chunk.kind
  if not it.first {
    block(width: 100%, height: 0.65pt, place(horizon, line(length: 100%, stroke: (paint: separator, thickness: 0.65pt, dash: "dashed"))))
  }
  let (top, bottom) = if k == "language" { (2.85pt, 2.85pt) } else if k == "line" and not it.first { (3pt, 3.1pt) } else { (3.1pt, 3.1pt) }
  block(width: 100%, inset: (top: top, bottom: bottom), {
    if k == "entry" { entry(d) } else if k == "text" { text-item(d) } else if k == "tags" { tags(d) } else if k == "achievement" {
      achievement(d)
    } else if k == "language" { language(d) } else if k == "profiles" { profiles(d) } else if k == "reference" { reference(d) } else if k == "line" {
      line-item(d)
    }
  })
})

// CSS: `.section { margin-top: 9pt }`, none for the first in its column.
#let section(chunk, first) = {
  if not first { block(width: 100%, height: 9pt) }
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

#let column(chunks) = for (ci, chunk) in chunks.enumerate() { section(chunk, ci == 0) }

#let columns(page) = grid(columns: (left-width, 1fr, right-width),
  block(width: left-width, column(page.columns.at("left"))), [], block(width: right-width, column(page.columns.at("right"))))

// ------------------------------------------------------------------ document

#if measuring {
  set page(width: data.page.width * 1pt, height: auto, margin: 0pt)
  block(width: width, {
    header()
    mark("c")
    columns(data.pages.at(0))
  })
  [#metadata((text: 10, line: lh(1.23) * 10, width: left-width / 1pt))<cvforge-measure>]
} else {
  set page(
    width: data.page.width * 1pt,
    height: data.page.height * 1pt,
    margin: (top: m.top * 1pt, right: m.right * 1pt, bottom: m.bottom * 1pt, left: m.left * 1pt),
  )
  for (pi, p) in data.pages.enumerate() {
    if pi > 0 { pagebreak() }
    if p.header { header() }
    columns(p)
  }
}
