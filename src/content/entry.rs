//! One content file: YAML frontmatter plus a Markdown body.

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::Deserialize;

use super::{Card, Hub, HubCommand, HubPath, Number, Question};

pub(crate) const TLDR: &str = "TL;DR";
pub(crate) const PITFALLS: &str = "Pitfalls";
pub(crate) const DEEP_DIVE: &str = "Deep dive";
const TLDR_MAX_LINES: usize = 12;

#[derive(Debug)]
pub(crate) enum Entry {
    Card(Card),
    Hub(Hub),
}

#[derive(Deserialize)]
struct KindOnly {
    kind: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CardFront {
    id: String,
    #[allow(dead_code)]
    kind: String,
    title: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    hubs: Vec<String>,
    #[serde(default)]
    probes: Vec<String>,
    #[serde(default)]
    see_also: Vec<String>,
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default)]
    paths: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HubFront {
    id: String,
    #[allow(dead_code)]
    kind: String,
    title: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    probes: Vec<String>,
    #[serde(default)]
    commands: Vec<HubCommand>,
    #[serde(default)]
    paths: Vec<HubPath>,
    #[serde(default)]
    cards: Vec<String>,
}

/// Parses one file. `path` is relative to `content/` and only used in messages.
pub(crate) fn parse(path: &str, src: &str) -> Result<Entry, Vec<String>> {
    let src = src.replace("\r\n", "\n");
    let fail = |msg: String| vec![format!("{path}: {msg}")];
    let (front, body) = split_frontmatter(&src).ok_or_else(|| {
        fail("missing frontmatter: the file must start with `---` and close it with `---`".into())
    })?;

    let kind: KindOnly =
        serde_yaml::from_str(front).map_err(|e| fail(format!("frontmatter: {e}")))?;
    match kind.kind.as_str() {
        "concept" => {
            let f: CardFront =
                serde_yaml::from_str(front).map_err(|e| fail(format!("frontmatter: {e}")))?;
            let sections = card_sections(body).map_err(|errs| {
                errs.into_iter()
                    .map(|e| format!("{path}: {e}"))
                    .collect::<Vec<_>>()
            })?;
            Ok(Entry::Card(Card {
                id: f.id,
                title: f.title,
                number: Number::default(),
                aliases: f.aliases,
                hubs: f.hubs,
                probes: f.probes,
                see_also: f.see_also,
                commands: f.commands,
                paths: f.paths,
                tldr: sections.tldr,
                pitfalls: sections.pitfalls,
                deep_dive: sections.deep_dive,
                source: path.to_string(),
            }))
        }
        "hub" => {
            let f: HubFront =
                serde_yaml::from_str(front).map_err(|e| fail(format!("frontmatter: {e}")))?;
            if !headings(body).is_empty() {
                return Err(fail("a hub body is a short intro, without headings".into()));
            }
            let intro = body.trim().to_string();
            if intro.is_empty() {
                return Err(fail("a hub needs a one or two sentence intro".into()));
            }
            Ok(Entry::Hub(Hub {
                id: f.id,
                title: f.title,
                chapter: 0,
                aliases: f.aliases,
                probes: f.probes,
                commands: f.commands,
                paths: f.paths,
                cards: f.cards,
                intro,
                source: path.to_string(),
            }))
        }
        other => Err(fail(format!(
            "unknown kind `{other}`, expected `concept` or `hub`"
        ))),
    }
}

/// Returns (frontmatter, body).
fn split_frontmatter(src: &str) -> Option<(&str, &str)> {
    let rest = src.strip_prefix("---\n")?;
    if let Some(body) = rest.strip_prefix("---\n") {
        return Some(("", body));
    }
    let end = rest.find("\n---\n").map(|i| (i, i + 5)).or_else(|| {
        rest.strip_suffix("\n---")
            .map(|front| (front.len(), rest.len()))
    })?;
    Some((&rest[..end.0], &rest[end.1..]))
}

struct Heading {
    level: HeadingLevel,
    text: String,
    /// Byte range of the heading line itself.
    start: usize,
    end: usize,
}

