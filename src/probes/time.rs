//! `time.clock`: time zone, hardware clock mode, clock source, who keeps the
//! clock in sync, and when the system booted.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "time.clock";

/// NTP daemons, as their process names appear in /proc (15 characters at
/// most).
const DAEMONS: &[(&str, &str)] = &[
    ("chronyd", "chronyd"),
    ("systemd-timesyn", "systemd-timesyncd"),
    ("ntpd", "ntpd"),
    ("openntpd", "openntpd"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimeInfo {
    /// `Europe/Paris`, from /etc/localtime or /etc/timezone.
    pub zone: Option<String>,
    /// The hardware clock keeps local time instead of UTC (/etc/adjtime).
    pub rtc_local: Option<bool>,
    /// What the kernel reads time from: `tsc`, `arch_sys_counter`, `kvm-clock`…
    pub clocksource: Option<String>,
    pub clocksources: Vec<String>,
    /// NTP daemon running, if any.
    pub daemon: Option<String>,
    /// systemd-timesyncd's flag file: synchronised at least once.
    pub synced: Option<bool>,
    /// Boot time, in seconds since 1970 (UTC).
    pub boot: Option<u64>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let zone = ctx
        .read_link("/etc/localtime")
        .and_then(|l| l.split_once("zoneinfo/").map(|(_, z)| z.to_string()))
        .or_else(|| ctx.read_trimmed("/etc/timezone").filter(|z| !z.is_empty()));
    let rtc_local = ctx
        .read("/etc/adjtime")
        .ok()
        .and_then(|a| a.lines().nth(2).map(|l| l.trim() == "LOCAL"));
    let cs = "/sys/devices/system/clocksource/clocksource0";
    let daemon = ctx
        .list("/proc")
        .unwrap_or_default()
        .iter()
        .filter(|n| n.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|pid| ctx.read_trimmed(&format!("/proc/{pid}/comm")))
        .find_map(|comm| {
            DAEMONS
                .iter()
                .find(|(c, _)| *c == comm)
                .map(|(_, name)| name.to_string())
        });
    let synced = (daemon.as_deref() == Some("systemd-timesyncd"))
        .then(|| ctx.exists("/run/systemd/timesync/synchronized"));
    let boot = ctx.read("/proc/stat").ok().and_then(|s| {
        s.lines()
            .find_map(|l| l.strip_prefix("btime "))
            .and_then(|v| v.trim().parse().ok())
    });
    let data = TimeInfo {
        zone,
        rtc_local,
        clocksource: ctx.read_trimmed(&format!("{cs}/current_clocksource")),
        clocksources: ctx
            .read_trimmed(&format!("{cs}/available_clocksource"))
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect(),
        daemon,
        synced,
        boot,
    };
    let empty = data.zone.is_none() && data.clocksource.is_none() && data.boot.is_none();
    if empty {
        let mut r = ProbeResult::new(NAME, Status::Unavailable, None);
        r.notes
            .push("no time zone, clock source or boot time found".to_string());
        return r;
    }
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Time(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn laptop_clock() {
        let Some(ProbeData::Time(t)) = probe(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(t.zone.as_deref(), Some("Europe/Paris"));
        assert_eq!(t.rtc_local, Some(false));
        assert_eq!(t.clocksource.as_deref(), Some("tsc"));
        assert_eq!(t.clocksources, ["tsc", "hpet", "acpi_pm"]);
        assert_eq!(t.daemon.as_deref(), Some("systemd-timesyncd"));
        assert_eq!(t.synced, Some(true));
        assert_eq!(t.boot, Some(1_791_700_000));
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
