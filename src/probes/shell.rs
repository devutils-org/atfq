//! Shell and binaries probes: `shell.env` (a few well-known variables),
//! `shell.startup` (which startup files exist) and `bin.path` (PATH,
//! library path and dynamic loaders).
//!
//! The environment can hold secrets, so only allowlisted names are read.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const ENV: &str = "shell.env";
pub(crate) const STARTUP: &str = "shell.startup";
pub(crate) const PATH: &str = "bin.path";

/// Variables shown by `shell.env`. Never all of them: tokens and passwords
/// often live in the environment.
const SHOWN: &[&str] = &[
    "SHELL",
    "HOME",
    "LANG",
    "LC_ALL",
    "EDITOR",
    "VISUAL",
    "PAGER",
    "TERM",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
];

/// Dynamic loaders, by the programs they can start.
const LOADERS: &[(&str, &str)] = &[
    ("/lib64/ld-linux-x86-64.so.2", "x86-64, glibc"),
    ("/lib/ld-linux-aarch64.so.1", "ARM 64-bit, glibc"),
    ("/lib/ld-linux-armhf.so.3", "ARM 32-bit, glibc"),
    ("/lib/ld-linux.so.2", "x86 32-bit, glibc"),
    ("/lib/ld-musl-x86_64.so.1", "x86-64, musl"),
    ("/lib/ld-musl-aarch64.so.1", "ARM 64-bit, musl"),
];

/// `NAME=value` pairs of this process, from /proc/self/environ.
fn environ(ctx: &ProbeCtx) -> Result<Vec<(String, String)>, super::ReadError> {
    let raw = ctx.read_bytes("/proc/self/environ")?;
    Ok(raw
        .split(|b| *b == 0)
        .filter_map(|kv| {
            let kv = String::from_utf8_lossy(kv);
            let (k, v) = kv.split_once('=')?;
            Some((k.to_string(), v.to_string()))
        })
        .collect())
}

/// Our uid's login shell and home, from /proc/self/status and /etc/passwd.
pub(crate) fn account(ctx: &ProbeCtx) -> (Option<String>, Option<String>) {
    let uid = ctx.read("/proc/self/status").ok().and_then(|s| {
        s.lines()
            .find_map(|l| l.strip_prefix("Uid:"))
            .and_then(|v| v.split_whitespace().next()?.parse::<u32>().ok())
    });
    let Some(uid) = uid else { return (None, None) };
    let passwd = ctx.read("/etc/passwd").unwrap_or_default();
    passwd
        .lines()
        .find_map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            (f.len() >= 7 && f[2].parse() == Ok(uid))
                .then(|| (Some(f[6].to_string()), Some(f[5].to_string())))
        })
        .unwrap_or((None, None))
}

// ----- shell.env -----------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShellEnv {
    /// Allowlisted variables that are set, in a fixed order.
    pub vars: Vec<(String, String)>,
    /// How many variables are set in all.
    pub total: u32,
    /// The login shell in /etc/passwd.
    pub login_shell: Option<String>,
}

pub(crate) fn env(ctx: &ProbeCtx) -> ProbeResult {
    let all = match environ(ctx) {
        Ok(a) => a,
        Err(e) => return ProbeResult::failed(ENV, e),
    };
    let vars = SHOWN
        .iter()
        .filter_map(|name| all.iter().find(|(k, _)| k == name).cloned())
        .collect();
    let data = ShellEnv {
        vars,
        total: all.len() as u32,
        login_shell: account(ctx).0,
    };
    ProbeResult::new(ENV, Status::Ok, Some(ProbeData::Env(data)))
}

// ----- shell.startup -------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Startup {
    pub login_shell: Option<String>,
    /// Startup files that exist, as `~/.bashrc` or `/etc/profile`.
    pub files: Vec<String>,
    /// bash login shells read only the first of these that exists.
    pub bash_login_file: Option<String>,
    /// Profile files bash ignores because an earlier one exists.
    pub ignored_by_bash: Vec<String>,
}

const BASH_LOGIN: &[&str] = &["~/.bash_profile", "~/.bash_login", "~/.profile"];
const USER_FILES: &[&str] = &[
    "~/.bash_profile",
    "~/.bash_login",
    "~/.profile",
    "~/.bashrc",
    "~/.zshenv",
    "~/.zprofile",
    "~/.zshrc",
    "~/.config/fish/config.fish",
];
const SYSTEM_FILES: &[&str] = &[
    "/etc/profile",
    "/etc/bash.bashrc",
    "/etc/bashrc",
    "/etc/zsh/zshrc",
    "/etc/zshrc",
];

