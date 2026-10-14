//! Network probes: `net.interfaces`, `net.listening` and `net.dns`.

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};

use serde::Serialize;

use super::user::names;
use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const INTERFACES: &str = "net.interfaces";
pub(crate) const LISTENING: &str = "net.listening";
pub(crate) const DNS: &str = "net.dns";
pub(crate) const FIREWALL: &str = "net.firewall";

const FIREWALL_UNITS: &[&str] = &[
    "systemctl",
    "is-active",
    "firewalld.service",
    "ufw.service",
    "nftables.service",
];

/// The listening list keeps at most this many sockets.
const LISTEN_MAX: usize = 30;

/// ARPHRD_LOOPBACK in /sys/class/net/*/type.
const LOOPBACK: &str = "772";

// ----- net.interfaces ------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetInterfaces {
    pub interfaces: Vec<Interface>,
    pub default_route: Option<Route>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Interface {
    pub name: String,
    /// `up`, `down`, `unknown` (loopback and some virtual links say this)…
    pub state: String,
    /// `loopback`, `wireless` or `virtual`; `None` for a plain wired card.
    pub kind: Option<&'static str>,
    pub mac: Option<String>,
    pub mtu: Option<u32>,
    /// Link speed in Mbit/s, when the driver knows it.
    pub speed_mbps: Option<u32>,
    /// `192.168.1.20/24`, `fe80::1/64`.
    pub addrs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Route {
    pub gateway: Ipv4Addr,
    pub iface: String,
}

pub(crate) fn interfaces(ctx: &ProbeCtx) -> ProbeResult {
    let names = match ctx.list("/sys/class/net") {
        Ok(n) => n,
        Err(e) => return ProbeResult::failed(INTERFACES, e),
    };
    let mut notes = Vec::new();
    let addrs = ctx.addrs();
    if addrs.is_none() {
        notes.push("IP addresses: can't list them".to_string());
    }
    let addrs = addrs.unwrap_or_default();

    let interfaces = names
        .into_iter()
        .map(|name| {
            let base = format!("/sys/class/net/{name}");
            let file = |f: &str| ctx.read_trimmed(&format!("{base}/{f}"));
            let kind = if file("type").as_deref() == Some(LOOPBACK) {
                Some("loopback")
            } else if ctx.exists(&format!("{base}/wireless")) {
                Some("wireless")
            } else if ctx
                .read_link(&base)
                .is_some_and(|l| l.contains("/virtual/"))
            {
                Some("virtual")
            } else {
                None
            };
            Interface {
                state: file("operstate").unwrap_or_else(|| "unknown".to_string()),
                kind,
                mac: file("address").filter(|m| !m.is_empty() && m != "00:00:00:00:00:00"),
                mtu: file("mtu").and_then(|m| m.parse().ok()),
                // Reading `speed` fails (EINVAL) or gives -1 when the link is down.
                speed_mbps: file("speed")
                    .and_then(|s| s.parse::<i64>().ok())
                    .and_then(|s| u32::try_from(s).ok().filter(|&s| s > 0)),
                addrs: addrs
                    .iter()
                    .filter(|a| a.iface == name)
                    .map(|a| format!("{}/{}", a.addr, a.prefix))
                    .collect(),
                name,
            }
        })
        .collect();

    let data = NetInterfaces {
        interfaces,
        default_route: ctx
            .read("/proc/net/route")
            .ok()
            .and_then(|t| default_route(&t)),
    };
    let status = if notes.is_empty() {
        Status::Ok
    } else {
        Status::Partial
    };
    let mut r = ProbeResult::new(INTERFACES, status, Some(ProbeData::Net(data)));
    r.notes = notes;
    r
}

/// The 0.0.0.0/0 line of /proc/net/route with the lowest metric.
fn default_route(table: &str) -> Option<Route> {
    table
        .lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let (iface, dest, gw, metric) = (f.first()?, f.get(1)?, f.get(2)?, f.get(6)?);
            (*dest == "00000000").then_some(())?;
            Some((
                metric.parse::<u32>().ok()?,
                iface.to_string(),
                hex_ipv4(gw)?,
            ))
        })
        .min_by_key(|(metric, ..)| *metric)
        .map(|(_, iface, gateway)| Route { gateway, iface })
}

