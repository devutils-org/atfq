//! The document model every page is built in: lines of styled spans, some
//! of them actionable. The CLI prints it as ANSI text; the TUI draws it as
//! widgets, with clickable links.

/// How a span looks. Surfaces map this to ANSI codes or widget styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Bold,
    Emph,
    Code,
    Dim,
    Warn,
}

/// What activating a span does.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    /// Open a hub or a card, by what a user would type (`memory`, `5.1`).
    Open(String),
    /// Expand or collapse a deep dive question, by index.
    Toggle(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub kind: Kind,
    pub action: Option<Action>,
}

impl Span {
    pub fn new(text: impl Into<String>, kind: Kind) -> Span {
        Span {
            text: text.into(),
            kind,
            action: None,
        }
    }

    pub fn plain(text: impl Into<String>) -> Span {
        Span::new(text, Kind::Plain)
    }

    pub fn with(mut self, action: Action) -> Span {
        self.action = Some(action);
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub spans: Vec<Span>,
}

impl Line {
    pub fn new(spans: Vec<Span>) -> Line {
        Line { spans }
    }

    pub fn styled(text: impl Into<String>, kind: Kind) -> Line {
        Line::new(vec![Span::new(text, kind)])
    }

    pub fn plain(text: impl Into<String>) -> Line {
        Line::styled(text, Kind::Plain)
    }

    pub fn spaces(n: usize) -> Line {
        if n == 0 {
            Line::default()
        } else {
            Line::plain(" ".repeat(n))
        }
    }

    /// `self` followed by `other`.
    pub fn then(mut self, other: Line) -> Line {
        self.spans.extend(other.spans);
        self
    }

    pub fn push(&mut self, span: Span) {
        self.spans.push(span);
    }

    /// Plain parts turn dim; styled parts keep their style.
    pub fn dimmed(mut self) -> Line {
        for s in &mut self.spans {
            if s.kind == Kind::Plain {
                s.kind = Kind::Dim;
            }
        }
        self
    }

    pub fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }

    pub fn width(&self) -> usize {
        self.spans.iter().map(|s| width(&s.text)).sum()
    }

    fn trim_end(mut self) -> Line {
        while let Some(last) = self.spans.last_mut() {
            let trimmed = last.text.trim_end().len();
            if trimmed == 0 {
                self.spans.pop();
            } else {
                last.text.truncate(trimmed);
                break;
            }
        }
        self
    }

    fn to_ansi(&self, color: bool) -> String {
        let mut out = String::new();
        for s in &self.spans {
            let code = match s.kind {
                Kind::Plain => None,
                Kind::Bold => Some("1"),
                Kind::Emph => Some("3"),
                Kind::Code => Some("36"),
                Kind::Dim => Some("2"),
                Kind::Warn => Some("33"),
            };
            match code {
                Some(c) if color && !s.text.is_empty() => {
                    out.push_str(&format!("\x1b[{c}m{}\x1b[0m", s.text));
                }
                _ => out.push_str(&s.text),
            }
        }
        out
    }
}

/// A rendered page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Doc {
    pub lines: Vec<Line>,
}

impl Doc {
    /// Terminal text, one `\n` after each line; ANSI styles only if `color`.
    pub fn to_text(&self, color: bool) -> String {
        let mut s = String::new();
        for l in &self.lines {
            s.push_str(&l.to_ansi(color));
            s.push('\n');
        }
        s
    }
}

/// Visible width. Content is mostly ASCII; one column per char is close enough.
pub fn width(s: &str) -> usize {
    s.chars().count()
}

/// Wraps `spans` to `max` columns, starting lines with `first` then `rest`.
/// Words break on whitespace; inline code never breaks, so a command stays
/// copy-pasteable.
pub(crate) fn wrap(spans: &[Span], first: &Line, rest: &Line, max: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut line = first.clone();
    let mut len = first.width();
    let mut empty = true;
    let mut prev_action: Option<Action> = None;
    for word in words(spans) {
        let w: usize = word.iter().map(|s| width(&s.text)).sum();
        if !empty && len + 1 + w > max {
            lines.push(std::mem::replace(&mut line, rest.clone()).trim_end());
            len = rest.width();
            empty = true;
        }
        if !empty {
            // A space inside a multi-word link belongs to the link.
            let first_action = word.first().and_then(|s| s.action.clone());
            let mut space = Span::plain(" ");
            if first_action.is_some() && first_action == prev_action {
                space.action = first_action;
            }
            line.push(space);
            len += 1;
        }
        prev_action = word.last().and_then(|s| s.action.clone());
        line.spans.extend(word);
        len += w;
        empty = false;
    }
    if !empty {
        lines.push(line.trim_end());
    }
    lines
}

/// Splits spans into words. A word can mix styles (`**free**:`).
fn words(spans: &[Span]) -> Vec<Vec<Span>> {
    fn push(word: &mut Vec<Span>, text: &str, from: &Span) {
        match word.last_mut() {
            Some(last) if last.kind == from.kind && last.action == from.action => {
                last.text.push_str(text)
            }
            _ => word.push(Span {
                text: text.to_string(),
                kind: from.kind,
                action: from.action.clone(),
            }),
        }
    }
    let mut words = Vec::new();
    let mut word: Vec<Span> = Vec::new();
    for span in spans {
        if span.kind == Kind::Code {
            push(&mut word, &span.text, span);
            continue;
        }
        for ch in span.text.chars() {
            if ch.is_whitespace() {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            } else {
                push(&mut word, ch.encode_utf8(&mut [0; 4]), span);
            }
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(lines: &[Line]) -> Vec<String> {
        lines.iter().map(Line::text).collect()
    }

    #[test]
    fn wraps_with_hanging_indent() {
        let lines = wrap(
            &[Span::plain("one two three four five")],
            &Line::plain("- "),
            &Line::spaces(2),
            12,
        );
        assert_eq!(texts(&lines), ["- one two", "  three four", "  five"]);
    }

    #[test]
    fn keeps_inline_code_whole_and_glues_punctuation() {
        let spans = vec![
            Span::plain("run "),
            Span::new("free -h", Kind::Code),
            Span::plain(", then look"),
        ];
        let lines = wrap(&spans, &Line::default(), &Line::default(), 9);
        assert_eq!(texts(&lines), ["run", "free -h,", "then look"]);
    }

    #[test]
    fn colors_only_when_asked() {
        let doc = Doc {
            lines: vec![Line::styled("x", Kind::Bold)],
        };
        assert_eq!(doc.to_text(false), "x\n");
        assert_eq!(doc.to_text(true), "\x1b[1mx\x1b[0m\n");
    }

    #[test]
    fn nothing_to_wrap() {
        assert!(wrap(&[Span::plain("  ")], &Line::spaces(2), &Line::spaces(2), 80).is_empty());
    }

    #[test]
    fn links_keep_their_action_across_words() {
        let open = Action::Open("5.1".into());
        let spans = vec![
            Span::plain("see "),
            Span::plain("free vs available").with(open.clone()),
        ];
        let lines = wrap(&spans, &Line::default(), &Line::default(), 80);
        let linked: String = lines[0]
            .spans
            .iter()
            .filter(|s| s.action.as_ref() == Some(&open))
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(linked, "free vs available");
    }
}
