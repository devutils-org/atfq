//! `git.config`: what your git configuration sets, from the system and
//! global files. Shows whether your name and email are set, never their
//! values.

use serde::Serialize;

use super::shell::account;
use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "git.config";

/// Settings shown with their value: they change how git behaves, and hold
/// nothing personal.
const SHOWN: &[&str] = &[
    "init.defaultbranch",
    "pull.rebase",
    "pull.ff",
    "push.autosetupremote",
    "push.default",
    "merge.conflictstyle",
    "rebase.autostash",
    "rebase.autosquash",
    "fetch.prune",
    "core.editor",
    "core.autocrlf",
    "credential.helper",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitConfig {
    /// Config files that exist, as `~/.gitconfig`, in the order git reads them.
    pub files: Vec<String>,
    pub name_set: bool,
    pub email_set: bool,
    /// Settings out of [`SHOWN`] that are set, with their effective value.
    pub settings: Vec<(String, String)>,
    /// Names of aliases (`git st`…).
    pub aliases: Vec<String>,
    /// `[include]` or `[includeIf]` sections: other files are read too.
    pub includes: u32,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let home = account(ctx).1;
    // System, then XDG, then ~/.gitconfig: a later file wins.
    let mut candidates = vec![("/etc/gitconfig".to_string(), "/etc/gitconfig".to_string())];
    if let Some(h) = &home {
        candidates.push((
            format!("{h}/.config/git/config"),
            "~/.config/git/config".into(),
        ));
        candidates.push((format!("{h}/.gitconfig"), "~/.gitconfig".into()));
    }
    let mut data = GitConfig {
        files: Vec::new(),
        name_set: false,
        email_set: false,
        settings: Vec::new(),
        aliases: Vec::new(),
        includes: 0,
    };
    for (path, shown) in candidates {
        let Ok(text) = ctx.read(&path) else { continue };
        data.files.push(shown);
        for (key, value) in parse(&text) {
            if key.starts_with("include.") || key.starts_with("includeif.") {
                if key.ends_with(".path") {
                    data.includes += 1;
                }
            } else if let Some(alias) = key.strip_prefix("alias.") {
                if !data.aliases.iter().any(|a| a == alias) {
                    data.aliases.push(alias.to_string());
                }
            } else if key == "user.name" {
                data.name_set = !value.is_empty();
            } else if key == "user.email" {
                data.email_set = !value.is_empty();
            } else if SHOWN.contains(&key.as_str()) {
                data.settings.retain(|(k, _)| *k != key);
                data.settings.push((key, value));
            }
        }
    }
    data.settings
        .sort_by_key(|(k, _)| SHOWN.iter().position(|s| s == k));
    let mut r = ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Git(data)));
    if home.is_none() {
        r.status = Status::Partial;
        r.notes
            .push("home folder unknown: only /etc/gitconfig was read".to_string());
    }
    r
}

/// Git's ini format → `section.key` (or `section.sub.key`) and value.
/// Section and key names are case-insensitive; subsections aren't.
fn parse(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(head) = line.strip_prefix('[') {
            let head = head.split(']').next().unwrap_or("");
            section = match head.split_once(' ') {
                Some((name, sub)) => {
                    format!(
                        "{}.{}",
                        name.to_ascii_lowercase(),
                        sub.trim().trim_matches('"')
                    )
                }
                None => head.to_ascii_lowercase(),
            };
            continue;
        }
        let (key, value) = match line.split_once('=') {
            Some((k, v)) => (k.trim(), strip_value(v)),
            // A key alone means true.
            None => (line, "true".to_string()),
        };
        out.push((format!("{section}.{}", key.to_ascii_lowercase()), value));
    }
    out
}

/// Drops an unquoted trailing comment and the quotes around a value.
fn strip_value(v: &str) -> String {
    let mut out = String::new();
    let mut quoted = false;
    for c in v.trim().chars() {
        match c {
            '"' => quoted = !quoted,
            '#' | ';' if !quoted => break,
            _ => out.push(c),
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn global_wins_over_system() {
        let Some(ProbeData::Git(g)) = probe(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(g.files, ["/etc/gitconfig", "~/.gitconfig"]);
        assert!(g.name_set && g.email_set);
        let settings: Vec<(&str, &str)> = g
            .settings
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        assert_eq!(
            settings,
            [
                ("init.defaultbranch", "main"),
                ("pull.rebase", "true"),
                ("core.autocrlf", "input"),
                ("credential.helper", "store"),
            ]
        );
        assert_eq!(g.aliases, ["st", "lg"]);
        assert_eq!(g.includes, 1);
    }

    #[test]
    fn parses_git_ini() {
        let got = parse(
            "[Core]\n\tEditor = \"vim -u NONE\" ; comment\n\tbare\n[remote \"origin\"]\n\turl = x\n",
        );
        assert_eq!(
            got,
            [
                ("core.editor".to_string(), "vim -u NONE".to_string()),
                ("core.bare".to_string(), "true".to_string()),
                ("remote.origin.url".to_string(), "x".to_string()),
            ]
        );
    }

    #[test]
    fn nothing_configured() {
        let Some(ProbeData::Git(g)) = probe(&fixture("empty")).data else {
            panic!()
        };
        assert!(g.files.is_empty());
        assert!(!g.name_set);
    }
}
