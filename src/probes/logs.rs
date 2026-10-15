//! `log.journal`: where this system keeps its logs, how much room they
//! take, and what rotates them.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "log.journal";

/// Largest files under /var/log listed.
const BIGGEST: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogInfo {
    /// `persistent` (/var/log/journal), `volatile` (/run/log/journal, lost
    /// at reboot), or `none`.
    pub storage: String,
    /// `Storage=` and `SystemMaxUse=` from journald.conf and its drop-ins.
    pub storage_setting: Option<String>,
    pub max_use: Option<String>,
    /// Journal files and their total size, where they could be listed.
    pub journal_files: u32,
    pub journal_bytes: u64,
    /// A classic syslog daemon is installed: `rsyslog`, `syslog-ng`.
    pub syslog: Option<String>,
    /// Rules in /etc/logrotate.d.
    pub logrotate_rules: Option<u32>,
    /// Largest files directly under /var/log or one folder down.
    pub biggest: Vec<(String, u64)>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    if !ctx.exists("/var/log") && !ctx.exists("/run/log/journal") {
        return ProbeResult::failed(NAME, super::ReadError::Missing("/var/log".into()));
    }
    let mut files = 0;
    let mut bytes = 0;
    let mut storage = "none";
    for (dir, kind) in [
        ("/var/log/journal", "persistent"),
        ("/run/log/journal", "volatile"),
    ] {
        let mut found = false;
        for machine in ctx.list(dir).unwrap_or_default() {
            for f in ctx.list(&format!("{dir}/{machine}")).unwrap_or_default() {
                if f.ends_with(".journal") || f.ends_with(".journal~") {
                    files += 1;
                    found = true;
                    bytes += ctx.size(&format!("{dir}/{machine}/{f}")).unwrap_or(0);
                }
            }
        }
        // journald's default (Storage=auto) writes to disk when the folder
        // exists, even before the first file.
        if storage == "none" && (found || (kind == "persistent" && ctx.exists(dir))) {
            storage = kind;
        }
    }
    // Later files win: journald.conf, then drop-ins in name order.
    let mut confs = vec!["/etc/systemd/journald.conf".to_string()];
    let mut dropins = ctx.list("/etc/systemd/journald.conf.d").unwrap_or_default();
    dropins.retain(|f| f.ends_with(".conf"));
    confs.extend(
        dropins
            .into_iter()
            .map(|f| format!("/etc/systemd/journald.conf.d/{f}")),
    );
    let mut storage_setting = None;
    let mut max_use = None;
    for conf in confs {
        for line in ctx.read(&conf).unwrap_or_default().lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("Storage=") {
                storage_setting = Some(v.to_string());
            } else if let Some(v) = line.strip_prefix("SystemMaxUse=") {
                max_use = Some(v.to_string());
            }
        }
    }
    if storage_setting.as_deref() == Some("volatile") {
        storage = "volatile";
    }
    let syslog = [
        ("/etc/rsyslog.conf", "rsyslog"),
        ("/etc/syslog-ng/syslog-ng.conf", "syslog-ng"),
    ]
    .iter()
    .find(|(f, _)| ctx.exists(f))
    .map(|(_, n)| n.to_string());
    let mut sizes: Vec<(String, u64)> = Vec::new();
    for name in ctx.list("/var/log").unwrap_or_default() {
        if name == "journal" {
            continue;
        }
        let path = format!("/var/log/{name}");
        match ctx.list(&path) {
            Ok(children) => {
                for c in children {
                    let p = format!("{path}/{c}");
                    if let Some(s) = ctx.size(&p).filter(|_| ctx.list(&p).is_err()) {
                        sizes.push((p, s));
                    }
                }
            }
            Err(_) => {
                if let Some(s) = ctx.size(&path) {
                    sizes.push((path, s));
                }
            }
        }
    }
    sizes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    sizes.retain(|(_, s)| *s > 0);
    sizes.truncate(BIGGEST);
    let data = LogInfo {
        storage: storage.to_string(),
        storage_setting,
        max_use,
        journal_files: files,
        journal_bytes: bytes,
        syslog,
        logrotate_rules: ctx.list("/etc/logrotate.d").ok().map(|l| l.len() as u32),
        biggest: sizes,
    };
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Logs(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn laptop_logs() {
        let Some(ProbeData::Logs(l)) = probe(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(l.storage, "persistent");
        assert_eq!(l.max_use.as_deref(), Some("500M"));
        assert_eq!(l.storage_setting, None);
        assert_eq!(l.journal_files, 2);
        assert_eq!(l.journal_bytes, 300);
        assert_eq!(l.syslog.as_deref(), Some("rsyslog"));
        assert_eq!(l.logrotate_rules, Some(2));
        let names: Vec<&str> = l.biggest.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            names,
            [
                "/var/log/syslog",
                "/var/log/nginx/access.log",
                "/var/log/auth.log"
            ]
        );
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
