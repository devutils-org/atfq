//! `security.lsm`: which Linux security modules run, and the state of
//! SELinux or AppArmor.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const LSM: &str = "security.lsm";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Lsm {
    /// Active modules, in order: `capability,landlock,yama,apparmor`.
    pub modules: Vec<String>,
    /// `Some(true)` enforcing, `Some(false)` permissive; `None` without SELinux.
    pub selinux_enforcing: Option<bool>,
    /// `SELINUX=` in /etc/selinux/config: what the next boot will use.
    pub selinux_config: Option<String>,
    pub apparmor: bool,
    /// Profiles by mode (`enforce`, `complain`); readable by root only.
    pub apparmor_profiles: Vec<(String, u32)>,
}

pub(crate) fn lsm(ctx: &ProbeCtx) -> ProbeResult {
    let modules: Vec<String> = ctx
        .read_trimmed("/sys/kernel/security/lsm")
        .map(|l| {
            l.split(',')
                .filter(|m| !m.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let selinux_enforcing = ctx
        .read_trimmed("/sys/fs/selinux/enforce")
        .map(|v| v == "1");
    let selinux_config = ctx.read("/etc/selinux/config").ok().and_then(|t| {
        t.lines()
            .find_map(|l| l.trim().strip_prefix("SELINUX="))
            .map(|v| v.trim().to_string())
    });
    let apparmor = ctx
        .read_trimmed("/sys/module/apparmor/parameters/enabled")
        .is_some_and(|v| v == "Y")
        || modules.iter().any(|m| m == "apparmor");
    let mut notes = Vec::new();
    let mut apparmor_profiles: Vec<(String, u32)> = Vec::new();
    if apparmor {
        match ctx.read("/sys/kernel/security/apparmor/profiles") {
            Ok(t) => {
                for line in t.lines() {
                    // `name (enforce)`
                    let Some(mode) = line.rsplit_once('(').map(|(_, m)| m.trim_end_matches(')'))
                    else {
                        continue;
                    };
                    match apparmor_profiles.iter_mut().find(|(m, _)| m == mode) {
                        Some((_, n)) => *n += 1,
                        None => apparmor_profiles.push((mode.to_string(), 1)),
                    }
                }
            }
            Err(_) => notes.push("AppArmor profiles: reading them needs root".to_string()),
        }
    }
    if modules.is_empty() && selinux_enforcing.is_none() && !apparmor {
        let mut r = ProbeResult::new(LSM, Status::Unavailable, None);
        r.notes
            .push("/sys/kernel/security/lsm: can't read it".to_string());
        return r;
    }
    let data = Lsm {
        modules,
        selinux_enforcing,
        selinux_config,
        apparmor,
        apparmor_profiles,
    };
    let mut r = ProbeResult::new(LSM, Status::Ok, Some(ProbeData::Lsm(data)));
    r.notes = notes;
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn apparmor_laptop() {
        let r = lsm(&fixture("laptop"));
        let Some(ProbeData::Lsm(l)) = r.data else {
            panic!()
        };
        assert_eq!(l.modules, ["capability", "landlock", "yama", "apparmor"]);
        assert!(l.apparmor);
        assert_eq!(l.selinux_enforcing, None);
        assert_eq!(
            l.apparmor_profiles,
            [("enforce".to_string(), 2), ("complain".to_string(), 1)]
        );
    }

    #[test]
    fn selinux_machine() {
        let Some(ProbeData::Lsm(l)) = lsm(&fixture("old-kernel")).data else {
            panic!()
        };
        assert_eq!(l.selinux_enforcing, Some(true));
        assert_eq!(l.selinux_config.as_deref(), Some("enforcing"));
        assert!(!l.apparmor);
        assert_eq!(lsm(&fixture("empty")).status, Status::Unavailable);
    }
}
