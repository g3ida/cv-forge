// Grid: a Swiss-style page on two fixed columns, made for people to read. Port of cv-builder's
// templates/grid (cv.html.j2 + style.css), box for box.
//
// Horizontal grid from the left margin: section labels and the photo in 0-104pt, everything
// else on the text edge at 116pt. A section label is placed in the margin column and takes no
// height of its own (CSS: position absolute), so the items alone set a section's height.

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#2a2a2a")
#let heading-ink = rgb("#0f0f0f")
#let muted = rgb("#6b6b6b")
#let sans = "Inter"
#let gutter = 116pt
#let width = (data.page.width - m.left - m.right) * 1pt
#let text-width = width - gutter
#let lines(v) = line-box(sans, lh(v))

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: sans, size: 10pt, fill: ink, lang: data.lang, region: data.region, hyphenate: false, ..lines(1.42))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
#show heading: it => it.body

#let sep = text(fill: muted, [#" / "])

// ------------------------------------------------------------------ header

#let contacts(items) = par(items.enumerate().map(((i, c)) => {
  nowrap(link-or([#c.text], c.href), w: c.w, room: text-width) + if i < items.len() - 1 { sep } else { [] }
}).join())

// The name on two lines (first names, surname), split at its last space.
#let name() = {
  let parts = data.header.name.split(" ")
  if parts.len() > 1 { parts.slice(0, -1).join(" ") + linebreak() + parts.last() } else { data.header.name }
}

#let header() = block(width: 100%, inset: (bottom: 24pt), {
  block(width: 100%, heading(level: 1, text(size: 36pt, weight: 700, tracking: -1.2pt, fill: heading-ink, ..line-box(sans, 1), name())))
  pad-top(12pt, grid(
    columns: (gutter, 1fr),
    if data.header.photo != none { box(width: 78pt, height: 78pt, clip: true, image(data.header.photo, width: 78pt, height: 78pt, fit: "cover")) } else { [] },
    {
      if not blank(data.header.label) { block(width: 100%, text(size: 12pt, weight: 500, fill: accent, ..lines(1.3), rich(data.header.label))) }
      if data.header.contacts.len() > 0 { pad-top(4pt, contacts(data.header.contacts)) }
    },
  ))
})

// ------------------------------------------------------------------ items

#let strong-title(body) = text(weight: 600, fill: heading-ink, body)