/// The kernel prints IPv4 addresses as a hex number in host byte order:
/// the bytes in memory are the address in network order.
fn hex_ipv4(hex: &str) -> Option<Ipv4Addr> {
    let n = u32::from_str_radix(hex, 16).ok()?;
    Some(Ipv4Addr::from(n.to_ne_bytes()))
}

/// IPv6: four such 32-bit words.
fn hex_ipv6(hex: &str) -> Option<Ipv6Addr> {
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for i in 0..4 {
        let word = u32::from_str_radix(hex.get(i * 8..i * 8 + 8)?, 16).ok()?;
        bytes[i * 4..i * 4 + 4].copy_from_slice(&word.to_ne_bytes());
    }
    Some(Ipv6Addr::from(bytes))
}

// ----- net.listening -------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listening {
    /// Sorted by protocol, then port; at most 30.
    pub sockets: Vec<Listener>,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listener {
    /// `tcp` or `udp`.
    pub proto: &'static str,
    pub addr: String,
    pub port: u16,
    /// `all` (any interface), `local` (this machine only) or `address`.
    pub reach: &'static str,
    pub user: Option<String>,
    pub process: Option<String>,
    pub pid: Option<u32>,
}

/// File, protocol, state that means "waiting for others".
const SOCKET_TABLES: &[(&str, &str, &str)] = &[
    ("/proc/net/tcp", "tcp", "0A"),
    ("/proc/net/tcp6", "tcp", "0A"),
    ("/proc/net/udp", "udp", "07"),
    ("/proc/net/udp6", "udp", "07"),
];

pub(crate) fn listening(ctx: &ProbeCtx) -> ProbeResult {
    let mut found = Vec::new();
    let mut read_any = false;
    for (file, proto, state) in SOCKET_TABLES {
        let Ok(text) = ctx.read(file) else { continue };
        read_any = true;
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.get(3) != Some(state) {
                continue;
            }
            let Some((addr, port)) = f.get(1).and_then(|l| l.split_once(':')) else {
                continue;
            };
            let Ok(port) = u16::from_str_radix(port, 16) else {
                continue;
            };
            let addr = if addr.len() == 8 {
                hex_ipv4(addr).map(|a| a.to_string())
            } else {
                hex_ipv6(addr).map(|a| format!("[{a}]"))
            };
            let Some(addr) = addr else { continue };
            let uid: Option<u32> = f.get(7).and_then(|u| u.parse().ok());
            let inode = f.get(9).map(|i| i.to_string()).unwrap_or_default();
            found.push((*proto, addr, port, uid, inode));
        }
    }
    if !read_any {
        let mut r = ProbeResult::new(LISTENING, Status::Unavailable, None);
        r.notes.push("/proc/net/tcp: not found".to_string());
        return r;
    }

    let owners = socket_owners(ctx);
    let users = names(ctx, "/etc/passwd");
    let mut sockets: Vec<Listener> = found
        .into_iter()
        .map(|(proto, addr, port, uid, inode)| {
            let owner = owners.get(&inode);
            Listener {
                proto,
                reach: reach(&addr),
                port,
                user: uid.map(|u| users.get(&u).cloned().unwrap_or_else(|| u.to_string())),
                process: owner.map(|(_, c)| c.clone()),
                pid: owner.map(|(p, _)| *p),
                addr,
            }
        })
        .collect();
    sockets.sort_by(|a, b| (a.proto, a.port, &a.addr).cmp(&(b.proto, b.port, &b.addr)));
    sockets.dedup();
    let total = sockets.len() as u32;
    let unknown = sockets.iter().any(|s| s.process.is_none());
    sockets.truncate(LISTEN_MAX);

    let mut r = ProbeResult::new(
        LISTENING,
        Status::Ok,
        Some(ProbeData::Listening(Listening { sockets, total })),
    );
    if unknown {
        r.notes.push(
            "the process of a socket owned by another user needs root: sudo ss -tulpn".to_string(),
        );
    }
    r
}

fn reach(addr: &str) -> &'static str {
    match addr {
        "0.0.0.0" | "[::]" => "all",
        a if a.starts_with("127.") || a == "[::1]" => "local",
        _ => "address",
    }
}

