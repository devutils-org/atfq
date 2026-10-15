//! The view model: a resolved query plus live probe results. The CLI renders
//! it as text, `--json` serializes it as is, and later surfaces reuse it.

use serde::Serialize;

use crate::content::{ChapterKind, Library};
use crate::probes::{self, ProbeCtx, ProbeResult};
use crate::resolve::{resolve, Resolved, Target, MAX_SUGGESTIONS};
use crate::search::search;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Match {
    Exact,
    /// No exact match: `entry` is the closest one.
    Closest,
    None,
}

/// A short reference to a hub or a card.
#[derive(Debug, Clone, Serialize)]
pub struct Ref {
    pub id: String,
    pub kind: &'static str,
    /// What to type to open it: a hub id or a chapter number.
    pub handle: String,
    pub title: String,
}

impl From<Target<'_>> for Ref {
    fn from(t: Target<'_>) -> Ref {
        Ref {
            id: t.id().to_string(),
            kind: t.kind(),
            handle: t.handle(),
            title: t.title().to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Lookup<'a> {
    pub query: String,
    #[serde(rename = "match")]
    pub matched: Match,
    pub entry: Option<Target<'a>>,
    /// A card that shares the hub's name.
    pub also_card: Option<Ref>,
    /// "Did you mean" next to a closest match; search results when nothing matched.
    pub suggestions: Vec<Ref>,
    pub probes: Vec<ProbeResult>,
}

pub fn lookup<'a>(lib: &'a Library, query: &str, ctx: &ProbeCtx) -> Lookup<'a> {
    let (matched, entry, also_card, suggestions) = match resolve(lib, query) {
        Resolved::Exact { target, also_card } => (
            Match::Exact,
            Some(target),
            also_card.map(|c| Target::Card(c).into()),
            Vec::new(),
        ),
        Resolved::Fuzzy { target, others } => (
            Match::Closest,
            Some(target),
            None,
            others.into_iter().map(Ref::from).collect(),
        ),
        Resolved::NotFound => {
            let hits = search(lib, query, MAX_SUGGESTIONS);
            let refs = hits
                .into_iter()
                .map(|h| Ref {
                    id: h.id,
                    kind: h.kind,
                    handle: h.handle,
                    title: h.title,
                })
                .collect();
            (Match::None, None, None, refs)
        }
    };
    let probes = match entry {
        Some(Target::Hub(h)) => probes::run_all(&h.probes, ctx),
        Some(Target::Card(c)) => probes::run_all(&c.probes, ctx),
        None => Vec::new(),
    };
    Lookup {
        query: query.to_string(),
        matched,
        entry,
        also_card,
        suggestions,
        probes,
    }
}

/// The table of contents, for JSON and MCP.
#[derive(Debug, Serialize)]
pub struct Toc {
    pub hubs: Vec<Ref>,
    pub chapters: Vec<TocChapter>,
}

#[derive(Debug, Serialize)]
pub struct TocChapter {
    pub number: u32,
    pub title: String,
    /// The part of the book, when there are parts.
    pub part: Option<String>,
    /// The hub that serves as this chapter's page, if any.
    pub hub: Option<String>,
    pub cards: Vec<Ref>,
}

pub fn toc(lib: &Library) -> Toc {
    Toc {
        hubs: lib.hubs().map(|h| Target::Hub(h).into()).collect(),
        chapters: lib
            .chapters()
            .iter()
            .map(|ch| TocChapter {
                number: ch.number,
                title: ch.title.clone(),
                part: ch.part.clone(),
                hub: match &ch.kind {
                    ChapterKind::Hub(id) => Some(id.clone()),
                    ChapterKind::Plain => None,
                },
                cards: ch
                    .cards
                    .iter()
                    .filter_map(|id| lib.card(id))
                    .map(|c| Target::Card(c).into())
                    .collect(),
            })
            .collect(),
    }
}
