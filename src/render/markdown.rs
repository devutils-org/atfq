//! Markdown card bodies → terminal lines.
//!
//! Covers what cards use: paragraphs, nested lists, bold, emphasis, inline
//! code and code blocks. Anything else is shown as plain text.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use super::text::{wrap, Kind, Line, Span};
use super::warning;
use crate::content::danger::is_dangerous;

/// Code blocks sit this many columns right of the surrounding text.
const CODE_INDENT: usize = 4;

pub(crate) fn render(src: &str, indent: usize, width: usize) -> Vec<Line> {
    let mut md = Md {
        width,
        out: Vec::new(),
        spans: Vec::new(),
        kinds: Vec::new(),
        indents: vec![indent],
        lists: Vec::new(),
        marker: None,
        code: None,
        blank: false,
    };
    for event in Parser::new(src) {
        md.event(event);
    }
    md.flush();
    md.out
}

struct Md {
    width: usize,
    out: Vec<Line>,
    /// Inline text waiting to be wrapped.
    spans: Vec<Span>,
    /// Open inline styles, innermost last.
    kinds: Vec<Kind>,
    /// Content indent of each open list item, base indent first.
    indents: Vec<usize>,
    /// Next number of each open list, `None` for bullets.
    lists: Vec<Option<u64>>,
    /// List marker for the first line of the current item.
    marker: Option<String>,
    /// Text of the code block being read.
    code: Option<String>,
    /// A blank line goes before the next output.
    blank: bool,
}

impl Md {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => match self.code.as_mut() {
                Some(code) => code.push_str(&t),
                None => self.text(&t),
            },
            Event::Code(t) => self.spans.push(Span::new(t.as_ref(), Kind::Code)),
            Event::SoftBreak | Event::HardBreak => self.text(" "),
            Event::Html(t) | Event::InlineHtml(t) => self.text(&t),
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::List(first) => {
                self.flush();
                self.lists.push(first);
            }
            Tag::Item => {
                self.flush();
                let marker = match self.lists.last_mut() {
                    Some(Some(n)) => {
                        *n += 1;
                        format!("{}. ", *n - 1)
                    }
                    _ => "- ".to_string(),
                };
                let parent = self.indent();
                self.indents.push(parent + marker.len());
                self.marker = Some(format!("{}{marker}", " ".repeat(parent)));
            }
            Tag::CodeBlock(_) => {
                self.flush();
                self.code = Some(String::new());
            }
            Tag::Strong => self.kinds.push(Kind::Bold),
            Tag::Emphasis => self.kinds.push(Kind::Emph),
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush();
                self.blank = true;
            }
            TagEnd::Item => {
                self.flush();
                self.indents.pop();
                self.marker = None;
            }
            TagEnd::List(_) => {
                self.lists.pop();
                self.blank = true;
            }
            TagEnd::CodeBlock => {
                let code = self.code.take().unwrap_or_default();
                self.code_block(&code);
                self.blank = true;
            }
            TagEnd::Strong | TagEnd::Emphasis => {
                self.kinds.pop();
            }
            _ => {}
        }
    }

    fn text(&mut self, t: &str) {
        let kind = self.kinds.last().copied().unwrap_or(Kind::Plain);
        self.spans.push(Span::new(t, kind));
    }

    fn indent(&self) -> usize {
        self.indents.last().copied().unwrap_or(0)
    }

    fn push_lines(&mut self, lines: Vec<Line>) {
        if lines.is_empty() {
            return;
        }
        if self.blank && !self.out.is_empty() {
            self.out.push(Line::default());
        }
        self.blank = false;
        self.out.extend(lines);
    }

    /// Wraps the pending inline text as one paragraph.
    fn flush(&mut self) {
        if self.spans.is_empty() {
            return;
        }
        let spans = std::mem::take(&mut self.spans);
        let rest = Line::spaces(self.indent());
        let first = self.marker.take().map_or(rest.clone(), Line::plain);
        let lines = wrap(&spans, &first, &rest, self.width);
        self.push_lines(lines);
    }

    /// Code is never wrapped. Dangerous lines get a warning under them.
    fn code_block(&mut self, code: &str) {
        let pad = self.indent() + CODE_INDENT;
        let mut lines = Vec::new();
        for line in code.trim_end_matches('\n').lines() {
            lines.push(Line::spaces(pad).then(Line::styled(line, Kind::Code)));
            if is_dangerous(line) {
                lines.push(Line::spaces(pad).then(warning()));
            }
        }
        self.push_lines(lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(src: &str) -> String {
        let lines: Vec<String> = render(src, 2, 40).iter().map(Line::text).collect();
        lines.join("\n")
    }

    #[test]
    fn tight_list_then_code() {
        let out = md("- **free**: nobody uses it.\n- **available**: what apps can get.\n\n```\nfree -h\n```\n");
        assert_eq!(
            out,
            "  - free: nobody uses it.\n  - available: what apps can get.\n\n      free -h"
        );
    }

    #[test]
    fn wraps_items_under_their_text() {
        let out = md("- a fairly long bullet that will need to wrap here\n");
        assert_eq!(
            out,
            "  - a fairly long bullet that will need\n    to wrap here"
        );
    }

    #[test]
    fn nested_and_ordered_lists() {
        let out = md("1. one\n2. two\n   - sub\n");
        assert_eq!(out, "  1. one\n  2. two\n     - sub");
    }

    #[test]
    fn paragraphs_are_separated() {
        assert_eq!(md("First.\n\nSecond.\n"), "  First.\n\n  Second.");
    }

    #[test]
    fn code_is_not_wrapped_and_danger_is_flagged() {
        let long = "find . -name '*.tmp' -type f -mtime +30 -delete";
        let out = md(&format!("```\n{long}\nls\n```\n"));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], format!("      {long}"));
        assert!(lines[1].contains('⚠'));
        assert_eq!(lines[2], "      ls");
    }
}
