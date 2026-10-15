//! Export to a standalone HTML file (styles inlined, no scripts, no network).

use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, Parser, Tag, TagEnd};

/// What the exporting app can draw for the HTML: formulas and diagrams as
/// SVG, local images as `data:` URLs (so the file works wherever it goes).
/// Each returns `None` to keep the default output.
pub trait Embed {
    fn math(&self, _tex: &str, _display: bool) -> Option<String> {
        None
    }
    /// Fenced code in this language is a diagram (`mermaid`…).
    fn is_diagram(&self, _lang: &str) -> bool {
        false
    }
    fn diagram(&self, _lang: &str, _source: &str) -> Option<String> {
        None
    }
    fn image(&self, _url: &str) -> Option<String> {
        None
    }
}

/// Nothing embedded: math stays TeX, diagrams stay code, images keep their URL.
pub struct Plain;
impl Embed for Plain {}

/// Raw HTML that could run code when the file is opened is shown as text.
fn is_active(html: &str) -> bool {
    let h = html.to_ascii_lowercase();
    ["<script", "<iframe", "<object", "<embed", "javascript:"].iter().any(|t| h.contains(t))
        || h.split(|c: char| c.is_whitespace() || c == '/').any(|w| w.starts_with("on") && w.contains('='))
}

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
    body_with(markdown, &Plain)
}

/// [`body`], with formulas, diagrams and images from `embed`.
pub fn body_with(markdown: &str, embed: &dyn Embed) -> String {
    let headings = crate::outline::outline(markdown);
    let mut next_heading = headings.iter();
    let mut in_meta = false;
    // A fenced block being collected: (language, source).
    let mut fence: Option<(String, String)> = None;
    let events = Parser::new_ext(markdown, crate::parser_options()).filter_map(|e| match e {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref lang))) if !in_meta && embed.is_diagram(lang) => {
            fence = Some((lang.to_string(), String::new()));
            None
        }
        Event::Text(t) if fence.is_some() => {
            fence.as_mut().expect("fence").1.push_str(&t);
            None
        }
        Event::End(TagEnd::CodeBlock) if fence.is_some() => {
            let (lang, src) = fence.take().expect("fence");
            let html = match embed.diagram(&lang, &src) {
                Some(svg) => format!("<figure class=\"diagram\">{svg}</figure>\n"),
                None => format!("<pre><code class=\"language-{}\">{}</code></pre>\n", escape(&lang), escape(&src)),
            };
            Some(Event::Html(html.into()))
        }
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
        Event::Html(h) if crate::is_page_break(&h) => Some(Event::Html("<div class=\"page-break\"></div>\n".into())),
        // Only a comment on its own line breaks the page.
        Event::InlineHtml(h) if crate::is_page_break(&h) => None,
        Event::Html(h) | Event::InlineHtml(h) if is_active(&h) => Some(Event::Text(h)),
        Event::InlineMath(t) => match embed.math(&t, false) {
            Some(svg) => Some(Event::InlineHtml(format!("<span class=\"math\">{svg}</span>").into())),
            None => Some(Event::InlineMath(t)),
        },
        Event::DisplayMath(t) => match embed.math(&t, true) {
            Some(svg) => Some(Event::InlineHtml(format!("<span class=\"math display\">{svg}</span>").into())),
            None => Some(Event::DisplayMath(t)),
        },
        Event::Start(Tag::Image { link_type, dest_url, title, id }) => {
            let dest_url = embed.image(&dest_url).map_or(dest_url, CowStr::from);
            Some(Event::Start(Tag::Image { link_type, dest_url, title, id }))
        }
        Event::Start(Tag::Link { dest_url, .. })
            if dest_url.trim_start().to_ascii_lowercase().starts_with("javascript:") =>
        {
            Some(Event::Start(Tag::Link {
                link_type: pulldown_cmark::LinkType::Inline,
                dest_url: "#".into(),
                title: "".into(),
                id: "".into(),
            }))
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
img, figure.diagram svg { max-width: 100%; height: auto; }
figure.diagram { margin: 1em 0; text-align: center; }
.math svg { vertical-align: middle; }
.math.display { display: block; text-align: center; margin: .8em 0; }
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
    document_with(markdown, title, &Plain)
}

/// [`document`], with formulas, diagrams and images from `embed`.
pub fn document_with(markdown: &str, title: &str, embed: &dyn Embed) -> String {
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"generator\" content=\"TandaiTurun\">\n<title>{}</title>\n<style>{CSS}</style>\n</head>\n<body>\n<main>\n{}</main>\n</body>\n</html>\n",
        escape(title),
        body_with(markdown, embed)
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
    fn active_html_is_shown_as_text() {
        let h = body("<script>alert(1)</script>\n\nhi <img src=x onerror=alert(1)> [x](javascript:alert(1))\n");
        assert!(!h.contains("<script>"), "{h}");
        assert!(!h.contains("<img src=x"), "{h}");
        assert!(!h.contains("href=\"javascript"), "{h}");
        // Harmless HTML stays.
        assert!(body("<details><summary>s</summary>t</details>\n").contains("<details>"));
    }

    #[test]
    fn inline_page_break_is_dropped() {
        let h = body("text <!-- pagebreak --> more\n");
        assert!(!h.contains("page-break"), "{h}");
    }

    #[test]
    fn embeds_math_diagrams_and_images() {
        struct E;
        impl Embed for E {
            fn math(&self, tex: &str, display: bool) -> Option<String> {
                Some(format!("<svg data-tex=\"{tex}\" data-display=\"{display}\"></svg>"))
            }
            fn is_diagram(&self, lang: &str) -> bool {
                lang == "mermaid"
            }
            fn diagram(&self, _lang: &str, src: &str) -> Option<String> {
                Some(format!("<svg data-src=\"{}\"></svg>", src.trim()))
            }
            fn image(&self, url: &str) -> Option<String> {
                Some(format!("data:image/png;base64,{url}"))
            }
        }
        let h = body_with("$x$ and\n\n$$y$$\n\n```mermaid\nA-->B\n```\n\n```rust\nfn\n```\n\n![a](p.png)\n", &E);
        assert!(h.contains("data-tex=\"x\" data-display=\"false\""), "{h}");
        assert!(h.contains("data-tex=\"y\" data-display=\"true\""), "{h}");
        assert!(h.contains("<figure class=\"diagram\"><svg data-src=\"A-->B\">"), "{h}");
        assert!(h.contains("language-rust"), "{h}");
        assert!(h.contains("src=\"data:image/png;base64,p.png\""), "{h}");
    }

    #[test]
    fn title_is_escaped() {
        let d = document("x", "<script>");
        assert!(d.contains("<title>&lt;script&gt;</title>"));
        assert!(!d.contains("<script>"));
    }
}
