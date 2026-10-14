//! Styling and word wrapping for terminal text.

/// ANSI styling, or nothing at all when color is off.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    color: bool,
}

impl Style {
    pub fn new(color: bool) -> Style {
        Style { color }
    }

    fn paint(&self, ansi: &str, s: &str) -> String {
        if self.color && !s.is_empty() {
            format!("\x1b[{ansi}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    pub fn bold(&self, s: &str) -> String {
        self.paint("1", s)
    }

    pub fn emph(&self, s: &str) -> String {
        self.paint("3", s)
    }

    pub fn code(&self, s: &str) -> String {
        self.paint("36", s)
    }

    pub fn dim(&self, s: &str) -> String {
        self.paint("2", s)
    }

    pub fn warn(&self, s: &str) -> String {
        self.paint("33", s)
    }

    pub(crate) fn span(&self, s: &str, kind: Kind) -> String {
        match kind {
            Kind::Plain => s.to_string(),
            Kind::Bold => self.bold(s),
            Kind::Emph => self.emph(s),
            Kind::Code => self.code(s),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Plain,
    Bold,
    Emph,
    Code,
}

/// A run of inline text with one style.
#[derive(Debug, Clone)]
pub(crate) struct Span {
    pub text: String,
    pub kind: Kind,
}

impl Span {
    pub fn new(text: impl Into<String>, kind: Kind) -> Span {
        Span {
            text: text.into(),
            kind,
        }
    }
}

/// What starts a wrapped line: possibly styled text, and its visible width.
#[derive(Debug, Clone)]
pub(crate) struct Prefix {
    pub text: String,
    pub width: usize,
}

impl Prefix {
    pub fn spaces(n: usize) -> Prefix {
        Prefix {
            text: " ".repeat(n),
            width: n,
        }
    }

    pub fn plain(s: &str) -> Prefix {
        Prefix {
            text: s.to_string(),
            width: width(s),
        }
    }
}

/// Visible width. Content is mostly ASCII; one column per char is close enough.
pub(crate) fn width(s: &str) -> usize {
    s.chars().count()
}

/// Wraps `spans` to `max` columns. Words break on whitespace; inline code is
/// never broken, so a command stays copy-pasteable.
pub(crate) fn wrap(
    spans: &[Span],
    first: &Prefix,
    rest: &Prefix,
    max: usize,
    st: Style,
) -> Vec<String> {
    let words = words(spans);
    let mut lines = Vec::new();
    let mut line = first.text.clone();
    let mut len = first.width;
    let mut empty = true;
    for word in words {
        let w: usize = word.iter().map(|s| width(&s.text)).sum();
        if !empty && len + 1 + w > max {
            lines.push(std::mem::replace(&mut line, rest.text.clone()));
            len = rest.width;
            empty = true;
        }
        if !empty {
            line.push(' ');
            len += 1;
        }
        for s in &word {
            line.push_str(&st.span(&s.text, s.kind));
        }
        len += w;
        empty = false;
    }
    if !empty {
        lines.push(line);
    }
    lines
}

/// Splits spans into words. A word can mix styles (`**free**:`).
fn words(spans: &[Span]) -> Vec<Vec<Span>> {
    fn push(word: &mut Vec<Span>, text: &str, kind: Kind) {
        match word.last_mut() {
            Some(last) if last.kind == kind => last.text.push_str(text),
            _ => word.push(Span::new(text, kind)),
        }
    }
    let mut words = Vec::new();
    let mut word: Vec<Span> = Vec::new();
    for span in spans {
        if span.kind == Kind::Code {
            push(&mut word, &span.text, Kind::Code);
            continue;
        }
        for ch in span.text.chars() {
            if ch.is_whitespace() {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            } else {
                push(&mut word, ch.encode_utf8(&mut [0; 4]), span.kind);
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

    fn plain(s: &str) -> Vec<Span> {
        vec![Span::new(s, Kind::Plain)]
    }

    #[test]
    fn wraps_with_hanging_indent() {
        let lines = wrap(
            &plain("one two three four five"),
            &Prefix::plain("- "),
            &Prefix::spaces(2),
            12,
            Style::new(false),
        );
        assert_eq!(lines, ["- one two", "  three four", "  five"]);
    }

    #[test]
    fn keeps_inline_code_whole_and_glues_punctuation() {
        let spans = vec![
            Span::new("run ", Kind::Plain),
            Span::new("free -h", Kind::Code),
            Span::new(", then look", Kind::Plain),
        ];
        let lines = wrap(
            &spans,
            &Prefix::spaces(0),
            &Prefix::spaces(0),
            9,
            Style::new(false),
        );
        assert_eq!(lines, ["run", "free -h,", "then look"]);
    }

    #[test]
    fn colors_only_when_asked() {
        let spans = vec![Span::new("x", Kind::Bold)];
        let p = Prefix::spaces(0);
        assert_eq!(wrap(&spans, &p, &p, 80, Style::new(false)), ["x"]);
        assert_eq!(
            wrap(&spans, &p, &p, 80, Style::new(true)),
            ["\x1b[1mx\x1b[0m"]
        );
    }

    #[test]
    fn nothing_to_wrap() {
        let p = Prefix::spaces(2);
        assert!(wrap(&plain("  "), &p, &p, 80, Style::new(false)).is_empty());
    }
}
