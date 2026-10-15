//! Loads every content file, checks it against the rules in ROADMAP.md
//! section 6.3, and indexes it for lookups.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use include_dir::{include_dir, Dir};
use serde::Serialize;

use super::entry::{self, Entry};
use super::summary;
use super::{normalize, Card, Hub, Number};
use crate::probes;

static CONTENT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/content");

const SUMMARY: &str = "SUMMARY.md";

/// Words the CLI uses as subcommands: an alias equal to one would be
/// unreachable.
const RESERVED: &[&str] = &["toc", "search", "probe", "ask", "tui", "mcp", "help"];

/// A chapter of the table of contents.
#[derive(Debug, Clone, Serialize)]
pub struct Chapter {
    pub number: u32,
    pub title: String,
    /// The part of the book, when SUMMARY.md has parts.
    pub part: Option<String>,
    pub kind: ChapterKind,
    /// Ids of the cards in this chapter, in order.
    pub cards: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "hub", rename_all = "snake_case")]
pub enum ChapterKind {
    /// A plain chapter: a title grouping cards.
    Plain,
    /// A chapter whose page is a hub, by id.
    Hub(String),
}

/// Every problem found in the content, one line each.
#[derive(Debug)]
pub struct LoadError {
    pub problems: Vec<String>,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "invalid content ({} problems):", self.problems.len())?;
        for p in &self.problems {
            writeln!(f, "  {p}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LoadError {}

#[derive(Debug)]
pub struct Library {
    chapters: Vec<Chapter>,
    hubs: BTreeMap<String, Hub>,
    cards: BTreeMap<String, Card>,
    /// Normalized id or alias → hub id.
    hub_keys: HashMap<String, String>,
    /// Normalized id or alias → card id.
    card_keys: HashMap<String, String>,
}

impl Library {
    /// The content embedded in the binary at build time.
    pub fn embedded() -> Result<Library, LoadError> {
        let mut files = BTreeMap::new();
        let mut problems = Vec::new();
        collect(&CONTENT, &mut files, &mut problems);
        if !problems.is_empty() {
            return Err(LoadError { problems });
        }
        Library::from_files(&files)
    }

    /// Builds a library from `path → contents`, paths relative to `content/`.
    pub fn from_files(files: &BTreeMap<String, String>) -> Result<Library, LoadError> {
        let mut problems = Vec::new();

        let Some(summary_src) = files.get(SUMMARY) else {
            return Err(LoadError {
                problems: vec![format!("{SUMMARY} is missing")],
            });
        };
        let toc = summary::parse(summary_src).unwrap_or_else(|errs| {
            problems.extend(errs);
            Vec::new()
        });

        // Parse every file.
        let mut hubs = BTreeMap::new();
        let mut cards = BTreeMap::new();
        let mut id_source: HashMap<String, String> = HashMap::new();
        let mut by_path: HashMap<&str, String> = HashMap::new();
        for (path, src) in files {
            if path == SUMMARY {
                continue;
            }
            if !path.ends_with(".md") {
                problems.push(format!("{path}: only Markdown files belong in content/"));
                continue;
            }
            let parsed = match entry::parse(path, src) {
                Ok(e) => e,
                Err(errs) => {
                    problems.extend(errs);
                    continue;
                }
            };
            let id = match &parsed {
                Entry::Card(c) => c.id.clone(),
                Entry::Hub(h) => h.id.clone(),
            };
            if let Some(other) = id_source.insert(id.clone(), path.clone()) {
                problems.push(format!("{path}: id `{id}` is already used by {other}"));
                continue;
            }
            by_path.insert(path, id.clone());
            match parsed {
                Entry::Card(c) => {
                    cards.insert(id, c);
                }
                Entry::Hub(h) => {
                    hubs.insert(id, h);
                }
            }
        }

        // Match SUMMARY.md with the files, and number everything.
        let mut listed: HashMap<&str, usize> = HashMap::new();
        let mut chapters = Vec::new();
        for (ci, ch) in toc.iter().enumerate() {
            let number = ci as u32 + 1;
            let mut kind = ChapterKind::Plain;
            if let Some(file) = &ch.file {
                *listed.entry(file).or_default() += 1;
                match by_path.get(file.as_str()) {
                    Some(id) if hubs.contains_key(id) => {
                        let hub = hubs.get_mut(id).expect("checked above");
                        hub.chapter = number;
                        check_title(&mut problems, file, &ch.title, &hub.title);
                        kind = ChapterKind::Hub(id.clone());
                    }
                    Some(_) => problems.push(format!(
                        "{SUMMARY}: chapter `{}` points to {file}, which is a card: a chapter page must be a hub",
                        ch.title
                    )),
                    None => missing(&mut problems, files, file),
                }
            }
            let mut chapter_cards = Vec::new();
            for (ei, e) in ch.entries.iter().enumerate() {
                *listed.entry(&e.file).or_default() += 1;
                match by_path.get(e.file.as_str()) {
                    Some(id) if cards.contains_key(id) => {
                        let card = cards.get_mut(id).expect("checked above");
                        card.number = Number {
                            chapter: number,
                            section: ei as u32 + 1,
                        };
                        check_title(&mut problems, &e.file, &e.title, &card.title);
                        chapter_cards.push(id.clone());
                    }
                    Some(_) => problems.push(format!(
                        "{SUMMARY}: entry `{}` points to {}, which is a hub: hubs are chapters",
                        e.title, e.file
                    )),
                    None => missing(&mut problems, files, &e.file),
                }
            }
            chapters.push(Chapter {
                number,
                title: ch.title.clone(),
                part: ch.part.clone(),
                kind,
                cards: chapter_cards,
            });
        }
        for (file, n) in &listed {
            if *n > 1 {
                problems.push(format!("{SUMMARY}: {file} is listed {n} times"));
            }
        }
        for path in by_path.keys() {
            if !listed.contains_key(path) {
                problems.push(format!("{path}: not listed in {SUMMARY}"));
            }
        }

        // `{{id}}` in the text becomes the card's number, so prose references
        // follow any renumbering, and a wrong id is caught here.
        let numbers: HashMap<String, String> = cards
            .values()
            .map(|c| (c.id.clone(), c.number.to_string()))
            .collect();
        let hub_ids: Vec<String> = hubs.keys().cloned().collect();
        let mut expand = |source: &str, text: &mut String| {
            if let Some(n) = raw_reference(text) {
                problems.push(format!(
                    "{source}: \"see {n}\": write {{{{card.id}}}} instead of a number, so it follows renumbering"
                ));
            }
            match expand_refs(text, &numbers, &hub_ids) {
                Ok(t) => *text = t,
                Err(unknown) => {
                    for id in unknown {
                        problems.push(format!("{source}: {{{{{id}}}}} names no card or hub"));
                    }
                }
            }
        };
        for c in cards.values_mut() {
            let source = c.source.clone();
            expand(&source, &mut c.tldr);
            if let Some(p) = c.pitfalls.as_mut() {
                expand(&source, p);
            }
            for q in &mut c.deep_dive {
                expand(&source, &mut q.title);
                expand(&source, &mut q.body);
            }
        }
        for h in hubs.values_mut() {
            let source = h.source.clone();
            expand(&source, &mut h.intro);
        }

        // References and probes.
        for c in cards.values() {
            for h in &c.hubs {
                if !hubs.contains_key(h) {
                    problems.push(format!("{}: hubs: no hub with id `{h}`", c.source));
                }
            }
            for r in &c.see_also {
                if !hubs.contains_key(r) && !cards.contains_key(r) {
                    problems.push(format!(
                        "{}: see_also: no card or hub with id `{r}`",
                        c.source
                    ));
                }
            }
            check_probes(&mut problems, &c.source, &c.probes);
        }
        for h in hubs.values() {
            for r in &h.cards {
                if !cards.contains_key(r) {
                    problems.push(format!("{}: cards: no card with id `{r}`", h.source));
                }
            }
            for c in &h.commands {
                if let Some(r) = c.card.as_ref().filter(|r| !cards.contains_key(*r)) {
                    problems.push(format!(
                        "{}: commands: `{}` points to no card with id `{r}`",
                        h.source, c.cmd
                    ));
                }
            }
            check_probes(&mut problems, &h.source, &h.probes);
        }

        // Lookup keys. Hub/hub and card/card collisions are errors; a hub and
        // a card may share a key, the hub wins at lookup.
        let mut hub_keys = HashMap::new();
        for h in hubs.values() {
            add_keys(&mut problems, &mut hub_keys, &h.id, &h.aliases, &h.source);
        }
        let mut card_keys = HashMap::new();
        for c in cards.values() {
            add_keys(&mut problems, &mut card_keys, &c.id, &c.aliases, &c.source);
        }

        if problems.is_empty() {
            Ok(Library {
                chapters,
                hubs,
                cards,
                hub_keys,
                card_keys,
            })
        } else {
            problems.sort();
            Err(LoadError { problems })
        }
    }

    pub fn chapters(&self) -> &[Chapter] {
        &self.chapters
    }

    pub fn hub(&self, id: &str) -> Option<&Hub> {
        self.hubs.get(id)
    }

    pub fn card(&self, id: &str) -> Option<&Card> {
        self.cards.get(id)
    }

    /// Hubs in table of contents order.
    pub fn hubs(&self) -> impl Iterator<Item = &Hub> {
        self.chapters.iter().filter_map(|c| match &c.kind {
            ChapterKind::Hub(id) => self.hubs.get(id),
            ChapterKind::Plain => None,
        })
    }

    /// Cards in table of contents order.
    pub fn cards(&self) -> impl Iterator<Item = &Card> {
        self.chapters
            .iter()
            .flat_map(|c| c.cards.iter().filter_map(|id| self.cards.get(id)))
    }

    /// A hub by exact id or alias, ignoring case.
    pub fn hub_by_key(&self, key: &str) -> Option<&Hub> {
        self.hub_keys
            .get(&normalize(key))
            .and_then(|id| self.hub(id))
    }

    /// A card by exact id or alias, ignoring case.
    pub fn card_by_key(&self, key: &str) -> Option<&Card> {
        self.card_keys
            .get(&normalize(key))
            .and_then(|id| self.card(id))
    }

    pub fn chapter(&self, number: u32) -> Option<&Chapter> {
        let i = usize::try_from(number).ok()?.checked_sub(1)?;
        self.chapters.get(i)
    }

    pub fn card_by_number(&self, number: Number) -> Option<&Card> {
        let i = usize::try_from(number.section).ok()?.checked_sub(1)?;
        let id = self.chapter(number.chapter)?.cards.get(i)?;
        self.card(id)
    }
}

/// A chapter number written by hand after "see", like "see 5.2".
fn raw_reference(text: &str) -> Option<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    words.windows(2).find_map(|w| {
        let see = w[0].trim_start_matches('(').eq_ignore_ascii_case("see");
        let n = w[1].trim_end_matches([')', '.', ',', ':', ';']);
        let is_number = n.split_once('.').is_some_and(|(a, b)| {
            !a.is_empty()
                && !b.is_empty()
                && (a.chars().chain(b.chars())).all(|c| c.is_ascii_digit())
        });
        (see && is_number).then(|| n.to_string())
    })
}

/// Replaces `{{card.id}}` with the card's number and `{{hub}}` with the
/// command that opens the hub. Returns the unknown ids on failure.
fn expand_refs(
    text: &str,
    numbers: &HashMap<String, String>,
    hubs: &[String],
) -> Result<String, Vec<String>> {
    let mut out = String::with_capacity(text.len());
    let mut unknown = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let id = after[..end].trim();
        match numbers.get(id) {
            Some(n) => out.push_str(n),
            None if hubs.iter().any(|h| h == id) => {
                out.push_str(&format!("{} {id}", crate::BIN_NAME));
            }
            None => unknown.push(id.to_string()),
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    if unknown.is_empty() {
        Ok(out)
    } else {
        Err(unknown)
    }
}

fn collect(dir: &Dir<'_>, files: &mut BTreeMap<String, String>, problems: &mut Vec<String>) {
    for f in dir.files() {
        let path = f.path().to_string_lossy().replace('\\', "/");
        match f.contents_utf8() {
            Some(s) => {
                files.insert(path, s.to_string());
            }
            None => problems.push(format!("{path}: not valid UTF-8")),
        }
    }
    for d in dir.dirs() {
        collect(d, files, problems);
    }
}

fn check_title(problems: &mut Vec<String>, file: &str, in_summary: &str, in_file: &str) {
    if in_summary != in_file {
        problems.push(format!(
            "{SUMMARY}: title `{in_summary}` differs from the title `{in_file}` in {file}"
        ));
    }
}

fn missing(problems: &mut Vec<String>, files: &BTreeMap<String, String>, file: &str) {
    if !files.contains_key(file) {
        problems.push(format!("{SUMMARY}: {file} does not exist"));
    }
    // Otherwise the file failed to parse, and that problem is already reported.
}

fn check_probes(problems: &mut Vec<String>, source: &str, names: &[String]) {
    for p in names {
        if probes::find(p).is_none() {
            problems.push(format!("{source}: probes: no probe named `{p}`"));
        }
    }
}

fn add_keys(
    problems: &mut Vec<String>,
    keys: &mut HashMap<String, String>,
    id: &str,
    aliases: &[String],
    source: &str,
) {
    for key in std::iter::once(id).chain(aliases.iter().map(String::as_str)) {
        let k = normalize(key);
        if k.is_empty() {
            problems.push(format!("{source}: empty alias"));
            continue;
        }
        if RESERVED.contains(&k.as_str()) {
            problems.push(format!(
                "{source}: `{k}` is a command name, it can't be an alias"
            ));
        }
        if crate::resolve::parse_number(&k).is_some() {
            problems.push(format!("{source}: `{k}` looks like a chapter number"));
        }
        match keys.get(&k) {
            Some(other) if other != id => problems.push(format!(
                "{source}: alias `{k}` is already used by `{other}`"
            )),
            Some(_) => {}
            None => {
                keys.insert(k, id.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUMMARY_OK: &str = "# Summary\n\n- [Memory](hubs/memory.md)\n  - [free vs available](cards/free.md)\n- [Tools](cmd)\n  - [find](cards/find.md)\n";

    fn card(id: &str, title: &str, extra: &str) -> String {
        format!("---\nid: {id}\nkind: concept\ntitle: {title}\n{extra}---\n\n## TL;DR\n\n- yes\n")
    }

    fn hub(id: &str, title: &str, extra: &str) -> String {
        format!("---\nid: {id}\nkind: hub\ntitle: {title}\n{extra}---\n\nIntro.\n")
    }

    fn base() -> BTreeMap<String, String> {
        BTreeMap::from([
            (SUMMARY.to_string(), SUMMARY_OK.to_string()),
            (
                "hubs/memory.md".to_string(),
                hub("memory", "Memory", "aliases: [ram]\n"),
            ),
            (
                "cards/free.md".to_string(),
                card(
                    "memory.free",
                    "free vs available",
                    "aliases: [free]\nhubs: [memory]\nprobes: [mem.summary]\n",
                ),
            ),
            (
                "cards/find.md".to_string(),
                card("cmd.find", "find", "aliases: [find]\n"),
            ),
        ])
    }

    fn problems(files: &BTreeMap<String, String>) -> String {
        Library::from_files(files).unwrap_err().problems.join("\n")
    }

    #[test]
    fn valid_library_is_numbered_and_indexed() {
        let lib = Library::from_files(&base()).unwrap();
        let free = lib.card_by_key("FREE").unwrap();
        assert_eq!(free.number.to_string(), "1.1");
        assert_eq!(
            lib.card_by_key("cmd.find").unwrap().number.to_string(),
            "2.1"
        );
        assert_eq!(lib.hub_by_key("ram").unwrap().chapter, 1);
        assert_eq!(
            lib.card_by_number(Number {
                chapter: 2,
                section: 1
            })
            .unwrap()
            .id,
            "cmd.find"
        );
        assert!(lib
            .card_by_number(Number {
                chapter: 2,
                section: 2
            })
            .is_none());
        assert!(lib
            .card_by_number(Number {
                chapter: 0,
                section: 1
            })
            .is_none());
    }

    #[test]
    fn references_in_text_become_numbers() {
        let mut f = base();
        f.insert(
            "cards/find.md".into(),
            "---\nid: cmd.find\nkind: concept\ntitle: find\n---\n\n## TL;DR\n\n- See {{memory.free}} and {{ memory }}.\n".into(),
        );
        let lib = Library::from_files(&f).unwrap();
        assert_eq!(
            lib.card("cmd.find").unwrap().tldr,
            "- See 1.1 and idk memory."
        );

        f.insert(
            "cards/find.md".into(),
            "---\nid: cmd.find\nkind: concept\ntitle: find\n---\n\n## TL;DR\n\n- See {{nope.card}}.\n".into(),
        );
        assert!(problems(&f).contains("{{nope.card}} names no card or hub"));

        f.insert(
            "cards/find.md".into(),
            "---\nid: cmd.find\nkind: concept\ntitle: find\n---\n\n## TL;DR\n\n- It's in the kernel (see 5.2).\n".into(),
        );
        assert!(
            problems(&f).contains("\"see 5.2\": write {{card.id}}"),
            "{}",
            problems(&f)
        );
    }

    #[test]
    fn duplicate_ids() {
        let mut f = base();
        f.insert("cards/find.md".into(), card("memory.free", "find", ""));
        assert!(problems(&f).contains("id `memory.free` is already used"));
    }

    #[test]
    fn unresolved_references() {
        let mut f = base();
        f.insert(
            "cards/find.md".into(),
            card(
                "cmd.find",
                "find",
                "hubs: [nope]\nsee_also: [missing.card]\n",
            ),
        );
        f.insert(
            "hubs/memory.md".into(),
            hub("memory", "Memory", "cards: [ghost]\n"),
        );
        let p = problems(&f);
        assert!(p.contains("hubs: no hub with id `nope`"), "{p}");
        assert!(
            p.contains("see_also: no card or hub with id `missing.card`"),
            "{p}"
        );
        assert!(p.contains("cards: no card with id `ghost`"), "{p}");
    }

    #[test]
    fn hub_command_card_must_exist() {
        let mut f = base();
        let cmds = "commands:\n  - { cmd: \"free -h\", desc: \"RAM\", card: memory.free }\n  - { cmd: \"x\", desc: \"y\", card: nope }\n";
        f.insert("hubs/memory.md".into(), hub("memory", "Memory", cmds));
        let p = problems(&f);
        assert!(p.contains("`x` points to no card with id `nope`"), "{p}");
        assert!(!p.contains("free -h"), "{p}");
    }

    #[test]
    fn hubs_field_must_name_a_hub() {
        let mut f = base();
        f.insert(
            "cards/find.md".into(),
            card("cmd.find", "find", "hubs: [memory.free]\n"),
        );
        assert!(problems(&f).contains("no hub with id `memory.free`"));
    }

    #[test]
    fn unknown_probe() {
        let mut f = base();
        f.insert(
            "cards/find.md".into(),
            card("cmd.find", "find", "probes: [cpu.magic]\n"),
        );
        assert!(problems(&f).contains("no probe named `cpu.magic`"));
    }

    #[test]
    fn file_not_in_summary() {
        let mut f = base();
        f.insert("cards/extra.md".into(), card("cmd.extra", "extra", ""));
        assert!(problems(&f).contains("cards/extra.md: not listed in SUMMARY.md"));
    }

    #[test]
    fn summary_lists_missing_or_duplicate_file() {
        let mut f = base();
        f.insert(
            SUMMARY.into(),
            format!("{SUMMARY_OK}  - [ghost](cards/ghost.md)\n  - [find](cards/find.md)\n"),
        );
        let p = problems(&f);
        assert!(p.contains("cards/ghost.md does not exist"), "{p}");
        assert!(p.contains("cards/find.md is listed 2 times"), "{p}");
    }

    #[test]
    fn summary_kinds_and_titles_must_match() {
        let mut f = base();
        f.insert(
            SUMMARY.into(),
            "# Summary\n- [find](cards/find.md)\n- [Memory](cmd)\n  - [Memory](hubs/memory.md)\n  - [free](cards/free.md)\n".into(),
        );
        let p = problems(&f);
        assert!(p.contains("a chapter page must be a hub"), "{p}");
        assert!(p.contains("hubs are chapters"), "{p}");
        assert!(p.contains("title `free` differs"), "{p}");
    }

    #[test]
    fn alias_collisions() {
        let mut f = base();
        // card/card: error
        f.insert(
            "cards/find.md".into(),
            card("cmd.find", "find", "aliases: [free]\n"),
        );
        assert!(problems(&f).contains("alias `free` is already used"));

        // hub/card: allowed
        let mut f = base();
        f.insert(
            "cards/find.md".into(),
            card("cmd.find", "find", "aliases: [ram]\n"),
        );
        assert!(Library::from_files(&f).is_ok());
    }

    #[test]
    fn aliases_cannot_shadow_commands_or_numbers() {
        let mut f = base();
        f.insert(
            "cards/find.md".into(),
            card("cmd.find", "find", "aliases: [toc, \"5.1\"]\n"),
        );
        let p = problems(&f);
        assert!(p.contains("`toc` is a command name"), "{p}");
        assert!(p.contains("`5.1` looks like a chapter number"), "{p}");
    }

    #[test]
    fn reports_all_problems_at_once() {
        let mut f = base();
        f.insert(
            "cards/find.md".into(),
            card("cmd.find", "find", "probes: [x]\nhubs: [y]\n"),
        );
        f.insert("notes.txt".into(), String::new());
        assert_eq!(Library::from_files(&f).unwrap_err().problems.len(), 3);
    }
}