fn headings(body: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut current: Option<Heading> = None;
    for (event, range) in Parser::new(body).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some(Heading {
                    level,
                    text: String::new(),
                    start: range.start,
                    end: range.end,
                });
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some(h) = current.as_mut() {
                    h.text.push_str(&t);
                }
            }
            Event::End(TagEnd::Heading(_)) => out.extend(current.take()),
            _ => {}
        }
    }
    out
}

struct Sections {
    tldr: String,
    pitfalls: Option<String>,
    deep_dive: Vec<Question>,
}

fn card_sections(body: &str) -> Result<Sections, Vec<String>> {
    let hs = headings(body);
    let mut problems = Vec::new();

    for h in &hs {
        if h.level != HeadingLevel::H2 && h.level != HeadingLevel::H3 {
            problems.push(format!(
                "heading `{}`: only `##` sections and `###` deep dive questions are allowed",
                h.text
            ));
        }
    }
    let first = hs.first().map_or(body.len(), |h| h.start);
    if !body[..first].trim().is_empty() {
        problems.push("text before `## TL;DR`".to_string());
    }

    // Each `##` section runs until the next `##`.
    let h2: Vec<usize> = (0..hs.len())
        .filter(|&i| hs[i].level == HeadingLevel::H2)
        .collect();
    let order = [TLDR, PITFALLS, DEEP_DIVE];
    let mut expected = 0;
    let mut sections = Sections {
        tldr: String::new(),
        pitfalls: None,
        deep_dive: Vec::new(),
    };
    let mut found_tldr = false;

    for (n, &i) in h2.iter().enumerate() {
        let name = hs[i].text.as_str();
        let next = h2.get(n + 1).copied().unwrap_or(hs.len());
        let end = hs.get(next).map_or(body.len(), |h| h.start);
        let pos = order.iter().position(|&s| s == name);
        match pos {
            Some(p) if p >= expected => expected = p + 1,
            Some(_) => {
                problems.push(format!(
                    "section `## {name}` is out of order: expected TL;DR, Pitfalls, Deep dive"
                ));
                continue;
            }
            None => {
                problems.push(format!(
                    "unknown section `## {name}`: expected TL;DR, Pitfalls or Deep dive"
                ));
                continue;
            }
        }
        let questions = &hs[i + 1..next];
        if name != DEEP_DIVE {
            if let Some(q) = questions.first() {
                problems.push(format!(
                    "`### {}` is only allowed under `## Deep dive`",
                    q.text
                ));
            }
        }
        let text = body[hs[i].end..end].trim().to_string();
        match name {
            TLDR => {
                found_tldr = true;
                let lines = tldr_lines(&text);
                if text.is_empty() {
                    problems.push("`## TL;DR` is empty".to_string());
                } else if lines > TLDR_MAX_LINES {
                    problems.push(format!(
                        "`## TL;DR` has {lines} lines, the limit is {TLDR_MAX_LINES}"
                    ));
                }
                sections.tldr = text;
            }
            PITFALLS => {
                if text.is_empty() {
                    problems.push("`## Pitfalls` is empty: remove it".to_string());
                }
                sections.pitfalls = Some(text);
            }
            _ => {
                let intro_end = questions.first().map_or(end, |q| q.start);
                if !body[hs[i].end..intro_end].trim().is_empty() {
                    problems.push(
                        "`## Deep dive` holds only `###` questions, no text before the first one"
                            .to_string(),
                    );
                }
                if questions.is_empty() {
                    problems.push("`## Deep dive` has no `###` question: remove it".to_string());
                }
                for (k, q) in questions.iter().enumerate() {
                    let q_end = questions.get(k + 1).map_or(end, |n| n.start);
                    sections.deep_dive.push(Question {
                        title: q.text.clone(),
                        body: body[q.end..q_end].trim().to_string(),
                    });
                }
            }
        }
    }
    if !found_tldr {
        problems.push("missing `## TL;DR`".to_string());
    }

    if problems.is_empty() {
        Ok(sections)
    } else {
        Err(problems)
    }
}

