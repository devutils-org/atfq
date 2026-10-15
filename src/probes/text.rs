//! `text.locale`: the locale settings of this session and the system, and
//! whether text is read as UTF-8.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "text.locale";

/// Locale variables, in the order they take precedence for one category:
/// LC_ALL over LC_*, LC_* over LANG.
const VARS: &[&str] = &[
    "LC_ALL",
    "LC_CTYPE",
    "LC_COLLATE",
    "LC_TIME",
    "LC_NUMERIC",
    "LC_MESSAGES",
    "LANG",
    "LANGUAGE",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocaleInfo {
    /// Locale variables set in this process's environment.
    pub vars: Vec<(String, String)>,
    /// The locale that decides how bytes become characters (LC_CTYPE).
    pub ctype: String,
    pub utf8: bool,
    /// The system default, from /etc/locale.conf or /etc/default/locale.
    pub system: Option<String>,
    /// Locales compiled on this machine (/usr/lib/locale), and whether an
    /// archive holds more.
    pub installed: Vec<String>,
    pub archive: bool,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let raw = match ctx.read_bytes("/proc/self/environ") {
        Ok(r) => r,
        Err(e) => return ProbeResult::failed(NAME, e),
    };
    let env: Vec<(String, String)> = raw
        .split(|b| *b == 0)
        .filter_map(|kv| {
            let kv = String::from_utf8_lossy(kv);
            let (k, v) = kv.split_once('=')?;
            Some((k.to_string(), v.to_string()))
        })
        .collect();
    let vars: Vec<(String, String)> = VARS
        .iter()
        .filter_map(|n| env.iter().find(|(k, v)| k == n && !v.is_empty()).cloned())
        .collect();
    let get = |n: &str| vars.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    // Without any of them, programs run in the "C" (or "POSIX") locale.
    let ctype = get("LC_ALL")
        .or_else(|| get("LC_CTYPE"))
        .or_else(|| get("LANG"))
        .unwrap_or_else(|| "C".to_string());
    let lower = ctype.to_ascii_lowercase();
    let utf8 = lower.contains("utf-8") || lower.contains("utf8");
    let system = ["/etc/locale.conf", "/etc/default/locale"]
        .iter()
        .filter_map(|f| ctx.read(f).ok())
        .find_map(|t| {
            t.lines().find_map(|l| {
                l.trim()
                    .strip_prefix("LANG=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        });
    let entries = ctx.list("/usr/lib/locale").unwrap_or_default();
    let data = LocaleInfo {
        vars,
        ctype,
        utf8,
        system,
        archive: entries.iter().any(|e| e == "locale-archive"),
        installed: entries
            .into_iter()
            .filter(|e| e != "locale-archive")
            .collect(),
    };
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Locale(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn utf8_session() {
        let Some(ProbeData::Locale(l)) = probe(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(l.vars, [("LANG".to_string(), "en_US.UTF-8".to_string())]);
        assert_eq!(l.ctype, "en_US.UTF-8");
        assert!(l.utf8);
        assert_eq!(l.system.as_deref(), Some("fr_FR.UTF-8"));
        assert_eq!(l.installed, ["C.utf8"]);
        assert!(l.archive);
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
