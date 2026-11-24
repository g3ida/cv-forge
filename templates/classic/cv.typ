// Classic: serif, centered header, one column in reading order (safe for resume parsers).
// Port of cv-builder's templates/classic (cv.html.j2 + style.css), box for box: the same sizes,
// line heights, paddings and rules, so pages break where the original's do.
//
// Built for text extraction: one block after another in reading order, no floats or
// positioned boxes; no letter-spacing, small caps or hyphenation; ligatures off. Bullets and
// separators are real text.

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#1d1d1d")
#let heading-ink = rgb("#111111")
#let muted = rgb("#595959")
#let serif = "Noto Serif"
#let width = (data.page.width - m.left - m.right) * 1pt
#let lines(v) = line-box(serif, lh(v))

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: serif, size: 10pt, fill: ink, lang: data.lang, region: data.region, ligatures: false, hyphenate: false, ..lines(1.42))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
// Headings carry the PDF structure and bookmarks; their look is set inside each body, which
// overrides Typst's built-in heading sizes and weight.
#show heading: it => it.body

// ------------------------------------------------------------------ header

// The photo sits centered above the name: 64pt, 2pt of padding, a 0.75pt accent ring.
#let photo() = {
  let d = 64pt
  box(width: d + 5.5pt, height: d + 5.5pt, {
    place(center + horizon, circle(radius: (d + 4.75pt) / 2, stroke: 0.75pt + accent))
    place(center + horizon, box(width: d, height: d, radius: d / 2, clip: true,
      image(data.header.photo, width: d, height: d, fit: "cover")))
  })
}

#let contact-line(items) = par(items.enumerate().map(((i, c)) => {
  let sep = if i > 0 { [#" · "] } else { [] }
  sep + nowrap(link-or([#c.text], c.href), w: c.w, room: width)
}).join())

// Contacts on as few lines as fit, balanced so that no line ends or starts with a separator:
// whole contacts are grouped into lines by their length in characters. 100 characters of the
// contacts line take about 430pt, inside the 487pt measure of A4 with these margins.
#let contacts(items, max-chars: 100) = {
  let n = items.len()
  let lens = items.map(c => c.text.codepoints().len())
  let total = lens.sum(default: 0) + 3 * (n - 1)
  let groups = if total <= max-chars or n < 2 { (items,) } else {
    let best = 1
    let cost = 100000
    for k in range(1, n) {
      let a = lens.slice(0, k).sum() + 3 * (k - 1)
      let b = lens.slice(k).sum() + 3 * (n - k - 1)
      let c = calc.max(a, b)
      if c < cost {
        cost = c
        best = k
      }
    }
    if cost <= max-chars { (items.slice(0, best), items.slice(best)) } else {
      let out = ()
      let line = ()
      let len = 0
      for c in items {
        let l = c.text.codepoints().len() + (if line.len() > 0 { 3 } else { 0 })
        if line.len() > 0 and len + l > max-chars {
          out.push(line)
          line = ()
          len = 0
          l = c.text.codepoints().len()
        }
        line.push(c)
        len += l
      }
      out.push(line)
      out
    }
  }
  for (i, g) in groups.enumerate() { pad-top(if i > 0 { 1pt } else { 0pt }, contact-line(g)) }
}

#let header() = {
  set align(center)
  block(width: 100%, inset: (bottom: 10pt), {
    if data.header.photo != none { block(width: 100%, inset: (bottom: 9pt), photo()) }
    block(width: 100%, heading(level: 1, text(size: 26.5pt, weight: 400, fill: heading-ink, ..lines(1.15), data.header.name)))
    if not blank(data.header.label) {
      pad-top(4pt, {
        set text(size: 12pt, style: "italic", fill: accent, ..lines(1.3))
        rich(data.header.label)
      })
    }
    if data.header.contacts.len() > 0 {
      pad-top(if blank(data.header.label) { 8pt } else { 6pt }, {
        set text(size: 10pt, fill: muted)
        contacts(data.header.contacts)
      })
    }
  })
  // border-bottom: 2.25pt double
  rule(0.75pt, accent)
  block(width: 100%, height: 0.75pt)
  rule(0.75pt, accent)
}

// ------------------------------------------------------------------ items

#let section-title(chunk) = block(width: 100%, inset: (bottom: 5pt), {
  block(width: 100%, inset: (bottom: 2pt), heading(level: 2, text(size: 13pt, weight: 700, fill: accent, ..lines(1.25), chunk.title)))
  rule(0.6pt, accent)
})

