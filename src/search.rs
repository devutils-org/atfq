//! Full-text search over hubs and cards (`idk search`).
//!
//! Each query word scores in the best field it appears in: a title is worth
//! more than a deep dive. A word matches a whole word (full score), or at
//! half score the start of one (3 letters or more) or a word one typo away
//! (5 letters or more).

use std::cmp::Reverse;

use serde::Serialize;

use crate::content::Library;
use crate::resolve::{distance, Target};

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub id: String,
    pub kind: &'static str,
    /// What to type to open it: a hub id or a chapter number.
    pub handle: String,
    pub title: String,
    pub score: u32,
    /// The line that matched, when it isn't the title or an alias.
    pub snippet: Option<String>,
}

const TITLE: u32 = 10;
const KEYS: u32 = 8;
const REFS: u32 = 4;
const TLDR: u32 = 3;
const PITFALLS: u32 = 2;
const DEEP: u32 = 1;

const SNIPPET_MAX: usize = 64;

/// Field weight and text.
type Fields = Vec<(u32, String)>;

pub fn search(lib: &Library, query: &str, limit: usize) -> Vec<Hit> {
    let terms = words(query);
    if terms.is_empty() {
        return Vec::new();
    }
    // (words matched, score, table of contents order, hit)
    let mut hits: Vec<(usize, u32, usize, Hit)> = Vec::new();
    let mut order = 0;
    let mut add = |target: Target<'_>, fields: Fields| {
        order += 1;
        let mut matched = 0;
        let mut score = 0;
        for t in &terms {
            let best = fields.iter().map(|(w, text)| w * term_score(t, text)).max();
            if let Some(s) = best.filter(|&s| s > 0) {
                matched += 1;
                score += s;
            }
        }
        if matched == 0 {
            return;
        }
        hits.push((
            matched,
            score,
            order,
            Hit {
                id: target.id().to_string(),
                kind: target.kind(),
                handle: target.handle(),
                title: target.title().to_string(),
                score,
                snippet: snippet(&terms, &fields),
            },
        ));
    };

    for h in lib.hubs() {
        let mut f = vec![(TITLE, h.title.clone()), (KEYS, keys(&h.id, &h.aliases))];
        for c in &h.commands {
            f.push((REFS, format!("{}: {}", c.cmd, c.desc)));
        }
        for p in &h.paths {
            f.push((REFS, format!("{}: {}", p.path, p.desc)));
        }
        f.push((TLDR, h.intro.clone()));
        add(Target::Hub(h), f);
    }
    for c in lib.cards() {
        let mut f = vec![
            (TITLE, c.title.clone()),
            (KEYS, keys(&c.id, &c.aliases)),
            (REFS, c.commands.join(" ")),
            (REFS, c.paths.join(" ")),
            (TLDR, c.tldr.clone()),
        ];
        if let Some(p) = &c.pitfalls {
            f.push((PITFALLS, p.clone()));
        }
        for q in &c.deep_dive {
            f.push((DEEP, format!("{}\n{}", q.title, q.body)));
        }
        add(Target::Card(c), f);
    }

    // Most words matched first, then best score, then reading order.
    hits.sort_by_key(|&(matched, score, order, _)| (Reverse(matched), Reverse(score), order));
    hits.into_iter().take(limit).map(|(.., h)| h).collect()
}

fn keys(id: &str, aliases: &[String]) -> String {
    format!("{id} {}", aliases.join(" "))
}

/// Lowercase words: letters and digits only, so `/proc/meminfo` gives
/// `proc` and `meminfo`.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// 2 for a whole word, 1 for a prefix or a typo, 0 otherwise.
fn term_score(term: &str, text: &str) -> u32 {
    let len = term.chars().count();
    let mut best = 0;
    for w in words(text) {
        if w == term {
            return 2;
        }
        if (len >= 3 && w.starts_with(term)) || (len >= 5 && distance(term, &w) <= 1) {
            best = 1;
        }
    }
    best
}

/// The first line that holds a query word, cleaned of Markdown marks: from
/// the text first (TL;DR, pitfalls, deep dive), then commands and paths.
fn snippet(terms: &[String], fields: &Fields) -> Option<String> {
    let mut body: Vec<&(u32, String)> = fields.iter().filter(|(w, _)| *w < KEYS).collect();
    body.sort_by_key(|&&(w, _)| (w == REFS, Reverse(w)));
    let line = body
        .iter()
        .flat_map(|(_, text)| text.lines())
        .find(|line| terms.iter().any(|t| term_score(t, line) > 0))?;
    let clean: String = line
        .trim()
        .trim_start_matches("- ")
        .trim_start_matches("### ")
        .replace(['*', '`'], "");
    Some(shorten(clean.trim(), SNIPPET_MAX))
}

fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max - 1).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_words() {
        assert_eq!(
            words("/proc/meminfo vm.swappiness"),
            ["proc", "meminfo", "vm", "swappiness"]
        );
    }

    #[test]
    fn scores_terms() {
        assert_eq!(term_score("swap", "no swap here"), 2);
        assert_eq!(term_score("swap", "swappiness"), 1);
        assert_eq!(term_score("sw", "swappiness"), 0);
        assert_eq!(term_score("swap", "hotswap"), 0);
        assert_eq!(term_score("swapiness", "vm.swappiness"), 1);
        assert_eq!(term_score("swp", "swap"), 0);
    }

    #[test]
    fn shortens_long_lines() {
        assert_eq!(shorten("abcdef", 4), "abc…");
        assert_eq!(shorten("abc", 4), "abc");
    }
}