#let entry(e, first) = pad-top(if first or e.continued { 0pt } else if e.same_employer { 6pt } else { 10pt }, {
  let meta = e.meta.flatten()
  let when = meta.filter(x => x.icon == "calendar") + meta.filter(x => x.icon == "pin")
  let rest = meta.filter(x => x.icon not in ("pin", "calendar"))
  if not e.continued {
    if not blank(e.title) {
      block(width: 100%, heading(level: 3, text(size: 10.5pt, weight: 600, fill: heading-ink, ..lines(1.35), link-or(rich(e.title), e.title_url))))
    }
    if not blank(e.subtitle) or when.len() > 0 {
      block(width: 100%, text(fill: muted, par({
        if not blank(e.subtitle) { text(weight: 500, fill: ink, link-or(rich(e.subtitle), e.subtitle_url)) }
        for (i, x) in when.enumerate() {
          if not blank(e.subtitle) or i > 0 { sep }
          let t = x.text.replace(" - ", " – ")
          nowrap([#t], w: x.w, room: text-width)
        }
      })))
    }
    if rest.len() > 0 {
      block(width: 100%, text(fill: muted, rest.map(x => link-or([#x.text], x.href)).join(sep)))
    }
  }
  for p in e.paragraphs { pad-top(3pt, rich(p)) }
  if e.bullets.len() > 0 {
    pad-top(2pt, for b in e.bullets {
      block(width: 100%, inset: (top: 1.5pt, left: 11pt), {
        place(dx: -11pt, dy: 6pt, rect(width: 3.5pt, height: 3.5pt, fill: accent, stroke: none))
        rich(b)
      })
    })
  }
  if not blank(e.tags) { pad-top(3pt, text(fill: muted, rich(e.tags))) }
})

#let text-item(paras) = for (i, p) in paras.enumerate() { pad-top(if i > 0 { 4pt } else { 0pt }, rich(p)) }

#let tags(s) = par({
  strong-title({
    rich(s.name)
    if s.level != none { [ · #s.level] }
  })
  if not blank(s.text) { [ ] + rich(s.text) }
})

#let achievement(a) = {
  block(width: 100%, heading(level: 3, strong-title(rich(a.title))))
  if not blank(a.summary) { block(width: 100%, rich(a.summary)) }
}

#let language(l) = par({
  strong-title(l.name)
  if l.fluency != none { [ ] + text(fill: muted, l.fluency) } else if l.rating > 0 { [ ] + text(fill: muted, [#str(l.rating)/5]) }
})

#let profiles(items) = for (i, p) in items.enumerate() {
  pad-top(if i > 0 { 1.5pt } else { 0pt }, par({
    strong-title(p.network)
    [ ]
    text(fill: muted, link-or([#p.text], p.href))
  }))
}

#let reference(r) = {
  for p in r.text { block(width: 100%, text(font: "Noto Serif", style: "italic", ..line-box("Noto Serif", lh(1.42)), [“] + rich(p) + [”])) }
  pad-top(2pt, strong-title(r.name))
}

// Certificates and awards, one line each: "Name, Issuer, 2026".
#let line-item(l) = {
  block(width: 100%, par({
    strong-title(link-or(rich(l.title), l.url))
    if not blank(l.detail) { [, ] + rich(l.detail) }
  }))
  if not blank(l.summary) { pad-top(1pt, rich(l.summary)) }
}

// The space above an item, as the original's selectors decide it: `.item + .item` 6pt, none
// between entries, and later rules for tags (2pt), languages (1.5pt) and lines (3pt).
#let item-gap(kind, first) = if first or kind == "entry" { 0pt } else if kind == "tags" { 2pt } else if kind == "language" { 1.5pt } else if kind == "line" { 3pt } else { 6pt }

#let item(chunk, it) = block(width: 100%, breakable: it.overflow, pad-top(item-gap(chunk.kind, it.first), {
  let d = it.data
  let k = chunk.kind
  if k == "entry" { entry(d, it.first) } else if k == "text" { text-item(d) } else if k == "tags" { tags(d) } else if k == "achievement" {
    achievement(d)
  } else if k == "language" { language(d) } else if k == "profiles" { profiles(d) } else if k == "reference" { reference(d) } else if k == "line" {
    line-item(d)
  }
}))

// CSS: `.section { margin-top: 18pt }`, none for the first in its column; the label in the margin.
#let section(chunk, first) = {
  if not first { block(width: 100%, height: 18pt) }
  block(width: 100%, {
    if measuring { mark("s", chunk.id) }
    if data.repeat_titles or not chunk.continued {
      place(top + left, dx: -gutter, block(width: 104pt, heading(level: 2, text(size: 10.5pt, weight: 700, fill: heading-ink, ..lines(1.35), chunk.title))))
    }
    for it in chunk.items {
      if measuring { mark("i") }
      item(chunk, it)
      if measuring { mark("j") }
    }
  })
}

#let column(chunks) = pad(left: gutter, block(width: text-width, for (ci, chunk) in chunks.enumerate() { section(chunk, ci == 0) }))

// ------------------------------------------------------------------ document

#if measuring {
  set page(width: data.page.width * 1pt, height: auto, margin: 0pt)
  block(width: width, {
    header()
    mark("c")
    column(data.pages.at(0).columns.at("main"))
  })
  [#metadata((text: 10, line: lh(1.42) * 10, width: text-width / 1pt))<cvforge-measure>]
} else {
  set page(
    width: data.page.width * 1pt,
    height: data.page.height * 1pt,
    margin: (top: m.top * 1pt, right: m.right * 1pt, bottom: m.bottom * 1pt, left: m.left * 1pt),
  )
  for (pi, p) in data.pages.enumerate() {
    if pi > 0 { pagebreak() }
    if p.header { header() }
    column(p.columns.at("main"))
  }
}