// Heading lines in reading order: title (position), subtitle (employer), the dates alone on
// their line, then location, score and link on the next. Resume parsers attach whatever shares
// a line with a date range to the role, so nothing shares that line. A continued piece has
// bullets only.
#let entry(e, first) = pad-top(if first or e.continued { 0pt } else { 9pt }, {
  if not blank(e.title) {
    block(width: 100%, heading(level: 3, text(size: 11pt, weight: 700, fill: heading-ink, link-or(rich(e.title), e.title_url))))
  }
  if not blank(e.subtitle) {
    block(width: 100%, text(style: "italic", fill: heading-ink, link-or(rich(e.subtitle), e.subtitle_url)))
  }
  let meta = e.meta.flatten()
  let dates = meta.filter(x => x.icon == "calendar")
  let rest = meta.filter(x => x.icon == "pin") + meta.filter(x => x.icon not in ("pin", "calendar"))
  if dates.len() > 0 {
    block(width: 100%, text(fill: muted, dates.map(x => x.text).join(" | ")))
  }
  if rest.len() > 0 {
    block(width: 100%, text(fill: muted, rest.map(x => link-or([#x.text], x.href)).join([#" · "])))
  }
  for p in e.paragraphs { pad-top(2.5pt, rich(p)) }
  // The bullet is ordinary text at the start of the line, with a hanging indent.
  for b in e.bullets { pad-top(2pt, par(hanging-indent: 10pt, box(width: 10pt, [•]) + rich(b))) }
  if not blank(e.tags) { pad-top(2.5pt, text(style: "italic", fill: muted, rich(e.tags))) }
})

#let text-item(paras) = for (i, p) in paras.enumerate() {
  pad-top(if i > 0 { 4pt } else { 0pt }, rich(p))
}

// "Group: keyword, keyword" on one line; the colon only when keywords follow. Wrapped lines
// hang under the group name.
#let tags(s, first) = pad-top(if first { 0pt } else { 2pt }, par(hanging-indent: 10pt, {
  text(weight: 700, fill: heading-ink, {
    rich(s.name)
    if s.level != none { text(weight: 400, style: "italic", fill: muted, [ · #s.level]) }
    if not blank(s.text) { [:] }
  })
  if not blank(s.text) { [ ] + rich(s.text) }
}))

#let achievement(a, first) = pad-top(if first { 0pt } else { 6pt }, {
  block(width: 100%, heading(level: 3, text(size: 10pt, weight: 700, fill: heading-ink, rich(a.title))))
  if not blank(a.summary) { block(width: 100%, rich(a.summary)) }
})

#let language(l, first) = pad-top(if first { 0pt } else { 2pt }, par({
  text(weight: 700, fill: heading-ink, l.name)
  if l.fluency != none { [: #l.fluency] } else if l.rating > 0 { [: #str(l.rating)/5] }
}))

#let profiles(items) = for (i, p) in items.enumerate() {
  pad-top(if i > 0 { 2pt } else { 0pt }, par({
    text(weight: 700, fill: heading-ink, p.network)
    [: ]
    link-or([#p.text], p.href)
  }))
}

#let reference(r, first) = pad-top(if first { 0pt } else { 6pt }, {
  for p in r.text { block(width: 100%, text(style: "italic", [“] + rich(p) + [”])) }
  pad-top(2pt, text(weight: 700, fill: heading-ink, r.name))
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
  if k == "entry" { entry(d, it.first) } else if k == "text" { text-item(d) } else if k == "tags" {
    tags(d, it.first)
  } else if k == "achievement" { achievement(d, it.first) } else if k == "language" {
    language(d, it.first)
  } else if k == "profiles" { profiles(d) } else if k == "reference" { reference(d, it.first) } else if k == "line" {
    line-item(d, it.first)
  }
})

// CSS: `.section { margin-top: 14pt }`, none for the first in its column.
#let section(chunk, first) = {
  if not first { block(width: 100%, height: 14pt) }
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

#let page-header() = block(width: 100%, inset: (bottom: 14pt), header())

#if measuring {
  set page(width: data.page.width * 1pt, height: auto, margin: 0pt)
  block(width: width, {
    page-header()
    mark("c")
    for (ci, chunk) in data.pages.at(0).columns.at("main").enumerate() { section(chunk, ci == 0) }
  })
  [#metadata((text: 10, line: lh(1.42) * 10, width: width / 1pt))<cvforge-measure>]
} else {
  set page(
    width: data.page.width * 1pt,
    height: data.page.height * 1pt,
    margin: (top: m.top * 1pt, right: m.right * 1pt, bottom: m.bottom * 1pt, left: m.left * 1pt),
  )
  // A cover letter: this page's band and header, then the letter. The letter's 26pt top margin
  // meets .header's 14pt margin, which page-header() adds below header().
  if lettering {
    header()
    letter-body(serif, ink, heading-ink, muted, accent, top: 26pt)
  }
  for (pi, p) in data.pages.enumerate() {
    if pi > 0 { pagebreak() }
    if p.header { page-header() }
    for (ci, chunk) in p.columns.at("main").enumerate() { section(chunk, ci == 0) }
  }
}
