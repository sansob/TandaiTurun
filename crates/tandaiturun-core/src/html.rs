//! Export to a standalone HTML file (styles inlined, no scripts, no network).

use pulldown_cmark::{html, Event, Parser, Tag, TagEnd};

/// Escapes text for HTML element content and attribute values.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// The body HTML: front matter dropped, headings given their outline anchors,
/// page-break comments turned into print page breaks.
pub fn body(markdown: &str) -> String {
    let headings = crate::outline::outline(markdown);
    let mut next_heading = headings.iter();
    let mut in_meta = false;
    let events = Parser::new_ext(markdown, crate::parser_options()).filter_map(|e| match e {
        Event::Start(Tag::MetadataBlock(_)) => {
            in_meta = true;
            None
        }
        Event::End(TagEnd::MetadataBlock(_)) => {
            in_meta = false;
            None
        }
        _ if in_meta => None,
        Event::Start(Tag::Heading { level, id, classes, attrs }) => {
            let anchor = next_heading.next().map(|h| h.anchor.clone());
            let id = id.or(anchor.map(Into::into));
            Some(Event::Start(Tag::Heading { level, id, classes, attrs }))
        }
        Event::Html(h) | Event::InlineHtml(h) if crate::is_page_break(&h) => {
            Some(Event::Html("<div class=\"page-break\"></div>\n".into()))
        }
        other => Some(other),
    });
    let mut out = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut out, events);
    out
}

const CSS: &str = r#"
:root { color-scheme: light; }
body { margin: 0; background: #e6e6e6; color: #1f1f1f;
  font: 16px/1.6 Inter, "Segoe UI", system-ui, -apple-system, "Helvetica Neue", Arial, sans-serif; }
main { box-sizing: border-box; max-width: 210mm; min-height: 297mm; margin: 24px auto; padding: 20mm;
  background: #fff; box-shadow: 0 1px 4px rgba(0,0,0,.25); }
h1, h2, h3, h4, h5, h6 { line-height: 1.25; margin: 1.4em 0 .5em; }
h1 { font-size: 2em; border-bottom: 1px solid #ddd; padding-bottom: .25em; }
h2 { font-size: 1.5em; border-bottom: 1px solid #eee; padding-bottom: .2em; }
h1:first-child, h2:first-child { margin-top: 0; }
a { color: #1473e6; }
code, pre { font-family: "JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: .88em; }
code { background: #f2f2f2; padding: .1em .3em; border-radius: 3px; }
pre { background: #f6f8fa; border: 1px solid #e3e3e3; border-radius: 4px; padding: 12px 14px; overflow: auto; }
pre code { background: none; padding: 0; }
blockquote { margin: 1em 0; padding: .2em 1em; color: #555; border-left: 4px solid #d0d0d0; }
table { border-collapse: collapse; margin: 1em 0; }
th, td { border: 1px solid #d0d0d0; padding: 6px 12px; }
th { background: #f3f3f3; }
img { max-width: 100%; }
hr { border: 0; border-top: 1px solid #ccc; margin: 2em 0; }
li.task { list-style: none; }
.page-break { break-after: page; }
@page { size: A4; margin: 20mm; }
@media print {
  body { background: #fff; }
  main { margin: 0; padding: 0; max-width: none; min-height: 0; box-shadow: none; }
  pre, blockquote, table, img { break-inside: avoid; }
  h1, h2, h3, h4 { break-after: avoid; }
}
"#;

/// A complete HTML document.
pub fn document(markdown: &str, title: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"generator\" content=\"TandaiTurun\">\n<title>{}</title>\n<style>{CSS}</style>\n</head>\n<body>\n<main>\n{}</main>\n</body>\n</html>\n",
        escape(title),
        body(markdown)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_gfm_with_anchors() {
        let md = "---\ntitle: T\n---\n# Halo\n\n| a |\n|---|\n| b |\n\n- [x] done\n\n~~old~~\n\n<!-- pagebreak -->\n\n# Halo\n";
        let h = body(md);
        assert!(!h.contains("title: T"), "{h}");
        assert!(h.contains("<h1 id=\"halo\">Halo</h1>"), "{h}");
        assert!(h.contains("<h1 id=\"halo-1\">Halo</h1>"), "{h}");
        assert!(h.contains("<table>"));
        assert!(h.contains("checkbox"));
        assert!(h.contains("<del>old</del>"));
        assert!(h.contains("class=\"page-break\""));
    }

    #[test]
    fn title_is_escaped() {
        let d = document("x", "<script>");
        assert!(d.contains("<title>&lt;script&gt;</title>"));
        assert!(!d.contains("<script>"));
    }
}
