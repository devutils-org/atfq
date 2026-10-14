//! Plain text rendering for the CLI.

mod live;
mod markdown;
mod text;

pub use text::Style;

use text::{wrap, Kind, Prefix, Span};

use crate::content::danger::is_dangerous;
use crate::content::{Card, ChapterKind, Hub, Library};
use crate::probes::{ProbeInfo, ProbeResult};
use crate::resolve::Target;
use crate::search::Hit;
use crate::view::{Lookup, Match, Ref};
use crate::BIN_NAME;

/// Output never goes wider than this.
pub const WIDTH: usize = 80;

/// Body text sits this far right of section headings.
const INDENT: usize = 2;

/// Widest left column in the Commands and Paths tables before the
/// description moves to its own line.
const MAX_LEFT: usize = 30;

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Expand the deep dive.
    pub why: bool,
    pub color: bool,
}

pub(crate) fn warning(st: Style) -> String {
    st.warn("⚠ changes or deletes data: check what it targets first")
}

/// Lines grouped in blocks separated by one blank line.
#[derive(Default)]
struct Page {
    lines: Vec<String>,
}

impl Page {
    fn block(&mut self, lines: Vec<String>) {
        if lines.is_empty() {
            return;
        }
        if !self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.lines.extend(lines);
    }

    fn section(&mut self, st: Style, heading: &str, body: Vec<String>) {
        if body.is_empty() {
            return;
        }
        let mut lines = vec![st.bold(heading)];
        lines.extend(body);
        self.block(lines);
    }

    fn finish(self) -> String {
        let mut s = self.lines.join("\n");
        s.push('\n');
        s
    }
}

fn md(src: &str, indent: usize, st: Style) -> Vec<String> {
    markdown::render(src, indent, WIDTH, st)
}

fn command(s: &str) -> String {
    format!("{BIN_NAME} {s}")
}

pub fn card(lib: &Library, card: &Card, live: &[ProbeResult], opts: Options) -> String {
    let st = Style::new(opts.color);
    let mut page = Page::default();
    page.block(vec![st.bold(&format!("{}  {}", card.number, card.title))]);
    page.section(st, "TL;DR", md(&card.tldr, INDENT, st));
    page.section(st, "On your machine", live::block(live, st));
    if let Some(pitfalls) = &card.pitfalls {
        page.section(st, "Pitfalls", md(pitfalls, INDENT, st));
    }

    let n = card.deep_dive.len();
    if n > 0 && opts.why {
        let mut lines = vec![st.bold("Deep dive")];
        for q in &card.deep_dive {
            lines.push(String::new());
            lines.push(format!("{}▾ {}", " ".repeat(INDENT), st.bold(&q.title)));
            lines.push(String::new());
            lines.extend(md(&q.body, INDENT * 2, st));
        }
        page.block(lines);
    } else if n > 0 {
        let what = if n == 1 { "question" } else { "questions" };
        let spans = [
            Span::new(
                format!("Deep dive: {n} {what}. Read them with "),
                Kind::Plain,
            ),
            Span::new(command(&format!("{} --why", card.number)), Kind::Code),
        ];
        page.block(wrap(
            &spans,
            &Prefix::spaces(0),
            &Prefix::spaces(0),
            WIDTH,
            st,
        ));
    }

    let mut related = Vec::new();
    let mut row = |label: &str, spans: Vec<Span>| {
        let first = Prefix::plain(&format!("{}{label:<10}", " ".repeat(INDENT)));
        related.extend(wrap(
            &spans,
            &first,
            &Prefix::spaces(INDENT + 10),
            WIDTH,
            st,
        ));
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
    page.section(st, "Related", related);
    page.finish()
}

/// `a, b, c` with each item styled as code.
fn list(items: &[String]) -> Vec<Span> {
    let mut spans = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let sep = if i + 1 < items.len() { "," } else { "" };
        spans.push(Span::new(item.as_str(), Kind::Code));
        spans.push(Span::new(format!("{sep} "), Kind::Plain));
    }
    spans
}

