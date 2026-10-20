//! The content as a static website, laid out like an mdBook (the Rust book):
//! contents on the left, one page per hub, chapter and card, previous and
//! next arrows, search. Colours from Tokyo Night. The devutils site
//! (devutils-org/devutils, `npm run book:atfq`) publishes it in its own style.
//!
//! ```text
//! cargo run --example book            # writes target/book/
//! cargo run --example book -- out/    # somewhere else
//! ```
//!
//! A website can't see the reader's machine: "On your machine" shows what
//! the probes say on the `laptop` test fixture, marked as an example.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use atfq::content::danger::is_dangerous;
use atfq::content::{Card, Chapter, ChapterKind, Hub, Library};
use atfq::probes::{self, ProbeCtx};
use atfq::render::{self, roman, DeepDive, Doc, Kind, Layout, Line};
use atfq::BIN_NAME;

#[path = "../tests/common/mod.rs"]
mod common;

const CSS: &str = include_str!("book-assets/book.css");
const JS: &str = include_str!("book-assets/book.js");

/// One page of the book, in reading order.
struct Page {
    file: String,
    /// Shown in the sidebar and the window title.
    title: String,
    /// `17.1`, `17.`, or empty for the introduction.
    number: String,
    body: String,
}

fn main() {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/book".into()),
    );
    let lib = match load() {
        Ok(lib) => lib,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let refs = lib
        .cards()
        .map(|c| (c.id.clone(), (card_file(c), c.number.to_string())))
        .chain(lib.hubs().map(|h| {
            (
                h.id.clone(),
                (chapter_file(h.chapter), format!("{BIN_NAME} {}", h.id)),
            )
        }))
        .collect();
    REFS.with(|r| *r.borrow_mut() = refs);
    let ctx = common::laptop();

    let mut pages = vec![Page {
        file: "index.html".into(),
        title: "Introduction".into(),
        number: String::new(),
        body: intro(&lib),
    }];
    for ch in lib.chapters() {
        let body = match &ch.kind {
            ChapterKind::Hub(id) => hub_page(&lib, &ctx, ch, lib.hub(id).expect("validated")),
            ChapterKind::Plain => chapter_page(&lib, ch),
        };
        pages.push(Page {
            file: chapter_file(ch.number),
            title: ch.title.clone(),
            number: format!("{}.", ch.number),
            body,
        });
        for card in ch.cards.iter().filter_map(|id| lib.card(id)) {
            pages.push(Page {
                file: card_file(card),
                title: card.title.clone(),
                number: card.number.to_string(),
                body: card_page(&lib, &ctx, card),
            });
        }
    }

    fs::create_dir_all(&out).expect("create the output folder");
    for (i, page) in pages.iter().enumerate() {
        let prev = i.checked_sub(1).map(|j| &pages[j]);
        let next = pages.get(i + 1);
        let html = layout(&lib, page, prev, next);
        fs::write(out.join(&page.file), html).expect("write a page");
    }
    fs::write(out.join("book.css"), CSS).expect("write book.css");
    fs::write(out.join("book.js"), JS).expect("write book.js");
    fs::write(out.join("search-index.js"), search_index(&lib)).expect("write the index");
    println!("{} pages in {}", pages.len(), out.display());
}

// ----- references -----

/// `{{id}}` references stay in the text as `⟦id⟧`, so pages can turn them
/// into links; the library would replace them with bare numbers.
const OPEN: char = '⟦';
const CLOSE: char = '⟧';

thread_local! {
    /// Card or hub id → (page, text the terminal shows for the reference).
    static REFS: RefCell<HashMap<String, (String, String)>> = RefCell::default();
}

/// The content from `content/`, references kept.
fn load() -> Result<Library, atfq::content::LoadError> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("content");
    let mut files = BTreeMap::new();
    let mut dirs = vec![root.clone()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).expect("read content/") {
            let path = entry.expect("read content/").path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let rel = path.strip_prefix(&root).expect("under content/");
            let src = fs::read_to_string(&path).expect("read a content file");
            files.insert(rel.to_string_lossy().into_owned(), keep_refs(&src));
        }
    }
    Library::from_files(&files)
}

