---
title: Welcome to TandaiTurun
---

# Welcome to TandaiTurun

TandaiTurun shows Markdown the way a PDF reader shows a document: as **pages**,
with a zoom control, page thumbnails and bookmarks. When you want to change
something, press **Edit** (or `Ctrl+E`) and you get the Markdown source with
a live preview beside it.

> *Tandai turun* is Indonesian for **mark down**. That's the whole joke.

## What you can do

- [x] Open `.md`, `.markdown` and `.txt` files, or drop them onto the window
- [x] Read in **continuous**, **single page** or **two page** view
- [x] Zoom from 25% to 400%, fit the width or the whole page
- [x] Jump around with **Bookmarks** (the headings) and **Page Thumbnails**
- [x] Find text with `Ctrl+F`; every match is listed by page in the Search pane
- [x] Select and copy text right on the page
- [x] Draw diagrams from `mermaid` code blocks, no browser needed
- [x] Typeset math: $e^{i\pi} + 1 = 0$
- [x] Edit with a formatting toolbar, and see the pages update as you type
- [x] Export to **PDF** (real text, clickable links, bookmarks) or **HTML**
- [x] Print through your system's PDF viewer
- [x] Paper, sepia and night page colours, sans or serif text
- [x] English and Bahasa Indonesia

## Reading

| Action              | Shortcut                  |
| ------------------- | ------------------------- |
| Next / previous page | `PgDn` / `PgUp`           |
| First / last page    | `Home` / `End`            |
| Zoom in / out        | `Ctrl` `+` / `Ctrl` `-`   |
| Actual size          | `Ctrl+1`                  |
| Fit page / width     | `Ctrl+0` / `Ctrl+2`       |
| Read Mode            | `Ctrl+H`                  |
| Full screen          | `F11`                     |

Hold `Ctrl` and turn the mouse wheel to zoom. Pick the **Hand Tool** in the
toolbar to drag the pages around, or the **Select Tool** to select text.

## Editing

The editor understands Markdown: press `Enter` at the end of a list item and
the next bullet (or number, or checkbox) is added for you; press it on an empty
item to end the list. `Tab` and `Shift+Tab` indent and outdent list items.

```rust
// Code blocks get syntax highlighting in many languages.
fn main() {
    println!("Halo, dunia!");
}
```

Need a new page in the PDF? Put `<!-- pagebreak -->` on its own line.

## Your files stay yours

TandaiTurun never uploads anything. Images from the internet are only loaded
when you allow it, so a document can't tell anyone that you opened it; images
stored next to your file are shown as usual.

---

TandaiTurun is open source (MIT or Apache-2.0). Find the code, report a problem
or suggest an idea at <https://github.com/sansob/TandaiTurun>.