/// `idk <target>  Title`
fn link(target: &str, title: &str) -> Vec<Span> {
    vec![
        Span::new(command(target), Kind::Code),
        Span::new(format!("  {title}"), Kind::Plain),
    ]
}

/// Two columns: something to copy on the left, what it is on the right.
fn table(rows: &[(String, String)], st: Style) -> Vec<String> {
    let left = rows
        .iter()
        .map(|(l, _)| text::width(l))
        .filter(|&w| w <= MAX_LEFT)
        .max()
        .unwrap_or(0);
    let desc_col = INDENT + left + 2;
    let rest = Prefix::spaces(desc_col);
    let mut lines = Vec::new();
    for (l, desc) in rows {
        let w = text::width(l);
        let styled = format!("{}{}", " ".repeat(INDENT), st.code(l));
        let spans = [Span::new(desc.as_str(), Kind::Plain)];
        if w <= MAX_LEFT {
            let first = Prefix {
                text: format!("{styled}{}", " ".repeat(left - w + 2)),
                width: desc_col,
            };
            lines.extend(wrap(&spans, &first, &rest, WIDTH, st));
        } else {
            lines.push(styled);
            lines.extend(wrap(&spans, &rest, &rest, WIDTH, st));
        }
        if is_dangerous(l) {
            lines.push(format!("{}{}", " ".repeat(INDENT * 2), warning(st)));
        }
    }
    lines
}

pub fn hub(
    lib: &Library,
    hub: &Hub,
    also_card: Option<&Card>,
    live: &[ProbeResult],
    opts: Options,
) -> String {
    let st = Style::new(opts.color);
    let mut page = Page::default();
    if let Some(c) = also_card {
        let spans = [
            Span::new(format!("Looking for \"{}\"? Run", c.title), Kind::Plain),
            Span::new(command(&c.number.to_string()), Kind::Code),
        ];
        page.block(wrap(
            &spans,
            &Prefix::spaces(0),
            &Prefix::spaces(0),
            WIDTH,
            st,
        ));
    }
    page.block(vec![format!(
        "{}  {}",
        st.bold(&format!("{}. {}", hub.chapter, hub.title)),
        st.dim("hub")
    )]);
    page.block(md(&hub.intro, 0, st));
    page.section(st, "On your machine", live::block(live, st));

    let commands: Vec<_> = hub
        .commands
        .iter()
        .map(|c| (c.cmd.clone(), c.desc.clone()))
        .collect();
    page.section(st, "Commands", table(&commands, st));
    let paths: Vec<_> = hub
        .paths
        .iter()
        .map(|p| (p.path.clone(), p.desc.clone()))
        .collect();
    page.section(st, "Paths", table(&paths, st));

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
        .map(|c| (command(&c.number.to_string()), c.title.clone()))
        .collect();
    page.section(st, "Cards", table(&cards, st));
    page.finish()
}

pub fn toc(lib: &Library, opts: Options) -> String {
    let st = Style::new(opts.color);
    let mut page = Page::default();
    page.block(vec![st.bold("Contents")]);

    let hubs: Vec<_> = lib
        .hubs()
        .map(|h| (command(&h.id), h.title.clone()))
        .collect();
    page.section(st, "Hubs", table(&hubs, st));

    let digits = lib.chapters().len().to_string().len();
    let mut lines = vec![st.bold("Chapters")];
    for ch in lib.chapters() {
        let mut line = format!("{}{:>digits$}. {}", " ".repeat(INDENT), ch.number, ch.title);
        if let ChapterKind::Hub(id) = &ch.kind {
            line.push_str(&format!("  {}", st.dim(&format!("hub: {}", command(id)))));
        }
        lines.push(line);
        for id in &ch.cards {
            if let Some(c) = lib.card(id) {
                lines.push(format!(
                    "{}{}  {}",
                    " ".repeat(INDENT + digits + 2),
                    st.code(&c.number.to_string()),
                    c.title
                ));
            }
        }
    }
    page.block(lines);
    page.finish()
}