/// Counts the lines a TL;DR takes on screen: non-blank source lines, minus
/// code fences, which are not displayed.
fn tldr_lines(text: &str) -> usize {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("```") && !l.starts_with("~~~"))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(body: &str) -> Result<Card, Vec<String>> {
        let src = format!("---\nid: a.b\nkind: concept\ntitle: T\n---\n{body}");
        match parse("cards/a/b.md", &src)? {
            Entry::Card(c) => Ok(c),
            Entry::Hub(_) => panic!("expected a card"),
        }
    }

    fn errors(body: &str) -> String {
        card(body).unwrap_err().join("\n")
    }

    #[test]
    fn splits_sections_and_questions() {
        let c = card(
            "\n## TL;DR\n\n- one\n\n## Pitfalls\n\n- p\n\n## Deep dive\n\n### Why?\n\nBecause.\n\n### How?\n\nLike this.\n",
        )
        .unwrap();
        assert_eq!(c.tldr, "- one");
        assert_eq!(c.pitfalls.as_deref(), Some("- p"));
        assert_eq!(c.deep_dive.len(), 2);
        assert_eq!(c.deep_dive[0].title, "Why?");
        assert_eq!(c.deep_dive[0].body, "Because.");
        assert_eq!(c.deep_dive[1].body, "Like this.");
    }

    #[test]
    fn pitfalls_and_deep_dive_are_optional() {
        let c = card("## TL;DR\n\n- one\n").unwrap();
        assert!(c.pitfalls.is_none());
        assert!(c.deep_dive.is_empty());
    }

    #[test]
    fn requires_tldr() {
        assert!(errors("## Pitfalls\n\n- p\n").contains("missing `## TL;DR`"));
    }

    #[test]
    fn rejects_long_tldr() {
        let body = format!("## TL;DR\n\n{}", "- line\n".repeat(13));
        assert!(errors(&body).contains("13 lines"));
    }

    #[test]
    fn code_fences_do_not_count_as_tldr_lines() {
        let body = format!("## TL;DR\n\n{}\n```\nfree -h\n```\n", "- line\n".repeat(11));
        assert!(card(&body).is_ok());
    }

    #[test]
    fn rejects_wrong_order_and_unknown_sections() {
        let e = errors("## TL;DR\n\n- a\n\n## Deep dive\n\n### Q\n\nA\n\n## Pitfalls\n\n- p\n");
        assert!(e.contains("out of order"), "{e}");
        let e = errors("## TL;DR\n\n- a\n\n## Examples\n\n- x\n");
        assert!(e.contains("unknown section `## Examples`"), "{e}");
    }

    #[test]
    fn rejects_misplaced_headings_and_text() {
        assert!(errors("Intro.\n\n## TL;DR\n\n- a\n").contains("text before"));
        assert!(errors("## TL;DR\n\n### Q\n\n- a\n").contains("only allowed under"));
        assert!(errors("## TL;DR\n\n- a\n\n#### Deep\n").contains("only `##` sections"));
        assert!(
            errors("## TL;DR\n\n- a\n\n## Deep dive\n\nIntro.\n\n### Q\n\nA\n")
                .contains("no text before the first one")
        );
    }

    #[test]
    fn rejects_bad_frontmatter() {
        let err = parse("x.md", "no frontmatter").unwrap_err();
        assert!(err[0].contains("missing frontmatter"));
        let err = parse("x.md", "---\nid: a\nkind: recipe\ntitle: T\n---\n").unwrap_err();
        assert!(err[0].contains("unknown kind `recipe`"));
        let err = parse(
            "x.md",
            "---\nid: a\nkind: concept\ntitle: T\nalias: [x]\n---\n## TL;DR\n\n- a\n",
        )
        .unwrap_err();
        assert!(err[0].contains("unknown field `alias`"), "{err:?}");
    }

    #[test]
    fn parses_hub() {
        let src = "---\nid: disk\nkind: hub\ntitle: Disk\ncommands:\n  - { cmd: \"lsblk\", desc: \"tree\" }\n---\n\nAll about disks.\n";
        let Entry::Hub(h) = parse("hubs/disk.md", src).unwrap() else {
            panic!("expected a hub")
        };
        assert_eq!(h.commands[0].cmd, "lsblk");
        assert_eq!(h.intro, "All about disks.");
    }

    #[test]
    fn hub_needs_intro_without_headings() {
        let head = "---\nid: disk\nkind: hub\ntitle: Disk\n---\n";
        assert!(parse("h.md", head).unwrap_err()[0].contains("intro"));
        let err = parse("h.md", &format!("{head}Intro.\n\n## More\n")).unwrap_err();
        assert!(err[0].contains("without headings"));
    }
}
