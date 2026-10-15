//! `ssh.setup`: your SSH keys, client config and known hosts, the agent, and
//! the SSH server's main settings. Never reads a private key: only file
//! names, permissions, and the type field of public keys.

use serde::Serialize;

use super::shell::account;
use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "ssh.setup";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SshSetup {
    /// ~/.ssh exists.
    pub dir: bool,
    /// Permission bits of ~/.ssh: ssh refuses some files in a folder others
    /// can write to.
    pub dir_mode: Option<u32>,
    pub keys: Vec<SshKey>,
    /// `Host` blocks in ~/.ssh/config, `Host *` left out.
    pub config_hosts: u32,
    /// The config uses ProxyJump (or ProxyCommand) somewhere.
    pub jump: bool,
    /// Lines in ~/.ssh/known_hosts, and whether their names are hashed.
    pub known_hosts: Option<u32>,
    pub hashed: bool,
    /// Keys allowed to log in to this machine as you.
    pub authorized_keys: Option<u32>,
    /// SSH_AUTH_SOCK is set: an agent holds keys for this session.
    pub agent: bool,
    /// The server's effective settings, from /etc/ssh/sshd_config and its
    /// drop-ins; `None` without a server config.
    pub server: Option<SshServer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SshKey {
    /// `id_ed25519`.
    pub name: String,
    /// `ssh-ed25519`, `ssh-rsa`… from the .pub file.
    pub kind: Option<String>,
    /// Permission bits of the private key; ssh refuses it when others can
    /// read it.
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SshServer {
    pub port: Option<String>,
    pub password_auth: Option<String>,
    pub root_login: Option<String>,
    /// Some file couldn't be read (sshd_config is often root-only).
    pub unreadable: bool,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let Some(home) = account(ctx).1 else {
        let mut r = ProbeResult::new(NAME, Status::Unavailable, None);
        r.notes
            .push("home folder unknown: no entry in /etc/passwd".to_string());
        return r;
    };
    let dir = format!("{home}/.ssh");
    let entries = ctx.list(&dir).unwrap_or_default();
    // A key is a private file with a matching .pub next to it.
    let keys = entries
        .iter()
        .filter_map(|n| n.strip_suffix(".pub"))
        .filter(|name| entries.iter().any(|e| e == name))
        .map(|name| SshKey {
            name: name.to_string(),
            kind: ctx
                .read(&format!("{dir}/{name}.pub"))
                .ok()
                .and_then(|p| p.split_whitespace().next().map(str::to_string)),
            mode: ctx.mode(&format!("{dir}/{name}")),
        })
        .collect();
    let config = ctx.read(&format!("{dir}/config")).unwrap_or_default();
    let mut config_hosts = 0;
    let mut jump = false;
    for line in config.lines() {
        let mut words = line.split_whitespace();
        let Some(key) = words.next() else { continue };
        let key = key.to_ascii_lowercase();
        if key == "host" && words.any(|w| w != "*") {
            config_hosts += 1;
        }
        jump |= key == "proxyjump" || key == "proxycommand";
    }
    let lines = |f: &str| {
        ctx.read(&format!("{dir}/{f}")).ok().map(|t| {
            t.lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .count() as u32
        })
    };
    let known = ctx.read(&format!("{dir}/known_hosts")).unwrap_or_default();
    let agent = ctx.read_bytes("/proc/self/environ").is_ok_and(|e| {
        e.split(|b| *b == 0)
            .any(|kv| kv.starts_with(b"SSH_AUTH_SOCK="))
    });
    let data = SshSetup {
        dir: ctx.exists(&dir),
        dir_mode: ctx.mode(&dir),
        keys,
        config_hosts,
        jump,
        known_hosts: lines("known_hosts"),
        hashed: known.lines().any(|l| l.starts_with("|1|")),
        authorized_keys: lines("authorized_keys"),
        agent,
        server: server(ctx),
    };
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Ssh(data)))
}

/// sshd keeps the first value it reads for each setting, and reads drop-ins
/// where `Include` appears: at the top on Debian, Ubuntu and Fedora, so a
/// drop-in wins over the main file.
fn server(ctx: &ProbeCtx) -> Option<SshServer> {
    const MAIN: &str = "/etc/ssh/sshd_config";
    if !ctx.exists(MAIN) {
        return None;
    }
    let mut s = SshServer {
        port: None,
        password_auth: None,
        root_login: None,
        unreadable: false,
    };
    read_sshd(ctx, MAIN, &mut s, 0);
    Some(s)
}

fn read_sshd(ctx: &ProbeCtx, path: &str, s: &mut SshServer, depth: u32) {
    // sshd itself stops at a few levels of Include.
    if depth > 4 {
        return;
    }
    let Ok(text) = ctx.read(path) else {
        s.unreadable = true;
        return;
    };
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let (Some(key), Some(value)) = (words.next(), words.next()) else {
            continue;
        };
        let slot = match key.to_ascii_lowercase().as_str() {
            // What follows `Match` only applies to some connections.
            "match" => return,
            "include" => {
                for file in expand(ctx, value) {
                    read_sshd(ctx, &file, s, depth + 1);
                }
                continue;
            }
            "port" => &mut s.port,
            "passwordauthentication" => &mut s.password_auth,
            "permitrootlogin" => &mut s.root_login,
            _ => continue,
        };
        slot.get_or_insert_with(|| value.to_string());
    }
}

/// `sshd_config.d/*.conf` → the matching files, sorted, as sshd globs them.
fn expand(ctx: &ProbeCtx, pattern: &str) -> Vec<String> {
    let pattern = if pattern.starts_with('/') {
        pattern.to_string()
    } else {
        format!("/etc/ssh/{pattern}")
    };
    match pattern.rsplit_once("/*") {
        Some((folder, suffix)) => ctx
            .list(folder)
            .unwrap_or_default()
            .into_iter()
            .filter(|n| n.ends_with(suffix))
            .map(|n| format!("{folder}/{n}"))
            .collect(),
        None => vec![pattern],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    fn setup(ctx: &ProbeCtx) -> SshSetup {
        let Some(ProbeData::Ssh(s)) = probe(ctx).data else {
            panic!()
        };
        s
    }

    #[test]
    fn keys_config_and_known_hosts() {
        let ctx = fixture("laptop").with_mode("/home/alice/.ssh/id_rsa_old", 0o644);
        let s = setup(&ctx);
        let keys: Vec<(&str, Option<&str>, Option<u32>)> = s
            .keys
            .iter()
            .map(|k| (k.name.as_str(), k.kind.as_deref(), k.mode))
            .collect();
        assert_eq!(
            keys,
            [
                ("id_ed25519", Some("ssh-ed25519"), None),
                ("id_rsa_old", Some("ssh-rsa"), Some(0o644)),
            ]
        );
        // `Host *` doesn't count; `Host db-prod web-prod` is one entry.
        assert_eq!(s.config_hosts, 3);
        assert!(s.jump);
        assert_eq!(s.known_hosts, Some(4));
        assert!(s.hashed);
        assert_eq!(s.authorized_keys, Some(1));
        assert!(!s.agent);
    }

    #[test]
    fn server_drop_ins_win_and_match_is_ignored() {
        let server = setup(&fixture("laptop")).server.unwrap();
        assert_eq!(server.port.as_deref(), Some("2222"));
        assert_eq!(server.password_auth.as_deref(), Some("no"));
        assert_eq!(server.root_login, None);
        assert!(!server.unreadable);
    }

    #[test]
    fn no_home() {
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
