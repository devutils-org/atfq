//! From what the user typed to a hub or a card (docs/ROADMAP.md section 7).

use serde::Serialize;

use crate::content::{normalize, Card, ChapterKind, Hub, Library, Number};

/// Most "Did you mean" suggestions shown next to a fuzzy match.
pub const MAX_SUGGESTIONS: usize = 3;

/// A hub or a card.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target<'a> {
    Hub(&'a Hub),
    Card(&'a Card),
}

impl<'a> Target<'a> {
    pub fn id(&self) -> &'a str {
        match self {
            Target::Hub(h) => &h.id,
            Target::Card(c) => &c.id,
        }
    }

    pub fn title(&self) -> &'a str {
        match self {
            Target::Hub(h) => &h.title,
            Target::Card(c) => &c.title,
        }
    }

    /// What to type to come back here: a hub id or a chapter number.
    pub fn handle(&self) -> String {
        match self {
            Target::Hub(h) => h.id.clone(),
            Target::Card(c) => c.number.to_string(),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Target::Hub(_) => "hub",
            Target::Card(_) => "card",
        }
    }
}

#[derive(Debug)]
pub enum Resolved<'a> {
    /// Steps 1–3. `also_card` is a card sharing the hub's key, worth pointing to.
    Exact {
        target: Target<'a>,
        also_card: Option<&'a Card>,
    },
    /// Step 4: the closest match, and up to three other candidates.
    Fuzzy {
        target: Target<'a>,
        others: Vec<Target<'a>>,
    },
    NotFound,
}

/// Resolution order: hub id or alias, card id or alias, chapter number,
/// then fuzzy matching on ids, aliases and titles.
pub fn resolve<'a>(lib: &'a Library, query: &str) -> Resolved<'a> {
    if let Some(hub) = lib.hub_by_key(query) {
        return Resolved::Exact {
            target: Target::Hub(hub),
            also_card: lib.card_by_key(query),
        };
    }
    if let Some(card) = lib.card_by_key(query) {
        return exact(Target::Card(card));
    }
    match parse_number(query.trim()) {
        Some((chapter, Some(section))) => {
            if let Some(card) = lib.card_by_number(Number { chapter, section }) {
                return exact(Target::Card(card));
            }
        }
        // A bare chapter number opens the chapter's hub, when it has one.
        Some((chapter, None)) => {
            if let Some(ChapterKind::Hub(id)) = lib.chapter(chapter).map(|c| &c.kind) {
                if let Some(hub) = lib.hub(id) {
                    return exact(Target::Hub(hub));
                }
            }
        }
        None => {}
    }
    let mut close = fuzzy(lib, query).into_iter();
    match close.next() {
        Some(target) => Resolved::Fuzzy {
            target,
            others: close.take(MAX_SUGGESTIONS).collect(),
        },
        None => Resolved::NotFound,
    }
}

fn exact(target: Target<'_>) -> Resolved<'_> {
    Resolved::Exact {
        target,
        also_card: None,
    }
}

/// Hubs and cards whose id, alias or title is close to `query`, best first.
fn fuzzy<'a>(lib: &'a Library, query: &str) -> Vec<Target<'a>> {
    let q = normalize(query);
    if q.chars().count() < 3 {
        return Vec::new();
    }
    let mut scored: Vec<(usize, usize, Target<'a>)> = Vec::new();
    let mut consider = |target: Target<'a>, keys: Vec<&str>| {
        let best = keys
            .into_iter()
            .filter_map(|k| cost(&q, &normalize(k)))
            .min();
        if let Some(c) = best {
            // Ties: hubs before cards, then table of contents order.
            let order = match target {
                Target::Hub(h) => h.chapter as usize,
                Target::Card(c) => 1000 + (c.number.chapter * 1000 + c.number.section) as usize,
            };
            scored.push((c, order, target));
        }
    };
    for h in lib.hubs() {
        let keys = [h.id.as_str(), h.title.as_str()]
            .into_iter()
            .chain(h.aliases.iter().map(String::as_str))
            .collect();
        consider(Target::Hub(h), keys);
    }
    for c in lib.cards() {
        let keys = [c.id.as_str(), c.title.as_str()]
            .into_iter()
            .chain(c.aliases.iter().map(String::as_str))
            .collect();
        consider(Target::Card(c), keys);
    }
    scored.sort_by_key(|&(c, order, _)| (c, order));
    scored.into_iter().map(|(_, _, t)| t).collect()
}

/// How far `key` is from `query`, if close enough to suggest.
fn cost(query: &str, key: &str) -> Option<usize> {
    let len = query.chars().count();
    let max = match len {
        0..=4 => 1,
        5..=8 => 2,
        _ => 3,
    };
    let d = distance(query, key);
    if d <= max {
        Some(d)
    } else if key.starts_with(query) {
        // `zomb` → `zombie`: a typed prefix is a weak but real hint.
        Some(max)
    } else {
        None
    }
}

/// Edit distance where swapping two neighbours counts as one edit
/// (optimal string alignment), so `memroy` is 1 away from `memory`.
pub(crate) fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let sub = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + sub);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = v;
        }
    }
    d[a.len()][b.len()]
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
    use super::*;

    #[test]
    fn chapter_numbers() {
        assert_eq!(parse_number("5.1"), Some((5, Some(1))));
        assert_eq!(parse_number("12"), Some((12, None)));
        for bad in ["", "5.", ".1", "5.1.2", "a.1", "-1", "+5"] {
            assert_eq!(parse_number(bad), None, "{bad}");
        }
    }

    #[test]
    fn distances() {
        assert_eq!(distance("memroy", "memory"), 1);
        assert_eq!(distance("dsk", "disk"), 1);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("zombie", "zombie"), 0);
        assert_eq!(distance("kitten", "sitting"), 3);
    }

    #[test]
    fn costs() {
        assert_eq!(cost("memroy", "memory"), Some(1));
        assert_eq!(cost("zomb", "zombie"), Some(1));
        assert_eq!(cost("disk", "find"), None);
        // Short queries tolerate a single edit only.
        assert_eq!(cost("mem", "ram"), None);
    }
}
