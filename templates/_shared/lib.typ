// Helpers every bundled template imports (`#import "/lib.typ": *`).
//
// The builder (crates/cvforge/src/build.rs) passes the CV as `/data.json`:
//   mode        "measure" (report heights) or "render" (draw the pages)
//   tightness   0 = the template's line spacing, 1 = the floor of 1.15 (fitting to a page limit)
//   page        width, height, margins (pt)
//   header      name, label, contacts, photo
//   columns     the template's column names; pages[i].columns[name] = chunks of sections
//   metrics     ascender and descender (em) per font family, for CSS line boxes
//
// Templates follow the original CSS box for box. All vertical space is explicit (block
// insets and spacer blocks, never spacing between blocks), so positions measured on the tall
// measuring page hold on the real pages: that is what pagination relies on.

#let data = json("/data.json")
#let measuring = data.mode == "measure"

// A CSS line-height between 1.15 and 1.5 moves toward 1.15 by `tightness` (LINE_HEIGHT_FLOOR);
// values outside that range are design (icon boxes, initials), not text, and stay.
#let lh(v) = if 1.15 < v and v <= 1.5 { v - data.tightness * (v - 1.15) } else { v }

// CSS line boxes: every line is exactly `v` times the font size tall, the glyphs placed as CSS
// places them (half the leading above the ascender, half below the descender). Spread into
// `set text(..)`, with `par(leading: 0pt)`.
#let line-box(family, v) = {
  let (asc, desc) = data.metrics.at(family)
  let half = (v - asc - desc) / 2
  (top-edge: (asc + half) * 1em, bottom-edge: -(desc + half) * 1em)
}

// CSS `white-space: nowrap`: one line, as wide as the text, overflowing when it must (a box
// alone would wrap its content at the column's edge). `w` is the builder's upper bound of the
// text's width in em, `extra` the fixed width around it: when that surely fits `room` (the
// column), a plain box does, and nothing needs measuring.
#let nowrap(body, w: none, size: 10, extra: 0pt, room: 0pt) = if w != none and w * size * 1.02pt + extra <= room {
  box(body)
} else {
  context box(width: measure(body).width + 0.01pt, body)
}

// Formatted text from the builder: a string, or runs {t, b: bold, u: link, n: keep together,
// c: CSS class}. `classes` styles runs by class, e.g. (entry-project: it => text(weight: 500, it)).
#let run(r, classes) = {
  let c = [#r.t]
  // a word with "++": at most 30pt text, in at least 100pt of column
  if r.at("n", default: false) { c = nowrap(c, w: r.at("w", default: none), size: 30, room: 100pt) }
  let class = r.at("c", default: none)
  if class != none and class in classes { c = classes.at(class)(c) }
  if r.at("b", default: false) { c = strong(c) }
  if r.at("u", default: none) != none { c = link(r.u, c) }
  c
}
#let rich(x, classes: (:)) = if x == none { none } else if type(x) == str { [#x] } else {
  x.map(r => run(r, classes)).join()
}

// CSS `text-transform: uppercase` on formatted text.
#let rich-upper(x) = if x == none { none } else if type(x) == str { upper(x) } else {
  x.map(r => run(r + (t: upper(r.t)), (:))).join()
}
#let blank(x) = x == none or x == "" or x == ()

// Text that links when there is a target.
#let link-or(body, url) = if url == none { body } else { link(url, body) }

// A block that adds `top` points above its body (CSS padding-top).
#let pad-top(top, body) = block(width: 100%, inset: (top: top), body)

// A full-width horizontal rule `h` points thick.
#let rule(h, color) = block(width: 100%, height: h, fill: color)

// Measure pass. The template lays the whole CV out once, as the page would show it but on one
// page of any height, and drops markers the builder reads the positions of:
//   mark("c")         top of the columns (everything above is the page-one header)
//   mark("s", id)     top of a section (below the space that separates it from the one above)
//   mark("i")/("j")   top/bottom of an item
// plus `#metadata((text, line, width)) <cvforge-measure>`: the body text's size and line
// height and the main column's width, in pt. All vertical space is explicit, so the distance
// between markers is exactly what the item takes on a real page.
#let mark(..v) = [#metadata(v.pos())<cvforge-m>]

// The original's icons (icons.py: Feather, MIT, and Lucide, ISC) on a 24x24 grid, drawn as SVG.
#let icon-paths = (
  "at": "<circle cx=\"12\" cy=\"12\" r=\"4\"/><path d=\"M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-3.92 7.94\"/>",
  "award": "<circle cx=\"12\" cy=\"8\" r=\"7\"/><polyline points=\"8.21 13.89 7 23 12 20 17 23 15.79 13.88\"/>",
  "calendar": "<rect x=\"3\" y=\"4\" width=\"18\" height=\"18\" rx=\"2\" ry=\"2\"/><line x1=\"16\" y1=\"2\" x2=\"16\" y2=\"6\"/><line x1=\"8\" y1=\"2\" x2=\"8\" y2=\"6\"/><line x1=\"3\" y1=\"10\" x2=\"21\" y2=\"10\"/>",
  "gem": "<path d=\"M6 3h12l4 6-10 13L2 9Z\"/><path d=\"M11 3 8 9l4 13 4-13-3-6\"/><path d=\"M2 9h20\"/>",
  "github": "<path d=\"M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 17.13V21\"/>",
  "globe": "<circle cx=\"12\" cy=\"12\" r=\"10\"/><line x1=\"2\" y1=\"12\" x2=\"22\" y2=\"12\"/><path d=\"M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z\"/>",
  "link": "<path d=\"M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71\"/><path d=\"M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71\"/>",
  "linkedin": "<path d=\"M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-2-2 2 2 0 0 0-2 2v7h-4v-7a6 6 0 0 1 6-6z\"/><rect x=\"2\" y=\"9\" width=\"4\" height=\"12\"/><circle cx=\"4\" cy=\"4\" r=\"2\"/>",
  "mail": "<path d=\"M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z\"/><polyline points=\"22,6 12,13 2,6\"/>",
  "phone": "<path d=\"M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z\"/>",
  "pin": "<path d=\"M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z\"/><circle cx=\"12\" cy=\"10\" r=\"3\"/>",
  "star": "<polygon points=\"12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2\"/>",
  "target": "<circle cx=\"12\" cy=\"12\" r=\"10\"/><circle cx=\"12\" cy=\"12\" r=\"6\"/><circle cx=\"12\" cy=\"12\" r=\"2\"/>",
  "user": "<path d=\"M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2\"/><circle cx=\"12\" cy=\"7\" r=\"4\"/>",
  "zap": "<polygon points=\"13 2 3 14 12 14 11 22 21 10 12 10 13 2\"/>",
)

// An icon as the original drew it: `size` pt square, `color` a CSS color, `stroke` in grid units.
// Unknown names fall back to "link". No text: an icon adds nothing a parser reads.
#let icon(name, color, size, stroke: 2.0) = {
  let d = icon-paths.at(name, default: icon-paths.at("link"))
  let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"" + color + "\" stroke-width=\"" + str(stroke) + "\" stroke-linecap=\"round\" stroke-linejoin=\"round\">" + d + "</svg>"
  image(bytes(svg), format: "svg", width: size * 1pt, height: size * 1pt)
}
