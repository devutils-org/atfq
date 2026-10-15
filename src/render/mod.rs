//! Pages as [`Doc`]s: the CLI prints them as text, the TUI draws them.

mod live;
mod markdown;
mod text;

use std::collections::BTreeSet;

pub use text::{width, Action, Doc, Kind, Line, Span};

pub(crate) use text::wrap;

use crate::content::danger::is_dangerous;
use crate::content::{Card, ChapterKind, Hub, Library};
use crate::probes::{ProbeInfo, ProbeResult};
use crate::resolve::Target;
use crate::search::Hit;
use crate::view::{Lookup, Match, Ref};
use crate::BIN_NAME;

/// CLI output never goes wider than this.
pub const WIDTH: usize = 80;

/// Body text sits this far right of section headings.
const INDENT: usize = 2;

/// Widest left column in the Commands and Paths tables before the
/// description moves to its own line.
const MAX_LEFT: usize = 30;

/// CLI options.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Expand the deep dive.
    pub why: bool,
    pub color: bool,
}

/// How to lay a page out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub width: usize,
    pub deep_dive: DeepDive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepDive {
    /// A one-line hint pointing to `--why`.
    Hint,
    /// Every question with its answer.
    All,
    /// Every question as a toggle; the answers of these are shown.
    Toggle(BTreeSet<usize>),
}

impl Layout {
    pub fn cli(opts: Options) -> Layout {
        Layout {
            width: WIDTH,
            deep_dive: if opts.why {
                DeepDive::All
            } else {
                DeepDive::Hint
            },
        }
    }
}

pub(crate) fn warning() -> Line {
    Line::styled(
        "⚠ changes or deletes data: check what it targets first",
        Kind::Warn,
    )
}

/// Lines grouped in blocks separated by one blank line.
#[derive(Default)]
struct Page {
    lines: Vec<Line>,
}

impl Page {
    fn block(&mut self, lines: Vec<Line>) {
        if lines.is_empty() {
            return;
        }
        if !self.lines.is_empty() {
            self.lines.push(Line::default());
        }
        self.lines.extend(lines);
    }

    fn section(&mut self, heading: &str, body: Vec<Line>) {
        if body.is_empty() {
            return;
        }
        let mut lines = vec![Line::styled(heading, Kind::Bold)];
        lines.extend(body);
        self.block(lines);
    }

    fn finish(self) -> Doc {
        Doc { lines: self.lines }
    }
}

fn command(s: &str) -> String {
    format!("{BIN_NAME} {s}")
}

/// `atfq <handle>`, opening the page when activated.
fn open(handle: &str) -> Span {
    Span::new(command(handle), Kind::Code).with(Action::Open(handle.to_string()))
}

fn paragraph(spans: &[Span], width: usize) -> Vec<Line> {
    wrap(spans, &Line::default(), &Line::default(), width)
}

pub fn card_doc(lib: &Library, card: &Card, live: &[ProbeResult], layout: &Layout) -> Doc {
    let w = layout.width;
    let mut page = Page::default();
    page.block(vec![Line::styled(
        format!("{}  {}", card.number, card.title),
        Kind::Bold,
    )]);
    page.section("TL;DR", markdown::render(&card.tldr, INDENT, w));
    page.section("On your machine", live::block(live, w));
    if let Some(pitfalls) = &card.pitfalls {
        page.section("Pitfalls", markdown::render(pitfalls, INDENT, w));
    }

    let n = card.deep_dive.len();
    match &layout.deep_dive {
        _ if n == 0 => {}
        DeepDive::Hint => {
            let what = if n == 1 { "question" } else { "questions" };
            let spans = [
                Span::plain(format!("Deep dive: {n} {what}. Read them with ")),
                Span::new(command(&format!("{} --why", card.number)), Kind::Code),
            ];
            page.block(paragraph(&spans, w));
        }
        mode => {
            let mut lines = vec![Line::styled("Deep dive", Kind::Bold)];
            for (i, q) in card.deep_dive.iter().enumerate() {
                let open = match mode {
                    DeepDive::Toggle(set) => set.contains(&i),
                    _ => true,
                };
                let mark = if open { "▾ " } else { "▸ " };
                if open || i == 0 {
                    lines.push(Line::default());
                }
                let title = [Span::new(&q.title, Kind::Bold).with(Action::Toggle(i))];
                let first = Line::spaces(INDENT).then(Line::new(vec![
                    Span::new(mark, Kind::Bold).with(Action::Toggle(i))
                ]));
                lines.extend(wrap(&title, &first, &Line::spaces(INDENT + 2), w));
                if open {
                    lines.push(Line::default());
                    lines.extend(markdown::render(&q.body, INDENT * 2, w));
                }
            }
            page.block(lines);
        }
    }

    let mut related = Vec::new();
    let mut row = |label: &str, spans: Vec<Span>| {
        let first = Line::plain(format!("{}{label:<10}", " ".repeat(INDENT)));
        related.extend(wrap(&spans, &first, &Line::spaces(INDENT + 10), w));
    };
    if !card.commands.is_empty() {
        row("Commands", list(&card.commands));
    }
    if !card.paths.is_empty() {
        row("Paths", list(&card.paths));
    }
    for id in &card.hubs {
        if let Some(h) = lib.hub(id) {
            row("Hub", link(&h.id, &h.title));
        }
    }
    for id in &card.see_also {
        if let Some(h) = lib.hub(id) {
            row("See also", link(&h.id, &h.title));
        } else if let Some(c) = lib.card(id) {
            row("See also", link(&c.number.to_string(), &c.title));
        }
    }
    page.section("Related", related);
    page.finish()
}

