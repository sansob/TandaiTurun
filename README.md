<p align="center">
  <img src="assets/icon-256.png" width="96" alt="TandaiTurun logo">
</p>

<h1 align="center">TandaiTurun</h1>

<p align="center">
  Read Markdown like a PDF. Edit it like text.<br>
  A native, open-source Markdown reader and editor, written in Rust.
</p>

<p align="center">
  <a href="https://github.com/sansob/TandaiTurun/actions/workflows/ci.yml"><img src="https://github.com/sansob/TandaiTurun/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/sansob/TandaiTurun/releases"><img src="https://img.shields.io/github/v/release/sansob/TandaiTurun?include_prereleases" alt="Release"></a>
  <img src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue" alt="License">
</p>

<p align="center">
  <b>English</b> · <a href="README.id.md">Bahasa Indonesia</a>
</p>

![TandaiTurun showing a report as pages, with bookmarks and tools](docs/screenshots/read.png)

## Why this exists

Markdown is how a lot of us write now: READMEs, meeting notes, reports,
documentation. But to *read* it, you either squint at the raw text or open it in
a browser tab or an editor that renders it as one endless web page. And when
someone wants "the PDF", you go looking for a converter.

TandaiTurun treats a Markdown file like a document. It lays it out on real
pages (A4 or Letter) with the layout people know from PDF readers: page
thumbnails, bookmarks, a zoom box, two-page view. When you want to change
something you press **Edit** and get the source with a live preview, then
export the exact same pages to PDF.

It's a native app (one ~28 MB binary, no Electron, no browser engine), it starts
instantly, and it never sends anything anywhere.

The name is a literal Indonesian translation: *tandai* means "mark", *turun*
means "down". Mark down.

## What it can do

**Reading**
- Pages with real pagination: headings stay with the text after them, tables
  and code split cleanly between rows, and `<!-- pagebreak -->` starts a new page.
- Continuous, single page and two page view. Zoom 25 to 400 %, fit width, fit
  page, `Ctrl` + mouse wheel.
- **Page Thumbnails** and **Bookmarks** (the headings) in the navigation pane,
  with the current section highlighted as you scroll.
- **Find** (`Ctrl+F`) highlights every match on the pages; the Search pane lists
  them by page with some context. Match case and whole words.
- Select and copy text right on the page, or use the Hand tool to drag.
- Links work: web links open in your browser, `#section` links jump inside the
  document, links to other `.md` files open them in a new tab.
- **Read Mode** (`Ctrl+H`) and full screen (`F11`).
- Paper, sepia and night page colours; sans (Inter) or serif text; adjustable size.

**Markdown support**
- CommonMark plus GitHub extensions: tables (with alignment), task lists,
  strikethrough, footnotes, `<https://…>` autolinks, heading IDs.