/// Socket inode → (pid, name), from the /proc/PID/fd links we may read:
/// our own processes, or all of them as root.
fn socket_owners(ctx: &ProbeCtx) -> HashMap<String, (u32, String)> {
    let mut owners = HashMap::new();
    for pid in ctx.list("/proc").unwrap_or_default() {
        let Ok(pid_n) = pid.parse::<u32>() else {
            continue;
        };
        let Ok(fds) = ctx.list(&format!("/proc/{pid}/fd")) else {
            continue;
        };
        let mut comm = None;
        for fd in fds {
            let Some(link) = ctx.read_link(&format!("/proc/{pid}/fd/{fd}")) else {
                continue;
            };
            if let Some(inode) = link
                .strip_prefix("socket:[")
                .and_then(|l| l.strip_suffix(']'))
            {
                let name = comm
                    .get_or_insert_with(|| {
                        ctx.read_trimmed(&format!("/proc/{pid}/comm"))
                            .unwrap_or_default()
                    })
                    .clone();
                owners.entry(inode.to_string()).or_insert((pid_n, name));
            }
        }
    }
    owners
}

// ----- net.dns -------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DnsConfig {
    pub nameservers: Vec<String>,
    pub search: Vec<String>,
    /// A local resolver the nameserver points to, such as `systemd-resolved`.
    pub local_resolver: Option<&'static str>,
    /// Sources for host names, in order, from /etc/nsswitch.conf.
    pub hosts_order: Vec<String>,
}

