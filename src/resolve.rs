//! From what the user typed to a hub or a card (ROADMAP.md section 7).

use crate::content::{Card, ChapterKind, Hub, Library, Number};

#[derive(Debug)]
pub enum Resolved<'a> {
    /// A hub. `also_card` is a card that shares the same key, worth pointing to.
    Hub {
        hub: &'a Hub,
        also_card: Option<&'a Card>,
    },
    Card(&'a Card),
    NotFound,
}

/// Resolution order: hub id or alias, card id or alias, chapter number.
/// Fuzzy matching comes in phase 1.
pub fn resolve<'a>(lib: &'a Library, query: &str) -> Resolved<'a> {
    if let Some(hub) = lib.hub_by_key(query) {
        return Resolved::Hub {
            hub,
            also_card: lib.card_by_key(query),
        };
    }
    if let Some(card) = lib.card_by_key(query) {
        return Resolved::Card(card);
    }
    match parse_number(query.trim()) {
        Some((chapter, Some(section))) => lib
            .card_by_number(Number { chapter, section })
            .map_or(Resolved::NotFound, Resolved::Card),
        // A bare chapter number opens the chapter's hub, when it has one.
        Some((chapter, None)) => match lib.chapter(chapter).map(|c| &c.kind) {
            Some(ChapterKind::Hub(id)) => {
                lib.hub(id).map_or(Resolved::NotFound, |hub| Resolved::Hub {
                    hub,
                    also_card: None,
                })
            }
            _ => Resolved::NotFound,
        },
        None => Resolved::NotFound,
    }
}

/// Parses `5` or `5.1`.
pub fn parse_number(s: &str) -> Option<(u32, Option<u32>)> {
    let digits = |p: &str| {
        (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            .then(|| p.parse().ok())
            .flatten()
    };
    match s.split_once('.') {
        Some((c, n)) => Some((digits(c)?, Some(digits(n)?))),
        None => Some((digits(s)?, None)),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_number;

    #[test]
    fn chapter_numbers() {
        assert_eq!(parse_number("5.1"), Some((5, Some(1))));
        assert_eq!(parse_number("12"), Some((12, None)));
        for bad in ["", "5.", ".1", "5.1.2", "a.1", "-1", "+5"] {
            assert_eq!(parse_number(bad), None, "{bad}");
        }
    }
}
