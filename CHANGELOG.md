# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.2] - 2026-10-15

### Added
- Chinese, Japanese, Korean, Arabic, Hebrew, Indic, Thai and other scripts:
  fonts are borrowed from the system when a document needs them, on screen and
  in the PDF (with proper shaping: joined Arabic letters, Indic conjuncts,
  right-to-left words).
- HTML export embeds formulas and Mermaid diagrams as SVG and local images as
  data, so the file shows the same thing wherever it's opened. Scripts and
  other active HTML in a document are shown as text, not run.
- A notice when the open file is deleted or moved, with Save / Save As.

### Changed
- Footnotes are numbered in order of reference (¹ ²…), like on GitHub.
- Tables too wide for the page are set in smaller type instead of being cut off.
- Code no longer shows font ligatures: `<!--` and `-->` stay as typed
  (JetBrains Mono NL).
- Pictures, diagrams and formulas stay smooth when zoomed out and in thumbnails.

### Fixed
- Crashes when formatting or replacing text next to emoji or CJK characters,
  and when a file being edited was reloaded from disk.
- Bold, italic and the like on a heading or list line wrapped the `#` or `-`
  too; several selected lines are now wrapped one by one.
- The Undo and Redo buttons did nothing.
- Ctrl+Shift+X and Ctrl+Shift+C cut and copied instead of making text
  strikethrough or code; Ctrl+Alt+1 and Ctrl+Alt+2 zoomed instead of making
  a heading.
- Enter on a selection inside a list kept the selected text.
- Replace skipped every other match.
- Two Page view: Page Down and Next Page didn't move. Single Page view:
  zooming jumped back to the first page.
- Saving replaced a symbolic link with a file and reset the file's permissions.
- PDF: links to a heading that doesn't exist are left out.

## [0.1.1] - 2026-10-15

### Added
- Right-click a tab: Close, Close Others, Close Tabs to the Right, Close All,
  Copy Path, Show in Folder.
- Menu bar: while a menu is open, pointing at another title opens that one,
  like native menus.

### Changed
- New app icon: a page with "#tt" on the red badge.

### Fixed
- The ✕ on a tab didn't close it.
- Windows: paths showed a `\\?\` prefix (status bar, Recent, Properties), and
  images next to a document could fail to load because of it.
- Windows: making TandaiTurun the default app flashed console windows.

## [0.1.0] - 2026-10-15

First release.

### Added
- Reader: Markdown laid out on A4 or Letter pages, with continuous, single page
  and two page views, zoom (25–400 %, fit width, fit page), page thumbnails,
  bookmarks from the headings, find with highlights and a results list by page,
  text selection and copy, Hand tool, links (web, in-document anchors, other
  Markdown files), Read Mode and full screen.
- Paper, sepia and night page colours; sans or serif text; text size.
- Markdown: CommonMark with tables, task lists, strikethrough, footnotes,
  heading IDs, front matter, syntax-highlighted code, local images (incl. SVG),
  simple HTML (`<img>`, `<br>`, `<mark>`, `<sub>`, `<sup>`) and
  `<!-- pagebreak -->`.
- Mermaid diagrams, drawn natively with mermaid-rs-renderer.
- LaTeX math (`$…$`, `$$…$$`) typeset with latex-rust, on screen and in PDF.
- Internet images on request: a bar offers to show them for the document or
  always; `--allow-remote-images` for `--export-pdf`.
- Editor: Markdown colouring, live paginated preview that follows the cursor,
  formatting toolbar and shortcuts, list continuation and indentation, find and
  replace, undo/redo.
- Files: tabs, Home screen with recent files, drag and drop, session restore,
  line endings and BOM kept on save, atomic saves, reload when changed on disk.
- Export to PDF (embedded subset fonts, selectable text, links, bookmarks) and
  HTML; print through the system PDF viewer; `--export-pdf` and
  `--export-html` on the command line.
- Document Properties, Preferences, keyboard shortcuts, English and Bahasa Indonesia.
- Make TandaiTurun the default Markdown app (Linux: xdg-mime, macOS: Launch
  Services, Windows: registration plus Settings → Default apps). The Windows
  installer can also associate `.md` files.
- About dialog with the list of every open-source library in the app and the fonts.
- Packages: Windows installer and zip, macOS universal `.dmg`, `.deb`, `.rpm`,
  AppImage and `.tar.gz`, built and tested by GitHub Actions.

[Unreleased]: https://github.com/sansob/TandaiTurun/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/sansob/TandaiTurun/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/sansob/TandaiTurun/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/sansob/TandaiTurun/releases/tag/v0.1.0
