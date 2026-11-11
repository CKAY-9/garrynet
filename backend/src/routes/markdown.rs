//! Markdown sanitizing for user-written content (forum posts). The Markdown source is
//! stored and the frontend renders it, so this keeps the source itself safe: raw HTML is
//! escaped (it shows up as text), and links or images to anything but http(s), mailto or
//! relative URLs are rejected.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag};

use crate::{
    error::ApiError,
    routes::validate::{bad, required_text},
};

/// GitHub-flavored extensions, matching the frontend's remark-gfm.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
}

/// Escaping can expose new syntax (a `<` that was inside an HTML block may now start a
/// tag on its own), so it repeats until nothing changes. Real input settles in one or two.
const MAX_PASSES: usize = 4;

/// Required Markdown of at most `max` characters, with raw HTML escaped. 400 if a link or
/// image uses an unsafe URL.
pub fn markdown(field: &str, value: &str, max: usize) -> Result<String, ApiError> {
    let mut source = required_text(field, value, max)?;
    for _ in 0..MAX_PASSES {
        let html = html_ranges(&source);
        if html.is_empty() {
            check_urls(field, &source)?;
            return Ok(source);
        }
        source = escape_html(&source, &html);
    }
    Err(bad(format!("{field} contains HTML that couldn't be escaped")))
}

/// 400 unless every image in `source` (already checked by [`markdown`]) is a file served
/// by this backend, i.e. its URL starts with `files_base` (`{BACKEND_URL}/file/`). For
/// support tickets: an image from elsewhere would load from that host when staff open the
/// ticket, telling whoever runs it their IP address and when they read it. Links are left
/// alone; following one is the reader's choice.
pub fn local_images_only(field: &str, source: &str, files_base: &str) -> Result<(), ApiError> {
    for event in Parser::new_ext(source, options()) {
        if let Event::Start(Tag::Image { dest_url, .. }) = event
            && !dest_url.starts_with(files_base)
        {
            return Err(bad(format!(
                "{field} can only show images uploaded here; attach the image instead of linking {:?}",
                dest_url.as_ref()
            )));
        }
    }
    Ok(())
}

/// Byte ranges of raw HTML (blocks and inline tags), in source order.
fn html_ranges(source: &str) -> Vec<Range<usize>> {
    Parser::new_ext(source, options())
        .into_offset_iter()
        .filter(|(event, _)| matches!(event, Event::Html(_) | Event::InlineHtml(_)))
        .map(|(_, range)| range)
        .collect()
}

/// Backslash-escapes every `<` in `ranges`, so the HTML renders as literal text.
fn escape_html(source: &str, ranges: &[Range<usize>]) -> String {
    let mut out = String::with_capacity(source.len() + ranges.len() * 2);
    let mut last = 0;
    for range in ranges {
        // Ranges never overlap, but guard so a parser quirk can't panic the slice.
        let start = range.start.max(last);
        out.push_str(&source[last..start]);
        out.push_str(&source[start..range.end].replace('<', "\\<"));
        last = range.end;
    }
    out.push_str(&source[last..]);
    out
}

fn check_urls(field: &str, source: &str) -> Result<(), ApiError> {
    for event in Parser::new_ext(source, options()) {
        if let Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) = event
            && !safe_url(&dest_url)
        {
            return Err(bad(format!(
                "{field} links must be http(s), mailto or relative, got {:?}",
                dest_url.as_ref()
            )));
        }
    }
    Ok(())
}

/// Relative URLs, or absolute ones with an http(s)/mailto scheme. Browsers ignore
/// whitespace and control characters in a scheme (`java\tscript:`), so this does too.
fn safe_url(url: &str) -> bool {
    let cleaned: String = url.chars().filter(|c| !c.is_whitespace() && !c.is_control()).collect();
    match cleaned.find([':', '/', '?', '#']) {
        Some(i) if cleaned[i..].starts_with(':') => {
            matches!(cleaned[..i].to_ascii_lowercase().as_str(), "http" | "https" | "mailto")
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(value: &str) -> Result<String, ApiError> {
        markdown("content", value, 20_000)
    }

    #[test]
    fn keeps_markdown() {
        let src = "# Title\n\n**bold** _it_ ~~no~~ `a < b`\n\n- [x] done\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```\n<script>\n```";
        assert_eq!(md(src).unwrap(), src);
        assert_eq!(md("1 < 2 and <3").unwrap(), "1 < 2 and <3");
    }

    #[test]
    fn escapes_html() {
        assert_eq!(md("hi <b>there</b>").unwrap(), "hi \\<b>there\\</b>");
        assert_eq!(md("<script>alert(1)</script>").unwrap(), "\\<script>alert(1)\\</script>");
        let out = md("<div>\n<img src=x onerror=alert(1)>\n</div>").unwrap();
        assert!(html_ranges(&out).is_empty(), "{out}");
        let out = md("> <iframe src=\"https://x\">").unwrap();
        assert!(html_ranges(&out).is_empty(), "{out}");
    }

    #[test]
    fn rejects_unsafe_urls() {
        assert!(md("[x](javascript:alert(1))").is_err());
        assert!(md("[x](JaVaScRiPt:alert(1))").is_err());
        assert!(md("[x](<java\tscript:alert(1)>)").is_err());
        assert!(md("[x](javascript&#58;alert(1))").is_err());
        assert!(md("![x](data:image/svg+xml,abc)").is_err());
        assert!(md("<javascript:alert(1)>").is_err());
        assert!(md("[x]\n\n[x]: vbscript:msgbox").is_err());
        // An HTML block hides the link until the HTML is escaped.
        assert!(md("<div>\n[x](javascript:alert(1))\n</div>").is_err());
    }

    #[test]
    fn allows_safe_urls() {
        for src in [
            "[x](https://example.com)",
            "[x](http://example.com/a:b)",
            "[x](mailto:a@b.c)",
            "[x](/forum/1)",
            "[x](#post-2)",
            "[x](?page=2)",
            "<https://example.com>",
            "<a@b.c>",
            "![img](https://example.com/a.png)",
        ] {
            assert!(md(src).is_ok(), "{src}");
        }
    }

    #[test]
    fn local_images() {
        let base = "https://api.example.com/file/";
        let ok = |src: &str| local_images_only("content", &md(src).unwrap(), base).is_ok();
        assert!(ok("![x](https://api.example.com/file/1/abc)"));
        assert!(ok("[a link](https://elsewhere.example)"), "links are fine");
        assert!(ok("no images at all"));
        assert!(!ok("![x](https://evil.example/pixel.png)"));
        assert!(!ok("![x](https://api.example.com.evil.example/file/1/abc)"));
        assert!(!ok("![x](//evil.example/pixel.png)"));
        assert!(!ok("![x](/file/1/abc)"), "relative to the frontend, not ours");
        assert!(!ok("![x][p]\n\n[p]: https://evil.example/pixel.png"));
    }

    #[test]
    fn blank_or_long() {
        assert!(md("  ").is_err());
        assert!(markdown("content", "abc", 2).is_err());
    }
}
