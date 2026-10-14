//! systemd probes: `systemd.status` (overall state and failed units, from
//! systemctl) and `systemd.overrides` (what /etc changes, from the files).

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const STATUS: &str = "systemd.status";
pub(crate) const OVERRIDES: &str = "systemd.overrides";

const IS_RUNNING: &[&str] = &["systemctl", "is-system-running"];
const FAILED: &[&str] = &[
    "systemctl",
    "list-units",
    "--state=failed",
    "--no-legend",
    "--plain",
    "--no-pager",
];

/// Where packages install unit files.
const VENDOR_DIRS: &[&str] = &["/usr/lib/systemd/system", "/lib/systemd/system"];

const UNIT_SUFFIXES: &[&str] = &[
    ".service", ".socket", ".timer", ".mount", ".path", ".target", ".slice",
];

/// The lists of unit names keep at most this many entries.
const LIST_MAX: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SystemdStatus {
    /// `running`, `degraded`, `starting`, `maintenance`…
    pub state: String,
    pub failed: Vec<FailedUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailedUnit {
    pub unit: String,
    pub description: String,
}

pub(crate) fn status(ctx: &ProbeCtx) -> ProbeResult {
    let state = match ctx.run(IS_RUNNING) {
        Ok(out) => out.trim().to_string(),
        Err(e) => {
            let mut r = ProbeResult::failed(STATUS, e);
            r.notes =
                vec!["systemctl: not found or not answering (is this a systemd system?)".into()];
            return r;
        }
    };
    if state.is_empty() || state == "offline" {
        let mut r = ProbeResult::new(STATUS, Status::Unavailable, None);
        r.notes
            .push("systemd is not the init system here".to_string());
        return r;
    }
    let mut notes = Vec::new();
    let failed = match ctx.run(FAILED) {
        Ok(out) => out
            .lines()
            .filter_map(|l| {
                // unit load active sub description…
                let mut f = l.split_whitespace();
                let unit = f.next()?.to_string();
                let description = f.skip(3).collect::<Vec<_>>().join(" ");
                Some(FailedUnit { unit, description })
            })
            .collect(),
        Err(_) => {
            notes.push("systemctl list-units: no answer".to_string());
            Vec::new()
        }
    };
    let status = if notes.is_empty() {
        Status::Ok
    } else {
        Status::Partial
    };
    let mut r = ProbeResult::new(
        STATUS,
        status,
        Some(ProbeData::Systemd(SystemdStatus { state, failed })),
    );
    r.notes = notes;
    r
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Overrides {
    /// Units with drop-in files in /etc/systemd/system/UNIT.d/, all of them.
    pub dropins: Vec<DropIn>,
    /// Full unit files in /etc that replace a packaged one.
    pub replaced: Vec<String>,
    /// Unit files in /etc that no package provides: written locally.
    pub local: Vec<String>,
    /// Units masked (linked to /dev/null).
    pub masked: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DropIn {
    pub unit: String,
    pub files: Vec<String>,
}

pub(crate) fn overrides(ctx: &ProbeCtx) -> ProbeResult {
    const ETC: &str = "/etc/systemd/system";
    let entries = match ctx.list(ETC) {
        Ok(e) => e,
        Err(e) => return ProbeResult::failed(OVERRIDES, e),
    };
    let packaged = |unit: &str| {
        VENDOR_DIRS
            .iter()
            .any(|d| ctx.exists(&format!("{d}/{unit}")))
    };
    let mut o = Overrides {
        dropins: Vec::new(),
        replaced: Vec::new(),
        local: Vec::new(),
        masked: Vec::new(),
    };
    for name in entries {
        let path = format!("{ETC}/{name}");
        if let Some(unit) = name.strip_suffix(".d") {
            let files: Vec<String> = ctx
                .list(&path)
                .unwrap_or_default()
                .into_iter()
                .filter(|f| f.ends_with(".conf"))
                .collect();
            if !files.is_empty() {
                o.dropins.push(DropIn {
                    unit: unit.to_string(),
                    files,
                });
            }
            continue;
        }
        if !UNIT_SUFFIXES.iter().any(|s| name.ends_with(s)) {
            continue;
        }
        match ctx.read_link(&path) {
            Some(target) if target == "/dev/null" => o.masked.push(name),
            // Other links are aliases or enablement, not changes.
            Some(_) => {}
            None if packaged(&name) => o.replaced.push(name),
            None => o.local.push(name),
        }
    }
    for list in [&mut o.replaced, &mut o.local, &mut o.masked] {
        list.truncate(LIST_MAX);
    }
    ProbeResult::new(OVERRIDES, Status::Ok, Some(ProbeData::Overrides(o)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    fn degraded() -> ProbeCtx {
        fixture("laptop")
            .with_command(IS_RUNNING, "degraded\n")
            .with_command(
                FAILED,
                "nginx.service loaded failed failed A high performance web server\n\
                 backup.timer  loaded failed failed Nightly backup\n",
            )
    }

    #[test]
    fn degraded_with_failed_units() {
        let Some(ProbeData::Systemd(s)) = status(&degraded()).data else {
            panic!()
        };
        assert_eq!(s.state, "degraded");
        assert_eq!(s.failed.len(), 2);
        assert_eq!(s.failed[0].unit, "nginx.service");
        assert_eq!(s.failed[0].description, "A high performance web server");
    }

    #[test]
    fn no_systemctl() {
        let r = status(&fixture("laptop"));
        assert_eq!(r.status, Status::Unavailable);
        let r = status(&fixture("laptop").with_command(IS_RUNNING, "offline\n"));
        assert_eq!(r.status, Status::Unavailable);
    }

    #[test]
    fn finds_overrides() {
        let Some(ProbeData::Overrides(o)) = overrides(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(o.dropins.len(), 1);
        assert_eq!(o.dropins[0].unit, "nginx.service");
        assert_eq!(o.dropins[0].files, ["override.conf"]);
        assert_eq!(o.replaced, ["sshd.service"]);
        assert_eq!(o.local, ["backup.service", "backup.timer"]);
        assert_eq!(o.masked, ["bluetooth.service"]);
        assert_eq!(overrides(&fixture("empty")).status, Status::Unavailable);
    }
}
