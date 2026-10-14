//! Cards and hubs: parsing, validation, and the index used to look them up.

mod entry;
mod library;
mod summary;

pub mod danger;

use std::fmt;

use serde::Deserialize;

pub use library::{Chapter, ChapterKind, Library, LoadError};

/// A chapter number such as `5.1`, derived from SUMMARY.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Number {
    pub chapter: u32,
    pub section: u32,
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.chapter, self.section)
    }
}

/// A concept card: one precise question, answered.
#[derive(Debug, Clone)]
pub struct Card {
    pub id: String,
    pub title: String,
    pub number: Number,
    pub aliases: Vec<String>,
    pub hubs: Vec<String>,
    pub probes: Vec<String>,
    pub see_also: Vec<String>,
    pub commands: Vec<String>,
    pub paths: Vec<String>,
    /// Markdown body of `## TL;DR`.
    pub tldr: String,
    /// Markdown body of `## Pitfalls`, if the card has one.
    pub pitfalls: Option<String>,
    /// One entry per `###` under `## Deep dive`.
    pub deep_dive: Vec<Question>,
    /// Path relative to `content/`.
    pub source: String,
}

/// A deep dive question and its Markdown answer.
#[derive(Debug, Clone)]
pub struct Question {
    pub title: String,
    pub body: String,
}

/// A hub: the entry point for a device or a domain.
#[derive(Debug, Clone)]
pub struct Hub {
    pub id: String,
    pub title: String,
    pub chapter: u32,
    pub aliases: Vec<String>,
    pub probes: Vec<String>,
    pub commands: Vec<HubCommand>,
    pub paths: Vec<HubPath>,
    /// Related cards outside the hub's own chapter.
    pub cards: Vec<String>,
    /// Markdown intro, one or two sentences.
    pub intro: String,
    /// Path relative to `content/`.
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubCommand {
    pub cmd: String,
    pub desc: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubPath {
    pub path: String,
    pub desc: String,
}

/// Lookup key for ids and aliases: lowercase, single spaces.
pub fn normalize(key: &str) -> String {
    key.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
