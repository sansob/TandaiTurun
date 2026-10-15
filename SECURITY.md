# Security policy

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

Please **do not** open a public issue. Use
[GitHub private vulnerability reporting](https://github.com/sansob/TandaiTurun/security/advisories/new)
with steps to reproduce and the affected version. You should get a reply within
a week. Fixes are released as soon as possible and credited unless you prefer otherwise.

## Scope notes

- TandaiTurun opens files you choose and makes no network requests of its own.
  Remote images in a document are **not** loaded, so opening a file can't be
  tracked by its author.
- Links are only opened when you click them: web links in your browser,
  links to other Markdown files in a new tab, other files with the program
  your system uses for them.
- Raw HTML in a document is never executed: only `<img>`, `<br>`, `<mark>`,
  `<sub>`, `<sup>` and plain text are shown.
- A malicious file could still try to make the parser or the layout slow or
  crash the app. Reports of such files are welcome.