pub(crate) fn dns(ctx: &ProbeCtx) -> ProbeResult {
    let resolv = ctx.read("/etc/resolv.conf");
    let nsswitch = ctx.read("/etc/nsswitch.conf");
    if let (Err(e), Err(_)) = (&resolv, &nsswitch) {
        return ProbeResult::failed(DNS, e.clone());
    }
    let mut notes = Vec::new();
    let resolv = resolv.unwrap_or_else(|_| {
        notes.push("/etc/resolv.conf: can't read it".to_string());
        String::new()
    });
    let values = |key: &str| -> Vec<String> {
        resolv
            .lines()
            .filter_map(|l| l.trim().strip_prefix(key))
            .flat_map(|rest| rest.split_whitespace().map(str::to_string))
            .collect()
    };
    let nameservers = values("nameserver");
    let mut search = values("search");
    search.retain(|s| s != ".");
    let local_resolver = nameservers.iter().find_map(|n| match n.as_str() {
        "127.0.0.53" | "127.0.0.54" => Some("systemd-resolved"),
        _ => None,
    });
    let hosts_order = nsswitch
        .unwrap_or_default()
        .lines()
        .find_map(|l| l.trim().strip_prefix("hosts:").map(str::to_string))
        .map(|rest| {
            rest.split_whitespace()
                .filter(|w| !w.starts_with('['))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let data = DnsConfig {
        nameservers,
        search,
        local_resolver,
        hosts_order,
    };
    let status = if notes.is_empty() {
        Status::Ok
    } else {
        Status::Partial
    };
    let mut r = ProbeResult::new(DNS, status, Some(ProbeData::Dns(data)));
    r.notes = notes;
    r
}

// ----- net.firewall ----------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Firewall {
    /// firewalld, ufw and nftables services, and their state.
    pub services: Vec<(String, String)>,
    /// `ENABLED=` in /etc/ufw/ufw.conf, when ufw is installed.
    pub ufw_enabled: Option<bool>,
}

pub(crate) fn firewall(ctx: &ProbeCtx) -> ProbeResult {
    let mut notes = Vec::new();
    let services: Vec<(String, String)> = match ctx.run(FIREWALL_UNITS) {
        Ok(out) => FIREWALL_UNITS[2..]
            .iter()
            .zip(out.lines())
            .map(|(unit, state)| {
                (
                    unit.trim_end_matches(".service").to_string(),
                    state.trim().to_string(),
                )
            })
            .collect(),
        Err(_) => {
            notes.push("systemctl: no answer, service states unknown".to_string());
            Vec::new()
        }
    };
    let ufw_enabled = ctx.read("/etc/ufw/ufw.conf").ok().map(|t| {
        t.lines()
            .any(|l| l.trim().eq_ignore_ascii_case("ENABLED=yes"))
    });
    if services.is_empty() && ufw_enabled.is_none() {
        let mut r = ProbeResult::new(FIREWALL, Status::Unavailable, None);
        r.notes = notes;
        return r;
    }
    let status = if notes.is_empty() {
        Status::Ok
    } else {
        Status::Partial
    };
    let mut r = ProbeResult::new(
        FIREWALL,
        status,
        Some(ProbeData::Firewall(Firewall {
            services,
            ufw_enabled,
        })),
    );
    r.notes = notes;
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    fn laptop() -> ProbeCtx {
        fixture("laptop")
            .with_addr("lo", "127.0.0.1/8")
            .with_addr("lo", "::1/128")
            .with_addr("wlp0s20f3", "192.168.1.23/24")
            .with_addr("docker0", "172.17.0.1/16")
    }

    #[test]
    fn hex_addresses() {
        // Little-endian dump of 192.168.1.1, as on x86 and ARM.
        if cfg!(target_endian = "little") {
            assert_eq!(hex_ipv4("0101A8C0"), Some(Ipv4Addr::new(192, 168, 1, 1)));
            assert_eq!(
                hex_ipv6("00000000000000000000000001000000"),
                Some(Ipv6Addr::LOCALHOST)
            );
        }
        assert_eq!(hex_ipv4("zz"), None);
        assert_eq!(hex_ipv6("00"), None);
    }

    #[test]
    fn lists_interfaces_and_route() {
        let r = interfaces(&laptop());
        assert_eq!(r.status, Status::Ok);
        let Some(ProbeData::Net(n)) = r.data else {
            panic!()
        };
        let summary: Vec<(&str, Option<&str>, &str)> = n
            .interfaces
            .iter()
            .map(|i| (i.name.as_str(), i.kind, i.state.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                ("docker0", Some("virtual"), "down"),
                ("enp0s31f6", None, "down"),
                ("lo", Some("loopback"), "unknown"),
                ("wlp0s20f3", Some("wireless"), "up"),
            ]
        );
        let wifi = &n.interfaces[3];
        assert_eq!(wifi.addrs, ["192.168.1.23/24"]);
        assert_eq!(wifi.mac.as_deref(), Some("8c:c6:81:2a:10:7f"));
        assert_eq!(n.interfaces[1].speed_mbps, None);
        assert_eq!(n.interfaces[2].mac, None);
        if cfg!(target_endian = "little") {
            let route = n.default_route.unwrap();
            assert_eq!(route.gateway, Ipv4Addr::new(192, 168, 1, 1));
            assert_eq!(route.iface, "wlp0s20f3");
        }
    }

    #[test]
    fn missing_sys_class_net() {
        assert_eq!(interfaces(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn lists_listening_sockets() {
        if cfg!(target_endian = "big") {
            return;
        }
        let r = listening(&laptop());
        let Some(ProbeData::Listening(l)) = r.data else {
            panic!()
        };
        let got: Vec<(&str, &str, u16, &str, Option<&str>)> = l
            .sockets
            .iter()
            .map(|s| {
                (
                    s.proto,
                    s.addr.as_str(),
                    s.port,
                    s.reach,
                    s.process.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                ("tcp", "0.0.0.0", 22, "all", None),
                ("tcp", "[::]", 22, "all", None),
                ("tcp", "127.0.0.1", 631, "local", None),
                ("tcp", "127.0.0.1", 8080, "local", Some("supervisor")),
                ("udp", "0.0.0.0", 5353, "all", None),
            ]
        );
        assert_eq!(l.sockets[0].user.as_deref(), Some("root"));
        assert_eq!(l.sockets[3].pid, Some(4100));
        assert_eq!(r.notes.len(), 1);
    }

    #[test]
    fn firewall_state() {
        let ctx = fixture("laptop").with_command(FIREWALL_UNITS, "inactive\nactive\ninactive\n");
        let Some(ProbeData::Firewall(f)) = firewall(&ctx).data else {
            panic!()
        };
        assert_eq!(f.services[1], ("ufw".to_string(), "active".to_string()));
        assert_eq!(f.ufw_enabled, Some(true));
        assert_eq!(firewall(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn reads_dns_config() {
        let Some(ProbeData::Dns(d)) = dns(&laptop()).data else {
            panic!()
        };
        assert_eq!(d.nameservers, ["127.0.0.53"]);
        assert_eq!(d.search, ["home.lan"]);
        assert_eq!(d.local_resolver, Some("systemd-resolved"));
        assert_eq!(
            d.hosts_order,
            ["mymachines", "resolve", "files", "myhostname", "dns"]
        );
        assert_eq!(dns(&fixture("empty")).status, Status::Unavailable);
    }
}