- Syntax highlighting for code blocks in many languages.
- **Mermaid diagrams** from ```` ```mermaid ```` blocks: flowcharts, sequence,
  class, state, ER, pie, Gantt, timeline, mindmap and more, drawn natively with
  [mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer), no browser.
- **Math**: `$…$` inline and `$$…$$` display formulas, typeset with TeX's rules
  by [latex-rust](https://github.com/jscarr64/LaTeX-Rust) (fractions, roots,
  sums, integrals, matrices, Greek…). Inline formulas sit on the text baseline.
- Images next to your document (PNG, JPEG, GIF, WebP, BMP, SVG), and simple HTML
  like `<img>`, `<br>`, `<mark>`, `<sub>`, `<sup>`.
- **Internet images** are only loaded when you say so: a bar above the page
  offers **Show images** (this document) or **Always show images**, and each
  placeholder can be clicked. Preferences has the same switch.
- YAML front matter is hidden from the page and shown in Document Properties.

**Editing**

![Editing with a live preview](docs/screenshots/edit.png)

- Source editor with Markdown colouring next to a live, paginated preview that
  follows your cursor.
- Formatting toolbar and shortcuts: headings (`Ctrl+Alt+1…6`), bold (`Ctrl+B`),
  italic (`Ctrl+I`), strikethrough, code, lists, task lists, quotes, links
  (`Ctrl+K`), images, tables, code blocks, rules and page breaks.
- `Enter` continues lists (bullets, numbers, checkboxes, quotes); `Tab` and
  `Shift+Tab` indent them.
- Find and replace in the source, undo and redo.
- Saving keeps your file's line endings (LF or CRLF) and BOM, and writes through
  a temporary file, so a crash never leaves half a document.
- Files changed by another program are reloaded (or you're asked, if you have
  unsaved edits).

**Export and print**
- **PDF** with real, selectable text (the fonts are embedded and subset, so files
  stay small), clickable links, and the headings as PDF bookmarks.
- **HTML**: one standalone file with the styles inside and no scripts.
- **Print** opens the PDF in your system's PDF viewer, where you print it.
- The same exports from the command line, see below.

**And also**
- Tabs, a Home screen with recent files, drag and drop, reopening the documents
  from last time.
- Make TandaiTurun the default app for `.md` files from Preferences or the Home
  screen (on Windows this opens Settings → Default apps, where you pick it:
  Windows doesn't let apps change the default themselves).
- Document Properties: words, characters, reading time, headings, links, images…
- **Help → About → Libraries** lists every open-source library inside the app,
  with its version and licence.
- English and Bahasa Indonesia.
- Light and dark themes.

| | |
|---|---|
| ![Thumbnails and two-page view](docs/screenshots/thumbs.png) | ![Night page colour, dark theme](docs/screenshots/dark.png) |
| ![Search results by page](docs/screenshots/search.png) | ![Bahasa Indonesia, sepia, serif](docs/screenshots/bahasa.png) |

## Install

Download the latest version from the
[Releases page](https://github.com/sansob/TandaiTurun/releases):

| Platform | File |
|---|---|
| Windows 10/11 | `TandaiTurun-…-windows-x64-setup.exe` (or `arm64`), or the portable `.zip` |
| macOS 11+ (Apple Silicon and Intel) | `TandaiTurun-…-macos-universal.dmg` |
| Debian, Ubuntu | `tandaiturun_…_amd64.deb` (or `arm64`) |
| Fedora, RHEL, openSUSE | `tandaiturun-….x86_64.rpm` (or `aarch64`) |
| Any Linux | `TandaiTurun-…-x86_64.AppImage`, or the `.tar.gz` |

The builds are not code-signed yet. On macOS, right-click the app and choose
**Open** the first time; on Windows, choose **More info → Run anyway**.

### Build from source

You need [Rust](https://rustup.rs) 1.95 or newer.

```sh
git clone https://github.com/sansob/TandaiTurun
cd TandaiTurun
cargo run --release
```

On Linux the app uses the system's OpenGL/EGL, xkbcommon and X11 or Wayland
libraries; every desktop has them. To build you may need the development headers:
`sudo apt install libxkbcommon-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`.

## Command line

```sh
tandaiturun notes.md README.md          # open documents in tabs
tandaiturun --export-pdf report.md      # → report.pdf, same layout as the app
tandaiturun --export-pdf report.md --allow-remote-images   # include internet images
tandaiturun --export-html report.md out.html
tandaiturun --version
```

## Keyboard shortcuts

| | |
|---|---|
| `Ctrl+O` / `Ctrl+N` / `Ctrl+S` / `Ctrl+Shift+S` | Open, new, save, save as |
| `Ctrl+W` / `Ctrl+Tab` | Close tab, next tab |
| `Ctrl+E` | Edit Markdown / back to reading |
| `Ctrl+F`, `F3`, `Shift+F3` | Find, next, previous |
| `Ctrl+=` / `Ctrl+-` | Zoom in, out |
| `Ctrl+1` / `Ctrl+0` / `Ctrl+2` | Actual size, fit page, fit width |
| `PgUp` / `PgDn` / `Home` / `End` | Previous, next, first, last page |
| `Ctrl+H` / `F11` | Read Mode, full screen |
| `F4` / `Shift+F4` | Navigation pane, tools pane |
| `Ctrl+P` / `Ctrl+D` | Print, document properties |
| `Ctrl+B` / `Ctrl+I` / `Ctrl+K` / `Ctrl+Alt+1…6` | Bold, italic, link, heading (editor) |

## Privacy

TandaiTurun makes no network requests of its own: no telemetry, no update
checks, no account. Images from the internet are only downloaded after you
allow it, because loading them tells those websites that you opened the
document. Settings live in your system's usual app-data folder.

## Good to know

- Math covers LaTeX math mode. Text-mode commands (`\section`, TikZ…) aren't
  supported; a formula that can't be typeset is shown as written.
- Mermaid rendering comes from a young project: most diagrams look right, some
  layouts differ from mermaid.js. A diagram it can't draw is shown as code with
  the error above it.

## Project layout

```
crates/tandaiturun-core   files, outline, search, editing commands, stats, HTML export (no GUI)
crates/tandaiturun        the app: Markdown → pages, viewer, editor, PDF export, UI
examples/                 the welcome guide, cheat sheet and a sample report
packaging/                .deb, .rpm, AppImage, Windows installer, macOS .dmg
```

## Contributing

Bug reports, ideas, translations and code are welcome: see
[CONTRIBUTING.md](CONTRIBUTING.md). Please report security issues privately
([SECURITY.md](SECURITY.md)).

## License

TandaiTurun is dual-licensed under [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option. The bundled fonts (Inter,
Liberation Serif, JetBrains Mono, Noto Emoji, Noto Sans Symbols 2, Noto Sans Math,
STIX Two Math)
are under the SIL Open Font License 1.1; their licences are in
[`assets/fonts`](assets/fonts).
