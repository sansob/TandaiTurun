# Contributing to TandaiTurun

Thanks for wanting to help! Bug reports, documentation fixes, translations and
code are all welcome, and you don't need to be a Rust expert to start.

## Quick version

1. Look for an existing issue, or open one to talk about bigger changes first.
2. Fork, then branch from `main`: `git checkout -b feature/my-thing main`.
3. Make the change, with a test when it touches `tandaiturun-core` or the page layout.
4. Run `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
5. Open a pull request and fill in the template.

## Where things live

| Path | What |
|---|---|
| `crates/tandaiturun-core` | No GUI: loading/saving files, outline, search, the editing commands, statistics, HTML export. |
| `crates/tandaiturun/src/markdown.rs` | Markdown → the block tree the layout works from. |
| `crates/tandaiturun/src/layout.rs` | The page layout: blocks → positioned text, boxes and images, cut into pages. |
| `crates/tandaiturun/src/view.rs` | The canvas: pages, zoom, selection, links, search highlights, thumbnails. |
| `crates/tandaiturun/src/pdf.rs` | PDF export from the same layout (subset fonts, links, bookmarks). |
| `crates/tandaiturun/src/mermaid.rs` | Mermaid diagrams (via `mermaid-rs-renderer` + `resvg`). |
| `crates/tandaiturun/src/editor.rs` | The source editor: colouring, formatting commands, list continuation. |
| `crates/tandaiturun/src/{app,chrome,panes,dialogs}.rs` | Application state and the window around the document. |
| `crates/tandaiturun/src/i18n_id.rs` | Bahasa Indonesia. A test fails when a UI text has no translation. |
| `packaging/` | `.deb`, `.rpm`, AppImage, Windows installer and macOS `.dmg`. |

### Screenshots

`TANDAITURUN_SCENE=<name> TANDAITURUN_SCREENSHOT=out.png tandaiturun examples/showcase.md`
opens a scripted scene, saves a screenshot and exits. The scenes are listed in
`crates/tandaiturun/src/scene.rs`; `scripts/screenshots.sh` makes all the
images in `docs/screenshots/` (it needs `xvfb-run` on Linux).

### Dependencies

After adding or updating a dependency, run `python3 scripts/gen-credits.py`:
it refreshes the library list shown in **Help → About → Libraries** (CI checks it).

### Translations

UI texts are written in English inside `tr("…")`. Add the Indonesian text to
`crates/tandaiturun/src/i18n_id.rs`; `cargo test` tells you what is missing.
Another language is welcome too: add it to `i18n.rs` like `Lang::Id`.

## Reporting bugs

Please include your OS, the TandaiTurun version (**Help → About** or
`tandaiturun --version`), what you did, what you expected and what happened.
A small Markdown file that shows the problem helps a lot.

Security problems: see [SECURITY.md](SECURITY.md), not public issues.

## License

By contributing you agree that your work is dual-licensed under MIT and
Apache-2.0, like the rest of the project.

Everyone taking part is expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
