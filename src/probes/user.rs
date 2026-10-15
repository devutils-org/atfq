//! `user.identity`: who `atfq` runs as, its groups and its umask.

use std::collections::HashMap;

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "user.identity";

/// Groups that usually come with sudo rights, by distro family.
const ADMIN_GROUPS: &[&str] = &["sudo", "wheel", "admin"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identity {
    pub user: Option<String>,
    pub uid: u32,
    pub gid: u32,
    /// Primary group first, then the others; names when known.
    pub groups: Vec<String>,
    /// Octal, as the shell prints it: `0022`.
    pub umask: Option<String>,
    /// The first of sudo, wheel or admin the user belongs to.
    pub admin_group: Option<String>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let status = match ctx.read("/proc/self/status") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(NAME, e),
    };
    let field = |k: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(k).map(|v| v.trim().to_string()))
    };
    let first_id = |k: &str| {
        field(k).and_then(|v| {
            v.split_whitespace()
                .next()
                .and_then(|n| n.parse::<u32>().ok())
        })
    };
    let (Some(uid), Some(gid)) = (first_id("Uid:"), first_id("Gid:")) else {
        let mut r = ProbeResult::new(NAME, Status::Unavailable, None);
        r.notes
            .push("/proc/self/status: no Uid or Gid line".to_string());
        return r;
    };

    let users = names(ctx, "/etc/passwd");
    let group_names = names(ctx, "/etc/group");
    let mut ids = vec![gid];
    for g in field("Groups:").unwrap_or_default().split_whitespace() {
        if let Ok(g) = g.parse::<u32>() {
            if !ids.contains(&g) {
                ids.push(g);
            }
        }
    }
    let groups: Vec<String> = ids
        .iter()
        .map(|g| group_names.get(g).cloned().unwrap_or_else(|| g.to_string()))
        .collect();
    let admin_group = ADMIN_GROUPS
        .iter()
        .find(|a| groups.iter().any(|g| g == *a))
        .map(|a| a.to_string());

    let mut notes = Vec::new();
    let umask = field("Umask:");
    if umask.is_none() {
        notes.push("no Umask in /proc/self/status (kernel older than 4.7)".to_string());
    }
    let data = Identity {
        user: users.get(&uid).cloned(),
        uid,
        gid,
        groups,
        umask,
        admin_group,
    };
    let status = if notes.is_empty() {
        Status::Ok
    } else {
        Status::Partial
    };
    let mut r = ProbeResult::new(NAME, status, Some(ProbeData::User(data)));
    r.notes = notes;
    r
}

/// id → name from /etc/passwd or /etc/group (`name:x:id:…`). Users from
/// LDAP or systemd-homed are not in these files and stay as numbers.
pub(crate) fn names(ctx: &ProbeCtx, file: &str) -> HashMap<u32, String> {
    ctx.read(file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut f = l.split(':');
            let name = f.next()?;
            let id = f.nth(1)?.parse().ok()?;
            Some((id, name.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn reads_ids_groups_and_umask() {
        let r = probe(&fixture("laptop"));
        assert_eq!(r.status, Status::Ok);
        let Some(ProbeData::User(u)) = r.data else {
            panic!()
        };
        assert_eq!(u.user.as_deref(), Some("alice"));
        assert_eq!(u.groups, ["alice", "wheel", "docker", "4242"]);
        assert_eq!(u.umask.as_deref(), Some("0022"));
        assert_eq!(u.admin_group.as_deref(), Some("wheel"));
    }

    #[test]
    fn old_kernel_without_umask() {
        let r = probe(&fixture("old-kernel"));
        assert_eq!(r.status, Status::Partial);
        let Some(ProbeData::User(u)) = r.data else {
            panic!()
        };
        assert_eq!(u.user, None);
        assert_eq!(u.groups, ["100"]);
    }

    #[test]
    fn missing_status() {
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
