// Engineer: the dense black-and-white page engineers pass around, in one column that resume
// parsers read in order. Port of cv-builder's templates/engineer (cv.html.j2 + style.css), box
// for box.
//
// Small capitals are the font's own (Noto Serif has smcp glyphs), so the text keeps its lower
// case. Each entry is headed "Title, Employer" on one line; the dates follow alone on their own
// line, on the left (pdftotext moves text set flush right to the end of the page).

#import "/lib.typ": *

#let m = data.page.margins
#let accent = rgb(data.accent)
#let ink = rgb("#1a1a1a")
#let muted = rgb("#444444")
#let serif = "Noto Serif"
#let width = (data.page.width - m.left - m.right) * 1pt
#let lines(v) = line-box(serif, lh(v))

#set document(title: data.title, author: data.author, description: data.description)
#set text(font: serif, size: 10pt, fill: ink, lang: data.lang, region: data.region, ligatures: false, hyphenate: false, ..lines(1.3))
#set par(leading: 0pt, spacing: 0pt, justify: false)
#set block(spacing: 0pt)
#show heading: it => it.body

// ------------------------------------------------------------------ header

#let contacts(items, room) = par(items.enumerate().map(((i, c)) => {
  let sep = if i < items.len() - 1 { [#" |"] } else { [] }
  let space = if i < items.len() - 1 { [ ] } else { [] }
  nowrap(link-or([#c.text], c.href) + sep, w: c.w + 1, room: room) + space
}).join())

#let header-text(room) = {
  set align(center)
  block(width: 100%, heading(level: 1, text(size: 23pt, weight: 400, fill: accent, ..lines(1.15), smallcaps(data.header.name))))
  if not blank(data.header.label) {
    pad-top(5pt, text(size: 11pt, style: "italic", ..lines(1.3), rich(data.header.label)))
  }
  if data.header.contacts.len() > 0 {
    pad-top(if blank(data.header.label) { 5pt } else { 3pt }, contacts(data.header.contacts, room))
  }
}

// With a photo the text stays centred on the page: an empty box the photo's width balances it.
#let header() = block(width: 100%, inset: (bottom: 8pt), if data.header.photo == none { header-text(width) } else {
  grid(
    columns: (54pt, 1fr, 54pt),
    align: horizon,
    box(width: 54pt, height: 54pt),
    pad(x: 10pt, header-text(width - 128pt)),
    box(width: 54pt, height: 54pt, radius: 2pt, clip: true, image(data.header.photo, width: 54pt, height: 54pt, fit: "cover")),
  )
})

// ------------------------------------------------------------------ items

#let section-title(chunk) = block(width: 100%, inset: (bottom: 3pt), {
  block(width: 100%, inset: (bottom: 1pt), heading(level: 2, text(size: 12.5pt, weight: 400, fill: accent, ..lines(1.2), smallcaps(chunk.title))))
  rule(0.6pt, accent)
})

// "Title, Employer" on one line, the dates alone below it, then the place and any other detail.
#let entry(e, first) = pad-top(if first or e.continued { 0pt } else if e.same_employer { 3pt } else { 5pt }, {
  let meta = e.meta.flatten()
  let dates = meta.filter(x => x.icon == "calendar")
  let rest = meta.filter(x => x.icon == "pin") + meta.filter(x => x.icon not in ("pin", "calendar"))
  if not e.continued and (not blank(e.title) or not blank(e.subtitle) or dates.len() > 0) {
    block(width: 100%, heading(level: 3, text(weight: 400, par({
      if not blank(e.title) { text(weight: 700, link-or(rich(e.title), e.title_url)) }
      if not blank(e.title) and not blank(e.subtitle) { [, ] }
      if not blank(e.subtitle) { text(style: "italic", link-or(rich(e.subtitle), e.subtitle_url)) }
    }))))
    if dates.len() > 0 { block(width: 100%, text(style: "italic", dates.map(x => x.text).join(" | "))) }
    if rest.len() > 0 { block(width: 100%, text(style: "italic", fill: muted, rest.map(x => link-or([#x.text], x.href)).join([#" | "]))) }
  }
  for p in e.paragraphs { pad-top(1.5pt, rich(p)) }
  if e.bullets.len() > 0 {
    pad(left: 10pt, for b in e.bullets { pad-top(1pt, par(hanging-indent: 9pt, box(width: 9pt, [•]) + rich(b))) })
  }
  if not blank(e.tags) { pad-top(1.5pt, text(style: "italic", fill: muted, rich(e.tags))) }
})

#let text-item(paras) = for (i, p) in paras.enumerate() { pad-top(if i > 0 { 3pt } else { 0pt }, rich(p)) }

// "Group: keyword, keyword" on one line; the colon only when keywords follow.
#let tags(s) = pad-top(1pt, par({
  text(weight: 700, {
    rich(s.name)
    if s.level != none { text(weight: 400, [ · #s.level]) }
    if not blank(s.text) { [:] }
  })
  if not blank(s.text) { [ ] + rich(s.text) }
}))

#let achievement(a, first) = pad-top(if first { 0pt } else { 3pt }, par({
  text(weight: 700, rich(a.title))
  if not blank(a.summary) { [: ] + rich(a.summary) }
}))

#let language(l, first) = pad-top(if first { 0pt } else { 1pt }, par({
  text(weight: 700, l.name)
  if l.fluency != none { [: #l.fluency] } else if l.rating > 0 { [: #str(l.rating)/5] }
}))

#let profiles(items) = for (i, p) in items.enumerate() {
  pad-top(if i > 0 { 1pt } else { 0pt }, par({
    text(weight: 700, p.network)
    [: ]
    link-or([#p.text], p.href)
  }))
}

#let reference(r, first) = pad-top(if first { 0pt } else { 3pt }, {
  for p in r.text { block(width: 100%, text(style: "italic", [“] + rich(p) + [”])) }
  block(width: 100%, text(weight: 700, r.name))
})

// Certificates and awards, one line each: "Name, Issuer, 2026".
#let line-item(l, first) = pad-top(if first { 0pt } else { 2pt }, {
  block(width: 100%, par({
    text(weight: 700, link-or(rich(l.title), l.url))
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

// ------------------------------------------------------------------ document

#if measuring {
  set page(width: data.page.width * 1pt, height: auto, margin: 0pt)
  block(width: width, {
    header()
    mark("c")
    for (ci, chunk) in data.pages.at(0).columns.at("main").enumerate() { section(chunk, ci == 0) }
  })
  [#metadata((text: 10, line: lh(1.3) * 10, width: width / 1pt))<cvforge-measure>]
} else {
  set page(
    width: data.page.width * 1pt,
    height: data.page.height * 1pt,
    margin: (top: m.top * 1pt, right: m.right * 1pt, bottom: m.bottom * 1pt, left: m.left * 1pt),
  )
  // A cover letter: this page's band and header, then the letter. The letter's 26pt top margin
  // meets .header's 8pt margin.
  if lettering {
    header()
    letter-body(serif, ink, black, muted, accent, top: 18pt)
  }
  for (pi, p) in data.pages.enumerate() {
    if pi > 0 { pagebreak() }
    if p.header { header() }
    for (ci, chunk) in p.columns.at("main").enumerate() { section(chunk, ci == 0) }
  }
}
