//! `atfq ask`: a ready-to-paste prompt for an AI assistant (ROADMAP.md 8.3).
//! Nothing is sent anywhere: the prompt is printed, the user decides.

use serde::Serialize;

use crate::probes::ProbeCtx;
use crate::render::live_text;
use crate::resolve::Target;
use crate::view::{Lookup, Match};

/// Probe results are laid out this wide in the prompt.
const WIDTH: usize = 80;

/// What the prompt says about the machine, plus what `--redact` hides.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemInfo {
    /// `PRETTY_NAME` from /etc/os-release.
    pub distro: Option<String>,
    pub kernel: Option<String>,
    pub arch: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub home: Option<String>,
}

impl SystemInfo {
    pub fn collect(ctx: &ProbeCtx) -> SystemInfo {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        SystemInfo {
            distro: ctx
                .read("/etc/os-release")
                .ok()
                .and_then(|t| os_release(&t)),
            kernel: ctx.read_trimmed("/proc/sys/kernel/osrelease"),
            arch: std::env::consts::ARCH.to_string(),
            hostname: ctx.read_trimmed("/proc/sys/kernel/hostname"),
            user: env("USER").or_else(|| env("LOGNAME")),
            home: env("HOME"),
        }
    }
}

/// `PRETTY_NAME="Ubuntu 24.04.1 LTS"` → `Ubuntu 24.04.1 LTS`.
fn os_release(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let v = l.strip_prefix("PRETTY_NAME=")?;
        let v = v.trim().trim_matches('"').trim_matches('\'');
        (!v.is_empty()).then(|| v.to_string())
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct Prompt {
    pub text: String,
    /// What system data the prompt contains, for the user to check.
    pub included: Vec<String>,
    /// Hostname, username and home paths were replaced.
    pub redacted: bool,
}

pub fn build(l: &Lookup<'_>, question: Option<&str>, sys: &SystemInfo, redact: bool) -> Prompt {
    let mut included = Vec::new();
    let mut out = String::from("I have a question about my Linux machine.\n\n");

    let topic = l.entry.map(|t| t.title().to_string());
    let question = match (question.map(str::trim).filter(|q| !q.is_empty()), &topic) {
        (Some(q), _) => q.to_string(),
        (None, Some(t)) => format!("Help me understand \"{t}\" on my machine."),
        (None, None) => l.query.clone(),
    };
    out.push_str(&format!("## My question\n\n{question}\n\n## My system\n\n"));

    let mut system = Vec::new();
    if let Some(d) = &sys.distro {
        system.push(format!("- Distribution: {d}"));
        included.push("distribution".to_string());
    }
    if let Some(k) = &sys.kernel {
        system.push(format!("- Kernel: {k}"));
        included.push("kernel version".to_string());
    }
    system.push(format!("- Architecture: {}", sys.arch));
    included.push("architecture".to_string());
    out.push_str(&system.join("\n"));
    out.push_str("\n\n");

    if let Some(target) = l.entry {
        let source = match target {
            Target::Hub(h) => format!("hub \"{}\"", h.title),
            Target::Card(c) => format!("card {} \"{}\"", c.number, c.title),
        };
        out.push_str(&format!("## Background (from the atfq {source})\n\n"));
        out.push_str(&background(target));
        out.push_str("\n\n");
    }

    if !l.probes.is_empty() {
        let names: Vec<&str> = l.probes.iter().map(|p| p.name).collect();
        included.push(format!("live data ({})", names.join(", ")));
        out.push_str("## What my machine shows right now\n\n```\n");
        out.push_str(&live_text(&l.probes, WIDTH));
        out.push_str("```\n\n");
    }

    out.push_str(
        "## How to answer\n\n\
         Answer concisely, in plain English, as you would to a developer who is \
         not a sysadmin. Start with the direct answer. If you suggest commands, \
         give real examples and say clearly which ones change the system.\n",
    );

    let text = if redact { redact_text(&out, sys) } else { out };
    Prompt {
        text,
        included,
        redacted: redact,
    }
}

/// The TL;DR of a card, or the intro and commands of a hub, as Markdown.
fn background(target: Target<'_>) -> String {
    match target {
        Target::Card(c) => c.tldr.clone(),
        Target::Hub(h) => {
            let mut s = h.intro.clone();
            if !h.commands.is_empty() {
                s.push_str("\n\nUseful commands:\n");
                for c in &h.commands {
                    s.push_str(&format!("\n- `{}`: {}", c.cmd, c.desc));
                }
            }
            s
        }
    }
}

/// Replaces the home directory with `~`, any other `/home/NAME` with
/// `/home/<user>`, and the hostname and username, as whole words, with
/// `<hostname>` and `<user>`.
pub fn redact_text(text: &str, sys: &SystemInfo) -> String {
    let mut s = text.to_string();
    if let Some(home) = sys.home.as_deref().filter(|h| h.len() > 1) {
        s = s.replace(home.trim_end_matches('/'), "~");
    }
    s = redact_home_dirs(&s);
    if let Some(h) = &sys.hostname {
        s = replace_word(&s, h, "<hostname>");
    }
    if let Some(u) = &sys.user {
        s = replace_word(&s, u, "<user>");
    }
    s
}

fn redact_home_dirs(s: &str) -> String {
    const HOME: &str = "/home/";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find(HOME) {
        out.push_str(&rest[..i + HOME.len()]);
        rest = &rest[i + HOME.len()..];
        let end = rest
            .find(|c: char| c == '/' || c.is_whitespace() || c == '`' || c == '"')
            .unwrap_or(rest.len());
        if end > 0 {
            out.push_str("<user>");
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Replaces `word` where it is not part of a longer word.
fn replace_word(s: &str, word: &str, with: &str) -> String {
    if word.is_empty() {
        return s.to_string();
    }
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    for (i, _) in s.match_indices(word) {
        if i < last {
            continue;
        }
        let before = s[..i].chars().next_back();
        let after = s[i + word.len()..].chars().next();
        if before.is_some_and(is_word) || after.is_some_and(is_word) {
            continue;
        }
        out.push_str(&s[last..i]);
        out.push_str(with);
        last = i + word.len();
    }
    out.push_str(&s[last..]);
    out
}

/// One line for stderr: what the prompt tells about this machine.
pub fn summary(p: &Prompt, l: &Lookup<'_>) -> String {
    let mut s = format!("Included: {}.", p.included.join(", "));
    if p.redacted {
        s.push_str(" Redacted: hostname, username, home paths.");
    }
    if l.matched == Match::None {
        s.push_str(&format!(" No card matches \"{}\": no background.", l.query));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pretty_name() {
        let t = "NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nID=ubuntu\n";
        assert_eq!(os_release(t).as_deref(), Some("Ubuntu 24.04.1 LTS"));
        assert_eq!(os_release("ID=arch\n"), None);
    }

    #[test]
    fn replaces_whole_words_only() {
        assert_eq!(
            replace_word("al alice malice al_x al", "al", "U"),
            "U alice malice al_x U"
        );
        assert_eq!(replace_word("x", "", "U"), "x");
    }

    #[test]
    fn redacts_home_user_and_host() {
        let sys = SystemInfo {
            hostname: Some("alice-laptop".into()),
            user: Some("alice".into()),
            home: Some("/home/alice".into()),
            ..SystemInfo::default()
        };
        let text = "alice on alice-laptop: /home/alice/big, /home/bob/x, /home/alice";
        assert_eq!(
            redact_text(text, &sys),
            "<user> on <hostname>: ~/big, /home/<user>/x, ~"
        );
    }
}
