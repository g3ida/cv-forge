# Glossary

cv-builder's terms and their names in cv-forge.

| Term | Meaning | In cv-forge |
| --- | --- | --- |
| CV | A validated CV document (JSON Resume v1.0.0 plus extensions) | `model::Cv` |
| problem | One validation error: location and pydantic's message | `model::Problem`, `model::Loc` |
| load_cv | Read and validate a CV file | `model::load_cv`, `LoadError::report` |
| layout | The CV's presentation settings (`layout` key) | `model::Layout` |
| header layout | Contacts, header profiles and photo switch | `model::HeaderSet` (as written), `view::HeaderLayout` (resolved) |
| template | A bundled or folder template: `template.json` + `cv.typ` | `template::Template`, `template::Config` |
| main column | The column that holds `work` (unplaced sections go there) | `Template::main_column` |
| settings | Template, page size, accent, page limit, columns, titles, header | `build::Settings`, `build::resolve_settings` |
| section | One CV section of display-ready items | `view::Section` |
| kind | What draws a section's items: entry, text, tags, line, language, achievement, reference, profiles | `Section::kind`, `view::Item` |
| entry | A dated item with title, subtitle, meta lines, paragraphs, bullets, tags | `view::Entry` |
| continued | A later piece of an entry cut up so a page break can fall inside it | `Entry::continued`, `view::split_entries` |
| markup | Escaped HTML text (`Markup`) in the original | `text::Rich` (runs), `text::to_html` |
| keep | Plain text with "++" kept on one line | `text::keep` |
| inline | Text with `**bold**` and `[label](url)` | `text::inline` |
| unit | One measured item: section, index, top and bottom in px | `paginate::Unit` |
| chunk | The part of a section on one column of one page | `build::Chunk` |
| page | Header flag and chunks per column | `build::Page` |
| measure | One measure pass: units, title heights, header height, text metrics | `build::Measure` |
| pagination | Units distributed over pages, per column | `build::Pagination` |
| overflow | An item taller than a page | `paginate::Overflow` |
| tightness | How far line spacing moved toward 1.15: 0 the template's, 1 the floor | `Builder::tightness`, `lh()` in `lib.typ` |
| fit | What fitting to a page limit did | `build::Fit`, `Builder::fit` |
| build result | Pages, notes, overflows, fit, last page fill | `build::BuildResult` (plus `Timings`) |
| marker | A position the measuring layout reports (new in cv-forge) | `mark()` in `lib.typ`, `<cvforge-m>` |
| engine / job | The Typst world: shared library and fonts / one document | `engine::Engine`, `engine::Job` |