pub(crate) fn startup(ctx: &ProbeCtx) -> ProbeResult {
    let (login_shell, home) = account(ctx);
    let Some(home) = home else {
        let mut r = ProbeResult::new(STARTUP, Status::Unavailable, None);
        r.notes
            .push("home folder unknown: no entry in /etc/passwd".to_string());
        return r;
    };
    let exists = |f: &str| match f.strip_prefix("~/") {
        Some(rel) => ctx.exists(&format!("{home}/{rel}")),
        None => ctx.exists(f),
    };
    let mut files: Vec<String> = USER_FILES
        .iter()
        .chain(SYSTEM_FILES)
        .filter(|f| exists(f))
        .map(|f| f.to_string())
        .collect();
    let profile_d = ctx.list("/etc/profile.d").map(|l| l.len()).unwrap_or(0);
    if profile_d > 0 {
        files.push(format!("/etc/profile.d/ ({profile_d} files)"));
    }
    let present: Vec<&str> = BASH_LOGIN.iter().copied().filter(|f| exists(f)).collect();
    let data = Startup {
        login_shell,
        files,
        bash_login_file: present.first().map(|f| f.to_string()),
        ignored_by_bash: present.iter().skip(1).map(|f| f.to_string()).collect(),
    };
    ProbeResult::new(STARTUP, Status::Ok, Some(ProbeData::Startup(data)))
}

// ----- bin.path ------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BinPath {
    pub entries: Vec<PathEntry>,
    /// Set means the loader searches these folders first.
    pub ld_library_path: Option<String>,
    /// Set means a library is forced into every program.
    pub ld_preload: Option<String>,
    /// Programs this system can start: one per dynamic loader found.
    pub loaders: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathEntry {
    pub dir: String,
    pub exists: bool,
    /// Already listed earlier in PATH.
    pub duplicate: bool,
    /// Empty or `.`: the current folder, wherever you are.
    pub current_dir: bool,
}

pub(crate) fn path(ctx: &ProbeCtx) -> ProbeResult {
    let all = match environ(ctx) {
        Ok(a) => a,
        Err(e) => return ProbeResult::failed(PATH, e),
    };
    let get = |k: &str| all.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let mut seen: Vec<String> = Vec::new();
    let entries = get("PATH")
        .unwrap_or_default()
        .split(':')
        .map(|dir| {
            let current_dir = dir.is_empty() || dir == ".";
            let e = PathEntry {
                exists: current_dir || ctx.exists(dir),
                duplicate: seen.iter().any(|s| s == dir),
                current_dir,
                dir: dir.to_string(),
            };
            seen.push(dir.to_string());
            e
        })
        .collect();
    let data = BinPath {
        entries,
        ld_library_path: get("LD_LIBRARY_PATH").filter(|v| !v.is_empty()),
        ld_preload: get("LD_PRELOAD").filter(|v| !v.is_empty()),
        loaders: LOADERS
            .iter()
            .filter(|(p, _)| ctx.exists(p))
            .map(|(_, what)| *what)
            .collect(),
    };
    ProbeResult::new(PATH, Status::Ok, Some(ProbeData::BinPath(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn shows_only_allowlisted_variables() {
        let Some(ProbeData::Env(e)) = env(&fixture("laptop")).data else {
            panic!()
        };
        let names: Vec<&str> = e.vars.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(names, ["SHELL", "HOME", "LANG", "EDITOR", "TERM"]);
        assert_eq!(e.total, 8);
        assert_eq!(e.login_shell.as_deref(), Some("/bin/zsh"));
        // The token in the fixture's environment never comes out.
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("ghp_"), "{json}");
    }

    #[test]
    fn startup_files_and_bash_precedence() {
        let Some(ProbeData::Startup(s)) = startup(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(
            s.files,
            [
                "~/.bash_profile",
                "~/.profile",
                "~/.bashrc",
                "~/.zshrc",
                "/etc/profile",
                "/etc/profile.d/ (2 files)"
            ]
        );
        assert_eq!(s.bash_login_file.as_deref(), Some("~/.bash_profile"));
        assert_eq!(s.ignored_by_bash, ["~/.profile"]);
    }

    #[test]
    fn path_entries_and_loaders() {
        let Some(ProbeData::BinPath(p)) = path(&fixture("laptop")).data else {
            panic!()
        };
        let got: Vec<(&str, bool, bool, bool)> = p
            .entries
            .iter()
            .map(|e| (e.dir.as_str(), e.exists, e.duplicate, e.current_dir))
            .collect();
        assert_eq!(
            got,
            [
                ("/home/alice/.local/bin", true, false, false),
                ("/usr/local/bin", false, false, false),
                ("/usr/bin", true, false, false),
                ("/usr/bin", true, true, false),
                (".", true, false, true),
            ]
        );
        assert_eq!(p.loaders, ["x86-64, glibc"]);
        assert_eq!(p.ld_library_path.as_deref(), Some("/opt/oracle/lib"));
        assert_eq!(p.ld_preload, None);
    }

    #[test]
    fn missing_environ() {
        assert_eq!(env(&fixture("empty")).status, Status::Unavailable);
        assert_eq!(path(&fixture("empty")).status, Status::Unavailable);
        assert_eq!(startup(&fixture("empty")).status, Status::Unavailable);
    }
}