/// The page for a lookup: the hub or card, preceded by a note when it is only
/// the closest match. `None` when nothing matched: see [`not_found`].
pub fn lookup(lib: &Library, l: &Lookup<'_>, opts: Options) -> Option<String> {
    let st = Style::new(opts.color);
    let also_card = l.also_card.as_ref().and_then(|r| lib.card(&r.id));
    let page = match l.entry? {
        Target::Hub(h) => hub(lib, h, also_card, &l.probes, opts),
        Target::Card(c) => card(lib, c, &l.probes, opts),
    };
    if l.matched != Match::Closest {
        return Some(page);
    }
    let shown = l.entry.map(|t| t.title()).unwrap_or_default();
    let mut lines = wrap(
        &[Span::new(
            format!("No exact match for \"{}\", showing \"{shown}\".", l.query),
            Kind::Plain,
        )],
        &Prefix::spaces(0),
        &Prefix::spaces(0),
        WIDTH,
        st,
    );
    lines.extend(did_you_mean("Did you mean:", &l.suggestions, st));
    let header: String = lines.iter().map(|l| st.dim(l) + "\n").collect();
    Some(format!("{header}\n{page}"))
}

/// `Did you mean: idk 5.1 (free vs available), idk disk (Disk)`
fn did_you_mean(intro: &str, refs: &[Ref], st: Style) -> Vec<String> {
    if refs.is_empty() {
        return Vec::new();
    }
    let mut spans = vec![Span::new(format!("{intro} "), Kind::Plain)];
    for (i, r) in refs.iter().enumerate() {
        let sep = if i + 1 < refs.len() { ")," } else { ")" };
        spans.push(Span::new(command(&r.handle), Kind::Code));
        spans.push(Span::new(format!(" ({}{sep} ", r.title), Kind::Plain));
    }
    wrap(&spans, &Prefix::spaces(0), &Prefix::spaces(0), WIDTH, st)
}

pub fn not_found(l: &Lookup<'_>, opts: Options) -> String {
    let st = Style::new(opts.color);
    let mut lines = vec![format!("Nothing matches \"{}\".", l.query)];
    lines.extend(did_you_mean("Related:", &l.suggestions, st));
    lines.push(format!(
        "Search every hub and card: {}",
        st.code(&command(&format!("search {}", l.query)))
    ));
    lines.join("\n") + "\n"
}

pub fn search(query: &str, hits: &[Hit], opts: Options) -> String {
    let st = Style::new(opts.color);
    if hits.is_empty() {
        return format!(
            "Nothing found for \"{query}\".\nBrowse everything with: {}\n",
            st.code(&command("toc"))
        );
    }
    let left = hits
        .iter()
        .map(|h| text::width(&command(&h.handle)))
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for h in hits {
        let cmd = command(&h.handle);
        lines.push(format!(
            "{}{}{}{}",
            " ".repeat(INDENT),
            st.code(&cmd),
            " ".repeat(left - text::width(&cmd) + 2),
            st.bold(&h.title)
        ));
        if let Some(snippet) = &h.snippet {
            let p = Prefix::spaces(INDENT + left + 2);
            let spans = [Span::new(snippet.as_str(), Kind::Plain)];
            lines.extend(wrap(&spans, &p, &p, WIDTH, st).iter().map(|l| st.dim(l)));
        }
    }
    let mut page = Page::default();
    page.section(st, &format!("Results for \"{query}\""), lines);
    page.finish()
}

/// `idk probe <name>`: one probe result.
pub fn probe(info: &ProbeInfo, r: &ProbeResult, opts: Options) -> String {
    let st = Style::new(opts.color);
    let mut page = Page::default();
    page.block(vec![format!(
        "{}  {}",
        st.bold(info.name),
        st.dim(info.description)
    )]);
    page.block(live::result(r, st));
    page.finish()
}

/// `idk probe` alone: the list of probes.
pub fn probe_list(probes: &[ProbeInfo], opts: Options) -> String {
    let st = Style::new(opts.color);
    let rows: Vec<_> = probes
        .iter()
        .map(|p| {
            (
                command(&format!("probe {}", p.name)),
                p.description.to_string(),
            )
        })
        .collect();
    let mut page = Page::default();
    page.section(st, "Probes", table(&rows, st));
    page.finish()
}