fn keep_refs(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find("{{") {
        let Some(end) = rest[start..].find("}}") else {
            break;
        };
        out.push_str(&rest[..start]);
        out.push(OPEN);
        out.push_str(rest[start + 2..start + end].trim());
        out.push(CLOSE);
        rest = &rest[start + end + 2..];
    }
    out.push_str(rest);
    out
}

/// Text with its references, each through `f(page, label)`, the rest through `text`.
fn expand(src: &str, text: impl Fn(&str) -> String, f: impl Fn(&str, &str) -> String) -> String {
    let mut out = String::new();
    let mut rest = src;
    while let Some(start) = rest.find(OPEN) {
        let after = &rest[start + OPEN.len_utf8()..];
        let Some(end) = after.find(CLOSE) else {
            break;
        };
        out.push_str(&text(&rest[..start]));
        let id = &after[..end];
        let target = REFS.with(|r| r.borrow().get(id).cloned());
        match target {
            Some((page, label)) => out.push_str(&f(&page, &label)),
            None => out.push_str(&text(id)),
        }
        rest = &after[end + CLOSE.len_utf8()..];
    }
    out.push_str(&text(rest));
    out
}

/// References as links, the rest escaped.
fn linked(src: &str) -> String {
    expand(src, esc, |page, label| {
        format!(r#"<a class="ref" href="{page}">{}</a>"#, esc(label))
    })
}

/// References as the terminal shows them.
fn unref(src: &str) -> String {
    expand(src, str::to_string, |_, label| label.to_string())
}

fn chapter_file(n: u32) -> String {
    format!("{n}.html")
}

fn card_file(card: &Card) -> String {
    format!("{}.html", card.number)
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

// ----- the frame around every page -----

fn layout(lib: &Library, page: &Page, prev: Option<&Page>, next: Option<&Page>) -> String {
    let title = if page.number.is_empty() {
        format!("The {BIN_NAME} book")
    } else {
        format!("{} {} · {BIN_NAME}", page.number, page.title)
    };
    let nav_link = |p: Option<&Page>, rel: &str, arrow: &str| match p {
        Some(p) => format!(
            r#"<a class="nav-{rel}" rel="{rel}" href="{}" title="{} {}"><span aria-hidden="true">{arrow}</span><span class="sr">{rel}: {}</span></a>"#,
            p.file,
            esc(&p.number),
            esc(&p.title),
            esc(&p.title)
        ),
        None => format!(r#"<span class="nav-{rel}"></span>"#),
    };
    let pager = |p: Option<&Page>, rel: &str, label: &str| match p {
        Some(p) => format!(
            r#"<a class="pager-{rel}" rel="{rel}" href="{}"><small>{label}</small><span>{} {}</span></a>"#,
            p.file,
            esc(&p.number),
            esc(&p.title)
        ),
        None => "<span></span>".into(),
    };
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>{title}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:ital,wght@0,400;0,500;0,600;1,400&family=JetBrains+Mono:wght@400;600&family=Martian+Mono:wdth,wght@87.5,500;112.5,700;112.5,800&display=swap">
<link rel="stylesheet" href="book.css">
<script>try{{var t=localStorage.getItem("atfq-theme");if(t)document.documentElement.dataset.theme=t;if(localStorage.getItem("atfq-sidebar")==="hidden"&&innerWidth>900)document.documentElement.classList.add("sidebar-hidden")}}catch(e){{}}</script>
</head>
<body>
<a class="skip" href="#content">Skip to the page</a>
<nav class="sidebar" id="sidebar" aria-label="Contents">
{sidebar}
</nav>
<div class="page">
<header class="bar">
  <button class="icon" id="toggle-sidebar" type="button" aria-controls="sidebar" aria-expanded="true" title="Contents (t)">☰</button>
  <button class="icon" id="toggle-theme" type="button" title="Theme">◐</button>
  <button class="icon" id="toggle-search" type="button" aria-controls="search" title="Search (/ or s)">⌕</button>
  <a class="book-title" href="index.html">{BIN_NAME}<span>_</span> book</a>
  <a class="icon gh" href="../index.html" title="atfq home">home</a>
  <a class="icon" href="https://github.com/devutils-org/atfq" title="Source">git</a>
</header>
<div class="search" id="search" hidden>
  <input id="search-input" type="search" placeholder="Search {count} cards: a word, an alias, an error message…" autocomplete="off" spellcheck="false" aria-label="Search the book">
  <ol id="search-results" class="results"></ol>
</div>
<main id="content" class="content">
{body}
<nav class="pager" aria-label="Pages">{pager_prev}{pager_next}</nav>
</main>
{nav_prev}{nav_next}
</div>
<script src="search-index.js"></script>
<script src="book.js"></script>
</body>
</html>
"##,
        title = esc(&title),
        sidebar = sidebar(lib, &page.file),
        count = lib.cards().count(),
        body = page.body,
        pager_prev = pager(prev, "prev", "Previous"),
        pager_next = pager(next, "next", "Next"),
        nav_prev = nav_link(prev, "prev", "‹"),
        nav_next = nav_link(next, "next", "›"),
    )
}

fn sidebar(lib: &Library, current: &str) -> String {
    let active = |file: &str| {
        if file == current {
            r#" class="active" aria-current="page""#
        } else {
            ""
        }
    };
    let mut s = String::from("<ol class=\"toc\">\n");
    let _ = writeln!(
        s,
        r#"<li class="ch"><a href="index.html"{}>Introduction</a></li>"#,
        active("index.html")
    );
    let mut part: Option<&str> = None;
    let mut part_n = 0;
    for ch in lib.chapters() {
        if ch.part.as_deref() != part {
            part = ch.part.as_deref();
            if let Some(p) = part {
                part_n += 1;
                let _ = writeln!(s, r#"<li class="part">{}. {}</li>"#, roman(part_n), esc(p));
            }
        }
        let file = chapter_file(ch.number);
        let open = current == file
            || ch
                .cards
                .iter()
                .filter_map(|id| lib.card(id))
                .any(|c| card_file(c) == current);
        let _ = write!(
            s,
            r#"<li class="ch{}"><a href="{file}"{}><b>{}.</b> {}</a>"#,
            if open { " open" } else { "" },
            active(&file),
            ch.number,
            esc(&ch.title)
        );
        if !ch.cards.is_empty() {
            let _ = write!(
                s,
                r#"<button class="fold" type="button" aria-label="Show the cards of chapter {}" aria-expanded="{open}"></button><ol>"#,
                ch.number
            );
            for card in ch.cards.iter().filter_map(|id| lib.card(id)) {
                let f = card_file(card);
                let _ = write!(
                    s,
                    r#"<li><a href="{f}"{}><b>{}</b> {}</a></li>"#,
                    active(&f),
                    card.number,
                    esc(&card.title)
                );
            }
            s.push_str("</ol>");
        }
        s.push_str("</li>\n");
    }
    s.push_str("</ol>");
    s
}

// ----- pages -----

fn intro(lib: &Library) -> String {
    let cards = lib.cards().count();
    let hubs = lib.hubs().count();
    let chapters = lib.chapters().len();
    let mut parts = String::new();
    let mut seen: Vec<&str> = Vec::new();
    for ch in lib.chapters() {
        let Some(p) = ch.part.as_deref() else {
            continue;
        };
        if seen.contains(&p) {
            continue;
        }
        seen.push(p);
        let in_part: Vec<&Chapter> = lib
            .chapters()
            .iter()
            .filter(|c| c.part.as_deref() == Some(p))
            .collect();
        let n: usize = in_part.iter().map(|c| c.cards.len()).sum();
        let first = in_part[0];
        let _ = write!(
            parts,
            r#"<li><a href="{}"><span class="roman">{}</span><b>{}</b><small>chapters {}–{} · {n} cards</small></a></li>"#,
            chapter_file(first.number),
            roman(seen.len() as u32),
            esc(p),
            first.number,
            in_part[in_part.len() - 1].number
        );
    }
    format!(
        r#"<p class="eyebrow">A Linux companion for the terminal</p>
<h1 class="hero">Ask the question<span class="q">.</span> Your machine answers.</h1>
<p class="lede"><strong>Short answers first</strong>, your machine's real state second, the deep "why" only if you want it. This is the book that ships inside <code>{BIN_NAME}</code>: every page here is one command away in your terminal.</p>
<div class="install"><pre id="install-cmd"><span class="p">$ </span>cargo install --git https://github.com/devutils-org/atfq</pre><button class="copy" type="button" data-copy="install-cmd">COPY</button></div>
<ul class="stats" aria-label="What's inside"><li><b>{chapters}</b>chapters</li><li><b>{cards}</b>cards</li><li><b>{hubs}</b>hubs</li><li><b>{probes}</b>live probes</li></ul>

<h2>Three parts</h2>
<ol class="parts">{parts}</ol>

<h2>How to read a card</h2>
<p>Every card answers one question, always in the same order. You can stop reading at any point: the first screen solves the problem.</p>
<ol class="layers">
<li><b>TL;DR</b><span>Three lines in plain English, and the command to type.</span></li>
<li><b>On your machine</b><span>What <code>/proc</code>, <code>/sys</code> and <code>/etc</code> say right now. This site shows an example machine; your terminal shows yours.</span></li>
<li><b>Pitfalls</b><span>The mistakes everyone makes once.</span></li>
<li><b>Deep dive</b><span>The "why", folded away until you open a question.</span></li>
</ol>

<h2>Finding your way</h2>
<p>Hubs (Memory, Disk, Network…) are where you start when you know the device but not the question: they list the commands, the files and every card about it. Chapter numbers are the ones <code>{BIN_NAME}</code> understands, so <code>{BIN_NAME} 17.1</code> opens the same page in your terminal.</p>
<p>On this site, <kbd>←</kbd> <kbd>→</kbd> turn the pages, <kbd>/</kbd> searches, <kbd>t</kbd> hides the contents.</p>
"#,
        probes = probes::REGISTRY.len(),
    )
}

fn chapter_page(lib: &Library, ch: &Chapter) -> String {
    let mut s = heading(
        &format!("{}.", ch.number),
        &ch.title,
        ch.part.as_deref(),
        lib,
    );
    s.push_str(r#"<ol class="card-list">"#);
    for card in ch.cards.iter().filter_map(|id| lib.card(id)) {
        let _ = write!(
            s,
            r#"<li><a href="{}"><b>{}</b><span>{}</span><small>{}</small></a></li>"#,
            card_file(card),
            card.number,
            esc(&card.title),
            esc(&summary_line(&card.tldr))
        );
    }
    s.push_str("</ol>");
    s
}

fn hub_page(lib: &Library, ctx: &ProbeCtx, ch: &Chapter, hub: &Hub) -> String {
    let mut s = heading(
        &format!("{}.", ch.number),
        &hub.title,
        ch.part.as_deref(),
        lib,
    );
    s.push_str(r#"<p class="kind">hub</p>"#);
    s.push_str(&markdown(&hub.intro));
    let live = probes::run_all(&hub.probes, ctx);
    let doc = render::hub_doc(lib, hub, None, &live, &LIVE_LAYOUT);
    s.push_str(&machine(&hub.id, &hub.probes, &doc));

    if !hub.commands.is_empty() {
        s.push_str("<h2>Commands</h2><dl class=\"rows\">");
        for c in &hub.commands {
            let desc = match c.card.as_deref().and_then(|id| lib.card(id)) {
                Some(card) => format!(
                    r#"{} <a href="{}">{}</a>"#,
                    esc(&c.desc),
                    card_file(card),
                    card.number
                ),
                None => esc(&c.desc),
            };
            let _ = write!(s, "<div><dt>{}</dt><dd>{desc}</dd></div>", code(&c.cmd));
        }
        s.push_str("</dl>");
    }
    if !hub.paths.is_empty() {
        s.push_str("<h2>Paths</h2><dl class=\"rows\">");
        for p in &hub.paths {
            let _ = write!(
                s,
                "<div><dt><code>{}</code></dt><dd>{}</dd></div>",
                esc(&p.path),
                esc(&p.desc)
            );
        }
        s.push_str("</dl>");
    }
    let own: Vec<&Card> = ch.cards.iter().filter_map(|id| lib.card(id)).collect();
    let related: Vec<&Card> = hub
        .cards
        .iter()
        .filter(|id| !ch.cards.contains(id))
        .filter_map(|id| lib.card(id))
        .collect();
    for (heading, cards) in [("Cards", own), ("Also about this", related)] {
        if cards.is_empty() {
            continue;
        }
        let _ = write!(s, "<h2>{heading}</h2><ol class=\"card-list\">");
        for card in cards {
            let _ = write!(
                s,
                r#"<li><a href="{}"><b>{}</b><span>{}</span><small>{}</small></a></li>"#,
                card_file(card),
                card.number,
                esc(&card.title),
                esc(&summary_line(&card.tldr))
            );
        }
        s.push_str("</ol>");
    }
    s
}

fn card_page(lib: &Library, ctx: &ProbeCtx, card: &Card) -> String {
    let ch = lib.chapter(card.number.chapter).expect("validated");
    let mut s = heading(
        &card.number.to_string(),
        &card.title,
        ch.part.as_deref(),
        lib,
    );
    if !card.aliases.is_empty() {
        s.push_str(r#"<p class="aliases"><span>also</span>"#);
        for a in &card.aliases {
            let _ = write!(s, "<code>{}</code>", esc(a));
        }
        s.push_str("</p>");
    }
    s.push_str("<section class=\"tldr\"><h2>TL;DR</h2>");
    s.push_str(&markdown(&card.tldr));
    s.push_str("</section>");
    let live = probes::run_all(&card.probes, ctx);
    let doc = render::card_doc(lib, card, &live, &LIVE_LAYOUT);
    s.push_str(&machine(&card.number.to_string(), &card.probes, &doc));
    if let Some(p) = &card.pitfalls {
        s.push_str("<h2>Pitfalls</h2>");
        s.push_str(&markdown(p));
    }
    if !card.deep_dive.is_empty() {
        let _ = write!(
            s,
            r#"<h2>Deep dive <small>{} questions</small></h2><div class="dive">"#,
            card.deep_dive.len()
        );
        for q in &card.deep_dive {
            let _ = write!(
                s,
                "<details><summary>{}</summary><div>{}</div></details>",
                inline(&q.title),
                markdown(&q.body)
            );
        }
        s.push_str("</div>");
    }

    let mut meta = String::new();
    if !card.commands.is_empty() {
        let list: Vec<String> = card
            .commands
            .iter()
            .map(|c| {
                let page = match (lib.hub_by_key(c), lib.card_by_key(c)) {
                    (Some(h), _) => Some(chapter_file(h.chapter)),
                    (None, Some(other)) if other.id != card.id => Some(card_file(other)),
                    _ => None,
                };
                match page {
                    Some(p) => format!(r#"<a href="{p}"><code>{}</code></a>"#, esc(c)),
                    None => format!("<code>{}</code>", esc(c)),
                }
            })
            .collect();
        let _ = write!(
            meta,
            "<div><dt>Commands</dt><dd>{}</dd></div>",
            list.join(" ")
        );
    }
    if !card.paths.is_empty() {
        let list: Vec<String> = card
            .paths
            .iter()
            .map(|p| format!("<code>{}</code>", esc(p)))
            .collect();
        let _ = write!(meta, "<div><dt>Paths</dt><dd>{}</dd></div>", list.join(" "));
    }
    let mut see: Vec<String> = Vec::new();
    for id in card.hubs.iter().chain(&card.see_also) {
        if let Some(h) = lib.hub(id) {
            see.push(format!(
                r#"<a href="{}">{}. {}</a>"#,
                chapter_file(h.chapter),
                h.chapter,
                esc(&h.title)
            ));
        } else if let Some(c) = lib.card(id) {
            see.push(format!(
                r#"<a href="{}">{} {}</a>"#,
                card_file(c),
                c.number,
                esc(&c.title)
            ));
        }
    }
    if !see.is_empty() {
        let _ = write!(
            meta,
            "<div><dt>See also</dt><dd>{}</dd></div>",
            see.join("<br>")
        );
    }
    if !meta.is_empty() {
        let _ = write!(s, r#"<dl class="meta">{meta}</dl>"#);
    }
    s
}

fn heading(number: &str, title: &str, part: Option<&str>, lib: &Library) -> String {
    let crumb = part
        .map(|p| {
            let n = lib
                .chapters()
                .iter()
                .filter_map(|c| c.part.as_deref())
                .fold(Vec::<&str>::new(), |mut v, x| {
                    if !v.contains(&x) {
                        v.push(x);
                    }
                    v
                })
                .iter()
                .position(|x| *x == p)
                .map_or(0, |i| i + 1);
            format!(
                r#"<p class="eyebrow">Part {} · {}</p>"#,
                roman(n as u32),
                esc(p)
            )
        })
        .unwrap_or_default();
    format!(
        r#"{crumb}<h1><span class="num">{}</span> {}</h1>"#,
        esc(number),
        esc(title)
    )
}

/// Wide enough for the probe tables, narrow enough for the page column.
const LIVE_LAYOUT: Layout = Layout {
    width: 76,
    deep_dive: DeepDive::Hint,
};

/// "On your machine" as the terminal prints it, from a page rendered
/// against the fixture: the lines under that heading, up to the next one.
fn machine(handle: &str, probe_names: &[String], doc: &Doc) -> String {
    if probe_names.is_empty() {
        return String::new();
    }
    let mut lines: Vec<&Line> = doc
        .lines
        .iter()
        .skip_while(|l| l.text() != "On your machine")
        .skip(1)
        .take_while(|l| {
            let t = l.text();
            t.is_empty() || t.starts_with(' ')
        })
        .collect();
    while lines.last().is_some_and(|l| l.text().trim().is_empty()) {
        lines.pop();
    }
    let mut out = String::new();
    for line in lines {
        out.push('\n');
        for span in &line.spans {
            let class = match span.kind {
                Kind::Plain => None,
                Kind::Bold => Some("b"),
                Kind::Emph => Some("e"),
                Kind::Code => Some("c"),
                Kind::Dim => Some("d"),
                Kind::Warn => Some("y"),
            };
            match class {
                Some(c) => {
                    let _ = write!(out, r#"<span class="{c}">{}</span>"#, esc(&span.text));
                }
                None => out.push_str(&esc(&span.text)),
            }
        }
    }
    format!(
        r#"<section class="machine"><h2>On your machine</h2><div class="term"><div class="term-bar"><span>alice@laptop</span><span>example</span></div><pre><span class="c">$</span> {BIN_NAME} {}{out}</pre></div><p class="note">An example: Alice's laptop, one of the test machines. In your terminal, this block shows your own machine, read live.</p></section>"#,
        esc(handle)
    )
}

// ----- Markdown -----

fn code(s: &str) -> String {
    if is_dangerous(s) {
        format!(
            r#"<code class="danger" title="Changes or deletes data">⚠ {}</code>"#,
            esc(s)
        )
    } else {
        format!("<code>{}</code>", esc(s))
    }
}

/// Card Markdown → HTML: what cards use (see `src/render/markdown.rs`), with
/// commands that change or delete data marked as the terminal marks them.
fn markdown(src: &str) -> String {
    let mut out = String::new();
    let mut block: Option<String> = None;
    for event in Parser::new(src) {
        if let Some(b) = block.as_mut() {
            match event {
                Event::Text(t) => b.push_str(&unref(&t)),
                Event::End(TagEnd::CodeBlock) => {
                    let text = block.take().expect("open block");
                    out.push_str("<pre class=\"code\"><code>");
                    for line in text.trim_end_matches('\n').lines() {
                        if is_dangerous(line) {
                            let _ = writeln!(
                                out,
                                r#"<span class="danger" title="Changes or deletes data">{}</span>"#,
                                esc(line)
                            );
                        } else {
                            let _ = writeln!(out, "{}", esc(line));
                        }
                    }
                    out.push_str("</code></pre>\n");
                }
                _ => {}
            }
            continue;
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => out.push_str("<p>"),
                Tag::List(Some(n)) if n != 1 => {
                    let _ = write!(out, r#"<ol start="{n}">"#);
                }
                Tag::List(Some(_)) => out.push_str("<ol>"),
                Tag::List(None) => out.push_str("<ul>"),
                Tag::Item => out.push_str("<li>"),
                Tag::Strong => out.push_str("<strong>"),
                Tag::Emphasis => out.push_str("<em>"),
                Tag::CodeBlock(_) => block = Some(String::new()),
                Tag::Link { dest_url, .. } => {
                    let _ = write!(out, r#"<a href="{}">"#, esc(&dest_url));
                }
                Tag::BlockQuote(_) => out.push_str("<blockquote>"),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => out.push_str("</p>\n"),
                TagEnd::List(true) => out.push_str("</ol>\n"),
                TagEnd::List(false) => out.push_str("</ul>\n"),
                TagEnd::Item => out.push_str("</li>\n"),
                TagEnd::Strong => out.push_str("</strong>"),
                TagEnd::Emphasis => out.push_str("</em>"),
                TagEnd::Link => out.push_str("</a>"),
                TagEnd::BlockQuote(_) => out.push_str("</blockquote>"),
                _ => {}
            },
            Event::Text(t) => out.push_str(&linked(&t)),
            Event::Code(t) => out.push_str(&code(&unref(&t))),
            Event::SoftBreak => out.push('\n'),
            Event::HardBreak => out.push_str("<br>"),
            _ => {}
        }
    }
    out
}

/// One line of Markdown, without the paragraph around it.
fn inline(src: &str) -> String {
    let html = markdown(src);
    html.trim()
        .trim_start_matches("<p>")
        .trim_end_matches("</p>")
        .to_string()
}

/// Plain text of some Markdown.
fn plain(src: &str) -> String {
    let mut s = String::new();
    for event in Parser::new(src) {
        match event {
            Event::Text(t) | Event::Code(t) => s.push_str(&t),
            Event::SoftBreak | Event::HardBreak | Event::End(_) => s.push(' '),
            _ => {}
        }
    }
    unref(&s.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// The first point of a TL;DR, for lists of cards.
fn summary_line(tldr: &str) -> String {
    let mut first = String::new();
    let mut depth = 0;
    for event in Parser::new(tldr) {
        match event {
            Event::Start(Tag::Item) => depth += 1,
            Event::End(TagEnd::Item) => {
                if depth == 1 && !first.is_empty() {
                    break;
                }
                depth -= 1;
            }
            Event::Text(t) | Event::Code(t) if depth >= 1 => first.push_str(&t),
            Event::SoftBreak if depth >= 1 => first.push(' '),
            Event::End(TagEnd::Paragraph) if depth == 0 && !first.is_empty() => break,
            Event::Text(t) | Event::Code(t) => first.push_str(&t),
            _ => {}
        }
    }
    unref(&first)
}

// ----- search -----

fn json(s: &str) -> String {
    serde_json::to_string(s).expect("a string serializes")
}

fn search_index(lib: &Library) -> String {
    let mut s = String::from("window.ATFQ_INDEX=[\n");
    for ch in lib.chapters() {
        if let ChapterKind::Hub(id) = &ch.kind {
            let hub = lib.hub(id).expect("validated");
            let _ = writeln!(
                s,
                "[{},{},{},{},{}],",
                json(&chapter_file(ch.number)),
                json(&format!("{}.", ch.number)),
                json(&hub.title),
                json(&format!("{} {}", hub.id, hub.aliases.join(" "))),
                json(&plain(&hub.intro))
            );
        }
        for card in ch.cards.iter().filter_map(|id| lib.card(id)) {
            let mut text = plain(&card.tldr);
            if let Some(p) = &card.pitfalls {
                text.push(' ');
                text.push_str(&plain(p));
            }
            for q in &card.deep_dive {
                text.push(' ');
                text.push_str(&plain(&q.title));
            }
            let _ = writeln!(
                s,
                "[{},{},{},{},{}],",
                json(&card_file(card)),
                json(&card.number.to_string()),
                json(&card.title),
                json(&format!("{} {}", card.id, card.aliases.join(" | "))),
                json(&text)
            );
        }
    }
    s.push_str("];\n");
    s
}