pub fn card(lib: &Library, card: &Card, live: &[ProbeResult], opts: Options) -> String {
    card_doc(lib, card, live, &Layout::cli(opts)).to_text(opts.color)
}

/// `a, b, c` with each item styled as code.
fn list(items: &[String]) -> Vec<Span> {
    let mut spans = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let sep = if i + 1 < items.len() { "," } else { "" };
        spans.push(Span::new(item.as_str(), Kind::Code));
        spans.push(Span::plain(format!("{sep} ")));
    }
    spans
}

/// `atfq <handle>  Title`, all of it a link.
fn link(handle: &str, title: &str) -> Vec<Span> {
    let action = Action::Open(handle.to_string());
    vec![
        open(handle),
        Span::plain("  "),
        Span::plain(title).with(action),
    ]
}

/// One row of a two-column table: something to copy, and what it is.
struct Row {
    left: String,
    desc: String,
    action: Option<Action>,
}

impl Row {
    fn new(left: impl Into<String>, desc: impl Into<String>) -> Row {
        Row {
            left: left.into(),
            desc: desc.into(),
            action: None,
        }
    }

    fn opening(handle: &str, desc: &str) -> Row {
        Row {
            left: command(handle),
            desc: desc.to_string(),
            action: Some(Action::Open(handle.to_string())),
        }
    }
}

fn table(rows: &[Row], width: usize) -> Vec<Line> {
    // In a narrow pane, long commands go on their own line rather than
    // squeezing every description.
    let max_left = MAX_LEFT.min(width / 3);
    let left = rows
        .iter()
        .map(|r| text::width(&r.left))
        .filter(|&w| w <= max_left)
        .max()
        .unwrap_or(0);
    let desc_col = INDENT + left + 2;
    let rest = Line::spaces(desc_col);
    let mut lines = Vec::new();
    for r in rows {
        let w = text::width(&r.left);
        let mut cmd = Span::new(r.left.as_str(), Kind::Code);
        let mut desc = Span::plain(r.desc.as_str());
        cmd.action = r.action.clone();
        desc.action = r.action.clone();
        let styled = Line::spaces(INDENT).then(Line::new(vec![cmd]));
        if w <= max_left {
            let first = styled.then(Line::spaces(left - w + 2));
            lines.extend(wrap(&[desc], &first, &rest, width));
        } else {
            lines.push(styled);
            lines.extend(wrap(&[desc], &rest, &rest, width));
        }
        if is_dangerous(&r.left) {
            lines.push(Line::spaces(INDENT * 2).then(warning()));
        }
    }
    lines
}

