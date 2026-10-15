//! SUMMARY.md: the table of contents, mdBook-style.
//!
//! The format is deliberately strict: a `# Summary` title, then chapters as
//! top-level list items and entries as items indented by two spaces.
//!
//! ```text
//! - [Memory](hubs/memory.md)        chapter backed by a hub
//!   - [free vs available](cards/memory/free-vs-available.md)
//! - [Finding and transforming](cmd) chapter without a page
//! ```
//!
//! As in mdBook, a `# Title` line after the first starts a part; chapter
//! numbers keep counting across parts.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SummaryChapter {
    pub title: String,
    /// The part this chapter belongs to, if the summary has parts.
    pub part: Option<String>,
    /// `Some(path)` when the chapter is a hub file, `None` for a plain chapter.
    pub file: Option<String>,
    pub entries: Vec<SummaryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SummaryEntry {
    pub title: String,
    pub file: String,
}

pub(crate) fn parse(src: &str) -> Result<Vec<SummaryChapter>, Vec<String>> {
    let mut chapters: Vec<SummaryChapter> = Vec::new();
    let mut problems = Vec::new();
    let mut seen_title = false;
    let mut part: Option<String> = None;

    for (i, line) in src.lines().enumerate() {
        let at = |msg: &str| format!("SUMMARY.md:{}: {msg}", i + 1);
        if line.trim().is_empty() {
            continue;
        }
        if !seen_title {
            if line.trim_end() == "# Summary" {
                seen_title = true;
            } else {
                problems.push(at("expected `# Summary` as the first line"));
                return Err(problems);
            }
            continue;
        }
        if let Some(title) = line.strip_prefix("# ") {
            if title.trim().is_empty() {
                problems.push(at("a part needs a title"));
            }
            part = Some(title.trim().to_string());
        } else if let Some(item) = line.strip_prefix("- ") {
            match parse_link(item) {
                Some((title, target)) => chapters.push(SummaryChapter {
                    title,
                    part: part.clone(),
                    file: is_file(&target).then_some(target),
                    entries: Vec::new(),
                }),
                None => problems.push(at("expected `- [Title](target)`")),
            }
        } else if let Some(item) = line.strip_prefix("  - ") {
            let Some((title, file)) = parse_link(item) else {
                problems.push(at("expected `  - [Title](path.md)`"));
                continue;
            };
            if !is_file(&file) {
                problems.push(at("an entry must point to a .md file"));
            } else if let Some(chapter) = chapters.last_mut() {
                chapter.entries.push(SummaryEntry { title, file });
            } else {
                problems.push(at("entry before any chapter"));
            }
        } else {
            problems.push(at(
                "expected a chapter `- [..](..)` or an entry indented by two spaces",
            ));
        }
    }

    if !seen_title {
        problems.push("SUMMARY.md: empty file".to_string());
    }
    if problems.is_empty() {
        Ok(chapters)
    } else {
        Err(problems)
    }
}

/// Parses `[Title](target)`, with nothing after the closing parenthesis.
fn parse_link(s: &str) -> Option<(String, String)> {
    let rest = s.trim_end().strip_prefix('[')?;
    let (title, rest) = rest.split_once("](")?;
    let target = rest.strip_suffix(')')?;
    if title.is_empty() || target.is_empty() || target.contains(['(', ')', ' ']) {
        return None;
    }
    Some((title.to_string(), target.to_string()))
}

fn is_file(target: &str) -> bool {
    target.ends_with(".md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chapters_and_entries() {
        let src = "# Summary\n\n- [Filesystem](fs)\n- [Memory](hubs/memory.md)\n  - [free vs available](cards/memory/free.md)\n";
        let chapters = parse(src).unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Filesystem");
        assert_eq!(chapters[0].file, None);
        assert_eq!(chapters[1].file.as_deref(), Some("hubs/memory.md"));
        assert_eq!(chapters[1].entries[0].title, "free vs available");
        assert_eq!(chapters[1].entries[0].file, "cards/memory/free.md");
    }

    #[test]
    fn parts_group_chapters() {
        let src = "# Summary\n\n- [Intro](intro)\n\n# How it works\n\n- [Kernel](kernel)\n- [Net](net)\n\n# Your system\n\n- [Disk](hubs/disk.md)\n";
        let ch = parse(src).unwrap();
        let parts: Vec<Option<&str>> = ch.iter().map(|c| c.part.as_deref()).collect();
        assert_eq!(
            parts,
            [
                None,
                Some("How it works"),
                Some("How it works"),
                Some("Your system")
            ]
        );
        assert!(parse("# Summary\n# \n- [A](a)\n").unwrap_err()[0].contains("part needs a title"));
    }

    #[test]
    fn rejects_missing_title() {
        let err = parse("- [Filesystem](fs)\n").unwrap_err();
        assert!(err[0].contains("# Summary"), "{err:?}");
    }

    #[test]
    fn rejects_bad_lines_with_line_numbers() {
        let src = "# Summary\n- [A](a)\n    - [B](b.md)\n  - [C](c)\n- broken\n";
        let err = parse(src).unwrap_err();
        assert_eq!(err.len(), 3, "{err:?}");
        assert!(err[0].starts_with("SUMMARY.md:3:"));
        assert!(err[1].contains("must point to a .md file"));
        assert!(err[2].starts_with("SUMMARY.md:5:"));
    }

    #[test]
    fn rejects_entry_before_chapter() {
        let err = parse("# Summary\n  - [B](b.md)\n").unwrap_err();
        assert!(err[0].contains("before any chapter"));
    }
}
