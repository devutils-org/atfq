//! Colours and borders of the interactive mode, in one place. Only the 16
//! ANSI colours are used, so the terminal's own theme decides the exact shades
//! and light terminals stay readable.

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::BorderType;

use crate::render::Kind;

pub const BORDER: BorderType = BorderType::Rounded;

/// Focused borders, titles, the current page, code.
pub const ACCENT: Color = Color::Cyan;
/// Borders out of focus, hints, key labels.
pub const MUTED: Color = Color::DarkGray;
pub const WARN: Color = Color::Yellow;

/// The colour of each mode's badge in the status bar.
pub const NORMAL: Color = Color::Cyan;
pub const SEARCH: Color = Color::Magenta;
pub const PROMPT: Color = Color::Green;

pub fn border(focused: bool) -> Style {
    Style::default().fg(if focused { ACCENT } else { MUTED })
}

pub fn title(focused: bool) -> Style {
    if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    }
}

/// A mode badge: dark text on the mode's colour.
pub fn badge(color: Color) -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(color)
        .add_modifier(Modifier::BOLD)
}

pub fn key() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

pub fn muted() -> Style {
    Style::default().fg(MUTED)
}

/// The selected TOC entry, when the TOC has the focus.
pub fn selection() -> Style {
    badge(ACCENT)
}

pub fn of(kind: Kind) -> Style {
    let s = Style::default();
    match kind {
        Kind::Plain => s,
        Kind::Bold => s.add_modifier(Modifier::BOLD),
        Kind::Emph => s.add_modifier(Modifier::ITALIC),
        Kind::Code => s.fg(ACCENT),
        Kind::Dim => s.fg(MUTED),
        Kind::Warn => s.fg(WARN),
    }
}