pub fn hub_doc(
    lib: &Library,
    hub: &Hub,
    also_card: Option<&Card>,
    live: &[ProbeResult],
    layout: &Layout,
) -> Doc {
    let w = layout.width;
    let mut page = Page::default();
    if let Some(c) = also_card {
        let spans = [
            Span::plain(format!("Looking for \"{}\"? Run ", c.title)),
            open(&c.number.to_string()),
        ];
        page.block(paragraph(&spans, w));
    }
    page.block(vec![Line::new(vec![
        Span::new(format!("{}. {}", hub.chapter, hub.title), Kind::Bold),
        Span::plain("  "),
        Span::new("hub", Kind::Dim),
    ])]);
    page.block(markdown::render(&hub.intro, 0, w));
    page.section("On your machine", live::block(live, w));

    let commands: Vec<_> = hub
        .commands
        .iter()
        .map(|c| {
            let mut row = Row::new(&c.cmd, &c.desc);
            // The command stays what you copy; in the TUI it also opens its card.
            row.action = c
                .card
                .as_ref()
                .and_then(|id| lib.card(id))
                .map(|card| Action::Open(card.number.to_string()));
            row
        })
        .collect();
    page.section("Commands", table(&commands, w));
    let paths: Vec<_> = hub
        .paths
        .iter()
        .map(|p| Row::new(&p.path, &p.desc))
        .collect();
    page.section("Paths", table(&paths, w));

    // The hub's own chapter first, then related cards from elsewhere.
    let mut ids: Vec<&String> = lib
        .chapter(hub.chapter)
        .map(|c| c.cards.iter().collect())
        .unwrap_or_default();
    for id in &hub.cards {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    let cards: Vec<_> = ids
        .into_iter()
        .filter_map(|id| lib.card(id))
        .map(|c| Row::opening(&c.number.to_string(), &c.title))
        .collect();
    page.section("Cards", table(&cards, w));
    page.finish()
}

pub fn hub(
    lib: &Library,
    hub: &Hub,
    also_card: Option<&Card>,
    live: &[ProbeResult],
    opts: Options,
) -> String {
    hub_doc(lib, hub, also_card, live, &Layout::cli(opts)).to_text(opts.color)
}

pub fn toc(lib: &Library, opts: Options) -> String {
    let mut page = Page::default();
    page.block(vec![Line::styled("Contents", Kind::Bold)]);

    let hubs: Vec<_> = lib.hubs().map(|h| Row::opening(&h.id, &h.title)).collect();
    page.section("Hubs", table(&hubs, WIDTH));

    let digits = lib.chapters().len().to_string().len();
    let mut lines = Vec::new();
    let mut part: Option<&str> = None;
    let mut parts = 0;
    for ch in lib.chapters() {
        if lines.is_empty() || ch.part.as_deref() != part {
            part = ch.part.as_deref();
            if !lines.is_empty() {
                lines.push(Line::default());
            }
            let heading = match part {
                Some(title) => {
                    parts += 1;
                    format!("Part {} · {title}", roman(parts))
                }
                None => "Chapters".to_string(),
            };
            lines.push(Line::styled(heading, Kind::Bold));
        }
        let mut line = Line::plain(format!(
            "{}{:>digits$}. {}",
            " ".repeat(INDENT),
            ch.number,
            ch.title
        ));
        if let ChapterKind::Hub(id) = &ch.kind {
            line.push(Span::plain("  "));
            line.push(Span::new(format!("hub: {}", command(id)), Kind::Dim));
        }
        lines.push(line);
        for id in &ch.cards {
            if let Some(c) = lib.card(id) {
                let n = c.number.to_string();
                lines.push(Line::new(vec![
                    Span::plain(" ".repeat(INDENT + digits + 2)),
                    Span::new(&n, Kind::Code).with(Action::Open(n.clone())),
                    Span::plain("  "),
                    Span::plain(&c.title).with(Action::Open(n.clone())),
                ]));
            }
        }
    }
    page.block(lines);
    page.finish().to_text(opts.color)
}

/// 1 → I, 4 → IV: part numbers, as books write them.
pub fn roman(mut n: u32) -> String {
    const TABLE: &[(u32, &str)] = &[(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];
    let mut s = String::new();
    for &(value, letters) in TABLE {
        while n >= value {
            s.push_str(letters);
            n -= value;
        }
    }
    s
}

/// The page for a lookup: the hub or card, preceded by a note when it is only
/// the closest match. `None` when nothing matched: see [`not_found`].
pub fn lookup_doc(lib: &Library, l: &Lookup<'_>, layout: &Layout) -> Option<Doc> {
    let also_card = l.also_card.as_ref().and_then(|r| lib.card(&r.id));
    let mut doc = match l.entry? {
        Target::Hub(h) => hub_doc(lib, h, also_card, &l.probes, layout),
        Target::Card(c) => card_doc(lib, c, &l.probes, layout),
    };
    if l.matched != Match::Closest {
        return Some(doc);
    }
    let shown = l.entry.map(|t| t.title()).unwrap_or_default();
    let mut lines = paragraph(
        &[Span::plain(format!(
            "No exact match for \"{}\", showing \"{shown}\".",
            l.query
        ))],
        layout.width,
    );
    lines.extend(did_you_mean("Did you mean:", &l.suggestions, layout.width));
    let mut header: Vec<Line> = lines.into_iter().map(Line::dimmed).collect();
    header.push(Line::default());
    header.append(&mut doc.lines);
    doc.lines = header;
    Some(doc)
}

pub fn lookup(lib: &Library, l: &Lookup<'_>, opts: Options) -> Option<String> {
    lookup_doc(lib, l, &Layout::cli(opts)).map(|d| d.to_text(opts.color))
}

/// `Did you mean: atfq 5.1 (free vs available), atfq disk (Disk)`
fn did_you_mean(intro: &str, refs: &[Ref], width: usize) -> Vec<Line> {
    if refs.is_empty() {
        return Vec::new();
    }
    let mut spans = vec![Span::plain(format!("{intro} "))];
    for (i, r) in refs.iter().enumerate() {
        let sep = if i + 1 < refs.len() { ")," } else { ")" };
        spans.push(open(&r.handle));
        spans.push(Span::plain(format!(" ({}{sep} ", r.title)));
    }
    paragraph(&spans, width)
}

pub fn not_found_doc(l: &Lookup<'_>, width: usize) -> Doc {
    let mut lines = vec![Line::plain(format!("Nothing matches \"{}\".", l.query))];
    lines.extend(did_you_mean("Related:", &l.suggestions, width));
    lines.push(Line::new(vec![
        Span::plain("Search every hub and card: "),
        Span::new(command(&format!("search {}", l.query)), Kind::Code),
    ]));
    Doc { lines }
}

pub fn not_found(l: &Lookup<'_>, opts: Options) -> String {
    not_found_doc(l, WIDTH).to_text(opts.color)
}

pub fn search_doc(query: &str, hits: &[Hit], width: usize) -> Doc {
    if hits.is_empty() {
        return Doc {
            lines: vec![
                Line::plain(format!("Nothing found for \"{query}\".")),
                Line::new(vec![
                    Span::plain("Browse everything with: "),
                    Span::new(command("toc"), Kind::Code),
                ]),
            ],
        };
    }
    let left = hits
        .iter()
        .map(|h| text::width(&command(&h.handle)))
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for h in hits {
        let action = Action::Open(h.handle.clone());
        let cmd = command(&h.handle);
        lines.push(Line::new(vec![
            Span::plain(" ".repeat(INDENT)),
            open(&h.handle),
            Span::plain(" ".repeat(left - text::width(&cmd) + 2)),
            Span::new(&h.title, Kind::Bold).with(action),
        ]));
        if let Some(snippet) = &h.snippet {
            let p = Line::spaces(INDENT + left + 2);
            let spans = [Span::plain(snippet.as_str())];
            lines.extend(wrap(&spans, &p, &p, width).into_iter().map(Line::dimmed));
        }
    }
    let mut page = Page::default();
    page.section(&format!("Results for \"{query}\""), lines);
    page.finish()
}

pub fn search(query: &str, hits: &[Hit], opts: Options) -> String {
    search_doc(query, hits, WIDTH).to_text(opts.color)
}

/// The probe results alone, as in "On your machine". Used by the AI prompt.
pub fn live_text(live: &[ProbeResult], width: usize) -> String {
    Doc {
        lines: live::block(live, width),
    }
    .to_text(false)
}

/// `atfq probe <name>`: one probe result.
pub fn probe(info: &ProbeInfo, r: &ProbeResult, opts: Options) -> String {
    let mut page = Page::default();
    page.block(vec![Line::new(vec![
        Span::new(info.name, Kind::Bold),
        Span::plain("  "),
        Span::new(info.description, Kind::Dim),
    ])]);
    page.block(live::result(r, WIDTH));
    page.finish().to_text(opts.color)
}

/// `atfq probe` alone: the list of probes.
pub fn probe_list(probes: &[ProbeInfo], opts: Options) -> String {
    let rows: Vec<_> = probes
        .iter()
        .map(|p| Row::new(command(&format!("probe {}", p.name)), p.description))
        .collect();
    let mut page = Page::default();
    page.section("Probes", table(&rows, WIDTH));
    page.finish().to_text(opts.color)
}
