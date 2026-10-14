//! `pkg.manager`: which package managers this system uses, and how many
//! packages each one holds, Snap and Flatpak included.

use serde::Serialize;

use super::shell::account;
use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "pkg.manager";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Packages {
    /// `dpkg`, `rpm`, `pacman`, `apk`, with their package count when known.
    pub native: Vec<(String, Option<u32>)>,
    /// Installed snaps (revisions count separately), if snapd is here.
    pub snaps: Option<u32>,
    /// Flatpak apps, system-wide and in your home, if Flatpak is here.
    pub flatpaks: Option<u32>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let mut native = Vec::new();
    if let Ok(status) = ctx.read("/var/lib/dpkg/status") {
        let n = status
            .lines()
            .filter(|l| l.trim() == "Status: install ok installed")
            .count();
        native.push(("dpkg".to_string(), Some(n as u32)));
    }
    if ctx.exists("/var/lib/rpm") || ctx.exists("/usr/lib/sysimage/rpm") {
        // The rpm database is binary: counting needs `rpm -qa`.
        native.push(("rpm".to_string(), None));
    }
    if let Ok(dirs) = ctx.list("/var/lib/pacman/local") {
        let n = dirs.iter().filter(|d| *d != "ALPM_DB_VERSION").count();
        native.push(("pacman".to_string(), Some(n as u32)));
    }
    if let Ok(db) = ctx.read("/lib/apk/db/installed") {
        let n = db.lines().filter(|l| l.starts_with("P:")).count();
        native.push(("apk".to_string(), Some(n as u32)));
    }
    let snaps = ctx.list("/var/lib/snapd/snaps").ok().map(|f| {
        let mut names: Vec<&str> = f
            .iter()
            .filter(|s| s.ends_with(".snap"))
            .filter_map(|s| s.rsplit_once('_').map(|(name, _)| name))
            .collect();
        names.dedup();
        names.len() as u32
    });
    let mut flatpak_dirs = vec!["/var/lib/flatpak/app".to_string()];
    if let (_, Some(home)) = account(ctx) {
        flatpak_dirs.push(format!("{home}/.local/share/flatpak/app"));
    }
    let lists: Vec<Vec<String>> = flatpak_dirs
        .iter()
        .filter_map(|d| ctx.list(d).ok())
        .collect();
    let flatpaks = (!lists.is_empty()).then(|| lists.iter().map(|l| l.len() as u32).sum());

    if native.is_empty() && snaps.is_none() && flatpaks.is_none() {
        let mut r = ProbeResult::new(NAME, Status::Unavailable, None);
        r.notes.push("no known package database found".to_string());
        return r;
    }
    ProbeResult::new(
        NAME,
        Status::Ok,
        Some(ProbeData::Packages(Packages {
            native,
            snaps,
            flatpaks,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn debian_like_with_snap_and_flatpak() {
        let Some(ProbeData::Packages(p)) = probe(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(p.native, [("dpkg".to_string(), Some(2))]);
        assert_eq!(p.snaps, Some(2));
        assert_eq!(p.flatpaks, Some(3));
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
