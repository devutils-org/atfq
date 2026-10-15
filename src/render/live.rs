//! The "On your machine" block: probe results as short lines.

use super::text::{wrap, Kind, Line, Span};
use super::INDENT;
use crate::probes::{
    BinPath, CgroupInfo, Firewall, Fstab, Lsm, Packages, ShellEnv, Startup, TopProcesses, UdevRules,
};
use crate::probes::{
    BlockDevice, BootInfo, CpuSummary, DiskTree, DnsConfig, FsSpace, Identity, Listening,
    MemSummary, Modules, Mount, NetInterfaces, Overrides, ProbeData, ProbeResult, ProcInfo,
    ProcStates, Status, Sysctl, SystemdStatus, TmpDirs,
};
use crate::probes::{CpuCaches, CpuFeatures, CpuFreq, CpuTopology, CpuVulns, MemErrors, Numa};
use crate::probes::{DiskHardware, DiskIo, GitConfig, Gpus, SshSetup};
use crate::probes::{Interrupts, KernelOverview, NetLayers, VirtualMemory};
use crate::probes::{Ipc, Limit, Limits, Threads, VirtInfo};

/// Width of the label column (`RAM`, `Swap`, `Zombies`…).
const LABEL: usize = 10;

/// Most processes listed under Zombies or Waiting.
const SHOW_PROCS: usize = 5;

pub(crate) fn block(results: &[ProbeResult], w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, r) in results.iter().enumerate() {
        if i > 0 {
            out.push(Line::default());
        }
        out.extend(result(r, w));
    }
    out
}

pub(crate) fn result(r: &ProbeResult, w: usize) -> Vec<Line> {
    let pad = || Line::spaces(INDENT);
    let mut out = match (&r.status, &r.data) {
        (Status::NeedsRoot, _) => vec![pad().then(Line::styled("needs root", Kind::Warn))],
        (Status::Unavailable, None) => {
            vec![pad().then(Line::styled("not available on this system", Kind::Dim))]
        }
        (Status::Unsupported, _) => {
            vec![pad().then(Line::styled("not supported on this OS", Kind::Dim))]
        }
        (_, Some(ProbeData::Mem(m))) => mem(m, w),
        (_, Some(ProbeData::Disk(d))) => disk(d, w),
        (_, Some(ProbeData::Proc(p))) => procs(p, w),
        (_, Some(ProbeData::Cpu(c))) => cpu(c, w),
        (_, Some(ProbeData::Net(n))) => net(n, w),
        (_, Some(ProbeData::Listening(l))) => listening(l, w),
        (_, Some(ProbeData::Dns(d))) => dns(d, w),
        (_, Some(ProbeData::Boot(b))) => boot(b, w),
        (_, Some(ProbeData::User(u))) => user(u, w),
        (_, Some(ProbeData::Tmp(t))) => tmp(t, w),
        (_, Some(ProbeData::Sysctl(s))) => sysctl(s, w),
        (_, Some(ProbeData::Modules(m))) => modules(m, w),
        (_, Some(ProbeData::Systemd(s))) => systemd(s, w),
        (_, Some(ProbeData::Overrides(o))) => overrides(o, w),
        (_, Some(ProbeData::Env(e))) => shell_env(e, w),
        (_, Some(ProbeData::Startup(s))) => startup(s, w),
        (_, Some(ProbeData::BinPath(p))) => bin_path(p, w),
        (_, Some(ProbeData::Top(t))) => top(t, w),
        (_, Some(ProbeData::Cgroup(c))) => cgroup(c, w),
        (_, Some(ProbeData::Fstab(f))) => fstab(f, w),
        (_, Some(ProbeData::Udev(u))) => udev(u, w),
        (_, Some(ProbeData::Firewall(f))) => firewall(f, w),
        (_, Some(ProbeData::Lsm(l))) => lsm(l, w),
        (_, Some(ProbeData::Packages(p))) => packages(p, w),
        (_, Some(ProbeData::Kernel(k))) => kernel(k, w),
        (_, Some(ProbeData::Irq(i))) => irq(i, w),
        (_, Some(ProbeData::Vm(v))) => vm(v, w),
        (_, Some(ProbeData::Layers(l))) => layers(l, w),
        (_, Some(ProbeData::Features(f))) => features(f, w),
        (_, Some(ProbeData::Topology(t))) => topology(t, w),
        (_, Some(ProbeData::Caches(c))) => caches(c, w),
        (_, Some(ProbeData::Freq(f))) => freq(f, w),
        (_, Some(ProbeData::Vulns(v))) => vulns(v, w),
        (_, Some(ProbeData::Numa(n))) => numa(n, w),
        (_, Some(ProbeData::MemErrors(e))) => mem_errors(e, w),
        (_, Some(ProbeData::DiskHardware(d))) => disk_hardware(d, w),
        (_, Some(ProbeData::DiskIo(d))) => disk_io(d, w),
        (_, Some(ProbeData::Gpus(g))) => gpus(g, w),
        (_, Some(ProbeData::Ssh(x))) => ssh(x, w),
        (_, Some(ProbeData::Git(g))) => git(g, w),
        (_, Some(ProbeData::Virt(v))) => virt(v, w),
        (_, Some(ProbeData::Threads(t))) => threads(t, w),
        (_, Some(ProbeData::Limits(l))) => limits(l, w),
        (_, Some(ProbeData::Ipc(i))) => ipc(i, w),
        (_, None) => Vec::new(),
    };
    for note in &r.notes {
        let spans = [Span::plain(format!("note: {note}"))];
        out.extend(
            wrap(&spans, &pad(), &pad(), w)
                .into_iter()
                .map(Line::dimmed),
        );
    }
    out
}

/// `  Label    text`, wrapped under the text.
fn row(label: &str, text: &str, w: usize) -> Vec<Line> {
    // A label wider than the column still gets a space after it.
    let pad = LABEL.max(label.chars().count() + 1);
    let first = Line::spaces(INDENT).then(Line::styled(format!("{label:<pad$}"), Kind::Bold));
    let rest = Line::spaces(INDENT + LABEL);
    let mut lines = wrap(&[Span::plain(text)], &first, &rest, w);
    if lines.is_empty() {
        lines.push(first);
    }
    lines
}

/// Like [`row`], but keeps the spacing of `text` (aligned columns) when the
/// line fits; wraps it like any text otherwise.
fn row_aligned(label: &str, text: &str, w: usize) -> Vec<Line> {
    if INDENT + LABEL + text.chars().count() > w {
        return row(label, text, w);
    }
    vec![Line::spaces(INDENT)
        .then(Line::styled(format!("{label:<LABEL$}"), Kind::Bold))
        .then(Line::plain(text))]
}

/// Binary units, like `free -h`: `15.5 GiB`.
pub(crate) fn size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    format!("{v:.1} {}", UNITS[unit])
}

fn percent(part: u64, total: u64) -> u64 {
    if total == 0 {
        0
    } else {
        (part as f64 * 100.0 / total as f64).round() as u64
    }
}

fn mem(m: &MemSummary, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    let ram = match m.available {
        Some(a) => format!(
            "{} available of {} ({}%) · {} completely free",
            size(a),
            size(m.total),
            percent(a, m.total),
            size(m.free)
        ),
        None => format!("{} total · {} completely free", size(m.total), size(m.free)),
    };
    out.extend(row("RAM", &ram, w));
    let cache = if m.shared > 0 {
        format!(
            "{} in buff/cache, {} of it shared (tmpfs)",
            size(m.buff_cache),
            size(m.shared)
        )
    } else {
        format!("{} in buff/cache", size(m.buff_cache))
    };
    out.extend(row("Cache", &cache, w));
    let mut swap = if m.swap_total == 0 {
        "none".to_string()
    } else {
        let used = m.swap_total.saturating_sub(m.swap_free);
        format!("{} used of {}", size(used), size(m.swap_total))
    };
    if let Some(s) = m.swappiness {
        swap.push_str(&format!(" · swappiness {s}"));
    }
    out.extend(row("Swap", &swap, w));
    out
}

/// State letter → (one, many), in the order shown.
const STATES: &[(&str, &str, &str)] = &[
    ("R", "running", "running"),
    ("S", "sleeping", "sleeping"),
    ("D", "waiting on I/O (D)", "waiting on I/O (D)"),
    ("T", "stopped", "stopped"),
    ("t", "stopped by a debugger", "stopped by a debugger"),
    ("I", "idle kernel thread", "idle kernel threads"),
    ("Z", "zombie", "zombies"),
];

fn procs(p: &ProcStates, w: usize) -> Vec<Line> {
    let mut parts = Vec::new();
    for (letter, one, many) in STATES {
        let n = p.counts.get(*letter).copied().unwrap_or(0);
        if n > 0 {
            parts.push(format!("{n} {}", if n == 1 { one } else { many }));
        }
    }
    for (letter, n) in &p.counts {
        if !STATES.iter().any(|(l, ..)| l == letter) {
            parts.push(format!("{n} in state {letter}"));
        }
    }
    let total = if p.total == 1 { "process" } else { "processes" };
    let mut out = row(
        "Total",
        &format!("{} {total}: {}", p.total, parts.join(", ")),
        w,
    );

    let zombies = p.counts.get("Z").copied().unwrap_or(0) as usize;
    if zombies == 0 {
        out.extend(row("Zombies", "none", w));
    } else {
        out.extend(proc_list("Zombies", &p.zombies, zombies, w));
    }
    let waiting = p.counts.get("D").copied().unwrap_or(0) as usize;
    if waiting > 0 {
        out.extend(proc_list("Waiting", &p.waiting, waiting, w));
    }
    out
}

fn proc_list(label: &str, list: &[ProcInfo], total: usize, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, p) in list.iter().take(SHOW_PROCS).enumerate() {
        let parent = match &p.parent {
            Some(name) => format!("parent {} {name}", p.ppid),
            None => format!("parent {}", p.ppid),
        };
        let text = format!("PID {} {}, {parent}", p.pid, p.comm);
        out.extend(row(if i == 0 { label } else { "" }, &text, w));
    }
    let shown = list.len().min(SHOW_PROCS);
    if total > shown {
        out.extend(row("", &format!("and {} more", total - shown), w));
    }
    out
}

/// One row of the disk table.
struct Row {
    depth: usize,
    name: String,
    size: String,
    fstype: String,
    mount: String,
    detail: String,
}

fn disk(d: &DiskTree, w: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    for dev in &d.devices {
        lines.push(device_line(dev));
        if let Some(m) = dev.mounts.first() {
            lines.push(mount_line(
                1,
                "(whole disk)".into(),
                String::new(),
                m,
                dev.mounts.len(),
            ));
        }
        for p in &dev.partitions {
            match p.mounts.first() {
                Some(m) => lines.push(mount_line(
                    1,
                    p.name.clone(),
                    size(p.size),
                    m,
                    p.mounts.len(),
                )),
                None => lines.push(Row {
                    depth: 1,
                    name: p.name.clone(),
                    size: size(p.size),
                    fstype: String::new(),
                    mount: String::new(),
                    detail: if p.swap { "swap".into() } else { String::new() },
                }),
            }
        }
    }

    let col = |f: &dyn Fn(&Row) -> usize| lines.iter().map(f).max().unwrap_or(0);
    let name_w = col(&|l| l.depth * 2 + l.name.chars().count());
    let size_w = col(&|l| l.size.chars().count());
    let fs_w = col(&|l| l.fstype.chars().count());
    let mount_w = col(&|l| l.mount.chars().count());

    let mut out = Vec::new();
    for l in &lines {
        let name = format!("{:<name_w$}", format!("{}{}", "  ".repeat(l.depth), l.name));
        let kind = if l.depth == 0 {
            Kind::Bold
        } else {
            Kind::Plain
        };
        let mut cells = format!("  {:>size_w$}", l.size);
        if !l.mount.is_empty() {
            cells.push_str(&format!("  {:<fs_w$}  {:<mount_w$}", l.fstype, l.mount));
        }
        let used = INDENT + name_w + cells.chars().count();
        let start = Line::spaces(INDENT).then(Line::styled(name, kind));
        if l.detail.is_empty() {
            out.push(start.then(Line::plain(cells.trim_end())));
        } else if used + 2 + l.detail.chars().count() <= w {
            out.push(start.then(Line::plain(format!("{cells}  {}", l.detail))));
        } else {
            out.push(start.then(Line::plain(cells.trim_end())));
            out.push(Line::spaces(INDENT + name_w + 2).then(Line::plain(&l.detail)));
        }
    }
    if d.hidden > 0 {
        let what = if d.hidden == 1 { "device" } else { "devices" };
        out.push(Line::spaces(INDENT).then(Line::styled(
            format!("{} loop or empty {what} not shown", d.hidden),
            Kind::Dim,
        )));
    }
    if out.is_empty() {
        out.push(Line::spaces(INDENT).then(Line::plain("no disks found")));
    }
    out
}

fn device_line(dev: &BlockDevice) -> Row {
    let mut detail = Vec::new();
    match dev.rotational {
        Some(false) => detail.push("SSD".to_string()),
        Some(true) => detail.push("rotational".to_string()),
        None => {}
    }
    if dev.removable {
        detail.push("removable".into());
    }
    if dev.swap {
        detail.push("swap".into());
    }
    if let Some(m) = &dev.mapper_name {
        detail.push(format!("mapper: {m}"));
    }
    if let Some(m) = &dev.model {
        detail.push(m.clone());
    }
    Row {
        depth: 0,
        name: dev.name.clone(),
        size: size(dev.size),
        fstype: String::new(),
        mount: String::new(),
        detail: detail.join(" · "),
    }
}

fn mount_line(depth: usize, name: String, size_text: String, m: &Mount, count: usize) -> Row {
    let mut mount = m.point.clone();
    if count > 1 {
        mount.push_str(&format!(" (+{})", count - 1));
    }
    let detail = m.space.map(free_text).unwrap_or_default();
    Row {
        depth,
        name,
        size: size_text,
        fstype: m.fstype.clone(),
        mount,
        detail,
    }
}

/// `120.0 GiB free (30%)`, plus inode use when the filesystem counts them.
fn free_text(s: FsSpace) -> String {
    let mut t = format!(
        "{} free ({}%)",
        size(s.available),
        percent(s.available, s.total)
    );
    if s.inodes_total > 0 {
        let used = s.inodes_total.saturating_sub(s.inodes_free);
        t.push_str(&format!(
            " · inodes {}% used",
            percent(used, s.inodes_total)
        ));
    }
    t
}

fn cpu(c: &CpuSummary, w: usize) -> Vec<Line> {
    let n = c.logical;
    let cpus = format!("{n} logical CPU{}", if n == 1 { "" } else { "s" });
    let mut out = row(
        "CPU",
        &match &c.model {
            Some(m) => format!("{m} · {cpus}"),
            None => format!("{cpus} (the kernel reports no model name)"),
        },
        w,
    );
    if let Some([a, b, z]) = c.load {
        out.extend(row(
            "Load",
            &format!("{a:.2}, {b:.2}, {z:.2} over 1, 5 and 15 minutes, for {n} CPUs"),
            w,
        ));
    }
    if let (Some(r), Some(t)) = (c.running, c.tasks) {
        out.extend(row(
            "Tasks",
            &format!("{r} running right now, {t} in all"),
            w,
        ));
    }
    if let Some(g) = &c.governor {
        out.extend(row("Governor", g, w));
    }
    out
}

fn net(n: &NetInterfaces, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for i in &n.interfaces {
        let mut facts = vec![i.state.clone()];
        facts.extend(i.kind.map(str::to_string));
        if let Some(s) = i.speed_mbps {
            facts.push(if s >= 1000 && s % 1000 == 0 {
                format!("{} Gbit/s", s / 1000)
            } else {
                format!("{s} Mbit/s")
            });
        }
        if let Some(m) = &i.mac {
            facts.push(format!("MAC {m}"));
        }
        out.extend(row(&i.name, &facts.join(" · "), w));
        if !i.addrs.is_empty() {
            out.extend(row("", &i.addrs.join(", "), w));
        }
    }
    match &n.default_route {
        Some(r) => out.extend(row(
            "Route",
            &format!("default via {} on {}", r.gateway, r.iface),
            w,
        )),
        None => out.extend(row(
            "Route",
            "no default route: no way out to the Internet",
            w,
        )),
    }
    out
}

fn listening(l: &Listening, w: usize) -> Vec<Line> {
    if l.sockets.is_empty() {
        return row("Ports", "nothing is waiting for connections", w);
    }
    let addr_w = l
        .sockets
        .iter()
        .map(|s| s.addr.chars().count() + 1 + s.port.to_string().len())
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    for s in &l.sockets {
        let reach = match s.reach {
            "all" => "all interfaces",
            "local" => "this machine only",
            _ => "that address only",
        };
        let owner = match (&s.process, s.pid, &s.user) {
            (Some(p), Some(pid), _) => format!("{p} (PID {pid})"),
            (_, _, Some(u)) => format!("user {u}"),
            _ => String::new(),
        };
        let addr = format!("{}:{}", s.addr, s.port);
        let text = format!("{addr:<addr_w$}  {reach:<17}  {owner}");
        out.extend(row_aligned(&s.proto.to_uppercase(), text.trim_end(), w));
    }
    if l.total as usize > l.sockets.len() {
        out.extend(row(
            "",
            &format!("and {} more", l.total as usize - l.sockets.len()),
            w,
        ));
    }
    out
}

fn dns(d: &DnsConfig, w: usize) -> Vec<Line> {
    let mut servers = if d.nameservers.is_empty() {
        "none listed in /etc/resolv.conf".to_string()
    } else {
        d.nameservers.join(", ")
    };
    if let Some(r) = d.local_resolver {
        servers.push_str(&format!(
            " ({r}: see resolvectl status for the real servers)"
        ));
    }
    let mut out = row("Servers", &servers, w);
    if !d.search.is_empty() {
        out.extend(row("Search", &d.search.join(", "), w));
    }
    if !d.hosts_order.is_empty() {
        out.extend(row("Order", &d.hosts_order.join(" → "), w));
    }
    out
}

fn boot(b: &BootInfo, w: usize) -> Vec<Line> {
    use crate::probes::Firmware;
    let mut out = Vec::new();
    if let Some(f) = b.firmware {
        let mut text = match f {
            Firmware::Uefi => "UEFI".to_string(),
            Firmware::Bios => "BIOS (legacy boot)".to_string(),
            Firmware::DeviceTree => "device tree (no UEFI)".to_string(),
        };
        match b.secure_boot {
            Some(true) => text.push_str(" · Secure Boot on"),
            Some(false) => text.push_str(" · Secure Boot off"),
            None => {}
        }
        out.extend(row("Firmware", &text, w));
    }
    if let Some(k) = &b.kernel {
        out.extend(row("Kernel", k, w));
    }
    if let Some(c) = &b.cmdline {
        out.extend(row("Cmdline", c, w));
    }
    if let Some(i) = &b.init {
        let text = match &b.default_target {
            Some(t) => format!("{i} · default target {t}"),
            None => i.clone(),
        };
        out.extend(row("Init", &text, w));
    }
    if let Some(l) = &b.bootloader {
        let mut text = l.clone();
        if b.ukis > 0 {
            text.push_str(&format!(" · {} unified kernel image(s)", b.ukis));
        }
        out.extend(row("Loader", &text, w));
    }
    if let Some(u) = b.uptime_secs {
        out.extend(row("Up", &duration(u), w));
    }
    out
}

/// `3 days, 4 h` / `2 h 5 min` / `42 min`.
fn duration(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60);
    match (d, h) {
        (0, 0) => format!("{m} min"),
        (0, _) => format!("{h} h {m} min"),
        (1, _) => format!("1 day, {h} h"),
        _ => format!("{d} days, {h} h"),
    }
}

fn user(u: &Identity, w: usize) -> Vec<Line> {
    let name = u.user.clone().unwrap_or_else(|| "(no name)".to_string());
    let mut out = row("User", &format!("{name} (uid {}, gid {})", u.uid, u.gid), w);
    out.extend(row("Groups", &u.groups.join(", "), w));
    out.extend(row(
        "Admin",
        &match &u.admin_group {
            Some(g) => format!("member of {g}: the group sudo usually trusts"),
            None => "not in sudo, wheel or admin".to_string(),
        },
        w,
    ));
    if let Some(mask) = &u.umask {
        if let Ok(m) = u32::from_str_radix(mask, 8) {
            let (f, d) = (0o666 & !m, 0o777 & !m);
            out.extend(row(
                "Umask",
                &format!(
                    "{mask}: new files {f:03o} ({}), new folders {d:03o}",
                    rwx(f)
                ),
                w,
            ));
        }
    }
    out
}

/// `0o644` → `rw-r--r--`.
fn rwx(mode: u32) -> String {
    (0..9)
        .map(|i| {
            let bit = 1 << (8 - i);
            if mode & bit == 0 {
                '-'
            } else {
                ['r', 'w', 'x'][i % 3]
            }
        })
        .collect()
}

fn tmp(t: &TmpDirs, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for d in &t.dirs {
        let place = if d.fstype == "tmpfs" {
            "in RAM (tmpfs), emptied at every boot".to_string()
        } else if d.mount_point == d.path {
            format!("on disk ({}), its own filesystem", d.fstype)
        } else {
            format!("on disk ({}), part of {}", d.fstype, d.mount_point)
        };
        let mut text = place;
        if let Some(s) = d.space {
            text.push_str(&format!(" · {}", free_text(s)));
        }
        out.extend(row(&d.path, &text, w));
    }
    out
}

fn sysctl(s: &Sysctl, w: usize) -> Vec<Line> {
    let name_w = s.values.iter().map(|v| v.name.len()).max().unwrap_or(0);
    let value_w = s
        .values
        .iter()
        .map(|v| v.value.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    for v in &s.values {
        let text = format!("{:<name_w$}  {:>value_w$}  {}", v.name, v.value, v.meaning);
        if INDENT + text.chars().count() <= w {
            out.push(Line::spaces(INDENT).then(Line::plain(text)));
        } else {
            out.extend(row(&v.name, &format!("{}  {}", v.value, v.meaning), w));
        }
    }
    if s.configured.is_empty() {
        out.extend(row(
            "Set in",
            "nothing in /etc/sysctl.conf or /etc/sysctl.d/",
            w,
        ));
    } else {
        for (i, c) in s.configured.iter().enumerate() {
            let label = if i == 0 { "Set in" } else { "" };
            out.extend(row(
                label,
                &format!("{} = {} ({})", c.name, c.value, c.file),
                w,
            ));
        }
    }
    out
}

fn modules(m: &Modules, w: usize) -> Vec<Line> {
    let biggest: Vec<String> = m
        .largest
        .iter()
        .map(|(name, bytes)| format!("{name} {}", size(*bytes)))
        .collect();
    let mut out = row(
        "Loaded",
        &format!("{} modules; biggest: {}", m.loaded, biggest.join(", ")),
        w,
    );
    match m.tainted {
        Some(0) => out.extend(row("Tainted", "no", w)),
        Some(_) => {
            for (i, (c, what)) in m.taint_flags.iter().enumerate() {
                out.extend(row(
                    if i == 0 { "Tainted" } else { "" },
                    &format!("{c}: {what}"),
                    w,
                ));
            }
        }
        None => {}
    }
    out
}

fn systemd(s: &SystemdStatus, w: usize) -> Vec<Line> {
    let state = match s.state.as_str() {
        "running" => "running: every unit is fine".to_string(),
        "degraded" => "degraded: some units failed".to_string(),
        other => other.to_string(),
    };
    let mut out = row("State", &state, w);
    if s.failed.is_empty() {
        out.extend(row("Failed", "none", w));
    }
    for (i, f) in s.failed.iter().enumerate() {
        let text = if f.description.is_empty() {
            f.unit.clone()
        } else {
            format!("{} ({})", f.unit, f.description)
        };
        out.extend(row(if i == 0 { "Failed" } else { "" }, &text, w));
    }
    out
}

/// Most drop-ins listed; the rest are counted.
const SHOW_DROPINS: usize = 6;

fn overrides(o: &Overrides, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, d) in o.dropins.iter().take(SHOW_DROPINS).enumerate() {
        out.extend(row(
            if i == 0 { "Drop-ins" } else { "" },
            &format!("{}: {}", d.unit, d.files.join(", ")),
            w,
        ));
    }
    if o.dropins.len() > SHOW_DROPINS {
        out.extend(row(
            "",
            &format!(
                "and {} more: systemd-delta lists them all",
                o.dropins.len() - SHOW_DROPINS
            ),
            w,
        ));
    }
    for (label, list) in [
        ("Replaced", &o.replaced),
        ("Local", &o.local),
        ("Masked", &o.masked),
    ] {
        if !list.is_empty() {
            out.extend(row(label, &list.join(", "), w));
        }
    }
    if out.is_empty() {
        out.extend(row(
            "Changes",
            "none: every unit is as its package ships it",
            w,
        ));
    }
    out
}

fn shell_env(e: &ShellEnv, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for (k, v) in &e.vars {
        out.extend(row(k, v, w));
    }
    if let Some(login) = &e.login_shell {
        let shell = e.vars.iter().find(|(k, _)| k == "SHELL").map(|(_, v)| v);
        let text = match shell {
            Some(s) if s != login => format!("{login} (SHELL says {s}: changed since login?)"),
            _ => login.clone(),
        };
        out.extend(row("Login", &text, w));
    }
    out.extend(row(
        "In all",
        &format!(
            "{} variables set; only well-known ones are shown here",
            e.total
        ),
        w,
    ));
    out
}

fn startup(s: &Startup, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if let Some(l) = &s.login_shell {
        out.extend(row("Shell", l, w));
    }
    out.extend(row(
        "Files",
        &if s.files.is_empty() {
            "none".to_string()
        } else {
            s.files.join(", ")
        },
        w,
    ));
    if let Some(f) = &s.bash_login_file {
        let mut text = format!("bash login shells read {f}");
        if !s.ignored_by_bash.is_empty() {
            text.push_str(&format!(" and ignore {}", s.ignored_by_bash.join(", ")));
        }
        out.extend(row("Bash", &text, w));
    }
    out
}

fn bin_path(p: &BinPath, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    // Each folder once, in search order; repeats are summed up after.
    for (i, e) in p.entries.iter().filter(|e| !e.duplicate).enumerate() {
        let mut text = if e.dir.is_empty() {
            "(empty)".to_string()
        } else {
            e.dir.clone()
        };
        if e.current_dir {
            text.push_str(" ⚠ the current folder, wherever you are");
        } else if !e.exists {
            text.push_str(" (doesn't exist)");
        }
        out.extend(row(if i == 0 { "PATH" } else { "" }, &text, w));
    }
    let mut repeated: Vec<&str> = Vec::new();
    for e in p.entries.iter().filter(|e| e.duplicate) {
        if !repeated.contains(&e.dir.as_str()) {
            repeated.push(&e.dir);
        }
    }
    if !repeated.is_empty() {
        out.extend(row(
            "Repeated",
            &format!(
                "{} (listed more than once: harmless, the first one wins)",
                repeated.join(", ")
            ),
            w,
        ));
    }
    if let Some(v) = &p.ld_library_path {
        out.extend(row("Libraries", &format!("LD_LIBRARY_PATH={v}"), w));
    }
    if let Some(v) = &p.ld_preload {
        out.extend(row("Preload", &format!("LD_PRELOAD={v}"), w));
    }
    out.extend(row(
        "Runs",
        &if p.loaders.is_empty() {
            "no known dynamic loader found".to_string()
        } else {
            format!("programs built for {}", p.loaders.join("; "))
        },
        w,
    ));
    out
}

fn top(t: &TopProcesses, w: usize) -> Vec<Line> {
    let list = |entries: &[crate::probes::TopEntry], unit: &dyn Fn(u64) -> String| {
        entries
            .iter()
            .map(|e| format!("{} ({}, PID {})", e.name, unit(e.amount), e.pid))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut out = row("Memory", &list(&t.by_memory, &|b| size(b)), w);
    out.extend(row("CPU time", &list(&t.by_cpu, &|s| duration_short(s)), w));
    out
}

/// `42 s`, `3 min 5 s`, `2 h 4 min`.
fn duration_short(secs: u64) -> String {
    match secs {
        0..60 => format!("{secs} s"),
        60..3600 => format!("{} min {} s", secs / 60, secs % 60),
        _ => format!("{} h {} min", secs / 3600, secs % 3600 / 60),
    }
}

fn cgroup(c: &CgroupInfo, w: usize) -> Vec<Line> {
    let mut out = row("Version", &format!("cgroup v{}", c.version), w);
    out.extend(row("Yours", &c.path, w));
    if !c.controllers.is_empty() {
        out.extend(row("Controls", &c.controllers.join(", "), w));
    }
    out.extend(row(
        "Memory",
        &match &c.memory_limit {
            Some((n, at)) => format!("limited to {} (set on {at})", size(*n)),
            None => "no limit on your session".to_string(),
        },
        w,
    ));
    out
}

fn fstab(f: &Fstab, w: usize) -> Vec<Line> {
    if f.entries.is_empty() {
        return row(
            "Entries",
            "none: mounts come from systemd units or the initramfs",
            w,
        );
    }
    let mut out = Vec::new();
    for (i, e) in f.entries.iter().enumerate() {
        let state = match e.mounted {
            Some(true) => "mounted",
            Some(false) => "NOT mounted",
            None => "swap",
        };
        let text = format!(
            "{} ({}, {}) from {} · {state}",
            e.mount_point, e.fstype, e.options, e.source
        );
        out.extend(row(if i == 0 { "Entries" } else { "" }, &text, w));
    }
    out
}

fn udev(u: &UdevRules, w: usize) -> Vec<Line> {
    let mut out = row(
        "Local",
        &if u.local.is_empty() {
            "no rules of your own in /etc/udev/rules.d".to_string()
        } else {
            u.local.join(", ")
        },
        w,
    );
    out.extend(row("Packaged", &format!("{} rules files", u.packaged), w));
    out
}

fn firewall(f: &Firewall, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for (name, state) in &f.services {
        out.extend(row(name, state, w));
    }
    if let Some(on) = f.ufw_enabled {
        out.extend(row(
            "ufw.conf",
            if on { "ENABLED=yes" } else { "ENABLED=no" },
            w,
        ));
    }
    let active = f.services.iter().any(|(_, s)| s == "active") || f.ufw_enabled == Some(true);
    if !active {
        out.extend(row(
            "Summary",
            "no firewall service active: rules, if any, were set another way",
            w,
        ));
    }
    out
}

fn lsm(l: &Lsm, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if !l.modules.is_empty() {
        out.extend(row("Modules", &l.modules.join(", "), w));
    }
    match l.selinux_enforcing {
        Some(true) => out.extend(row("SELinux", "enforcing: denials are blocked", w)),
        Some(false) => out.extend(row("SELinux", "permissive: denials are only logged", w)),
        None => {}
    }
    if let Some(c) = &l.selinux_config {
        out.extend(row("At boot", &format!("SELINUX={c}"), w));
    }
    if l.apparmor {
        let text = if l.apparmor_profiles.is_empty() {
            "enabled".to_string()
        } else {
            let modes: Vec<String> = l
                .apparmor_profiles
                .iter()
                .map(|(m, n)| format!("{n} {m}"))
                .collect();
            format!("enabled · profiles: {}", modes.join(", "))
        };
        out.extend(row("AppArmor", &text, w));
    }
    if l.selinux_enforcing.is_none() && !l.apparmor {
        out.extend(row("MAC", "neither SELinux nor AppArmor is active", w));
    }
    out
}

fn packages(p: &Packages, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for (name, count) in &p.native {
        let text = match count {
            Some(n) => format!("{n} packages"),
            None => "installed (count with: rpm -qa | wc -l)".to_string(),
        };
        out.extend(row(name, &text, w));
    }
    if let Some(n) = p.snaps {
        out.extend(row("Snap", &format!("{n} snaps"), w));
    }
    if let Some(n) = p.flatpaks {
        out.extend(row("Flatpak", &format!("{n} apps"), w));
    }
    out
}

/// `780,774`, `24.8 million`, `1.2 billion`.
fn count(n: u64) -> String {
    match n {
        1_000_000_000.. => format!("{:.1} billion", n as f64 / 1e9),
        1_000_000.. => format!("{:.1} million", n as f64 / 1e6),
        _ => grouped(n),
    }
}

/// `1,048,576`: exact, for limits and settings.
fn grouped(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn kernel(k: &KernelOverview, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if let Some(r) = &k.release {
        let text = match &k.compiler {
            Some(c) => format!("{r}, built with {c}"),
            None => r.clone(),
        };
        out.extend(row("Kernel", &text, w));
    }
    if let Some(b) = &k.build {
        out.extend(row("Build", b, w));
    }
    let per_sec = |n: u64| {
        k.uptime_secs
            .filter(|&s| s > 0)
            .map(|s| format!(" ({} per second)", count(n / s)))
            .unwrap_or_default()
    };
    let mut since = Vec::new();
    if let Some(n) = k.context_switches {
        since.push(format!("{} context switches{}", count(n), per_sec(n)));
    }
    if let Some(n) = k.processes_started {
        since.push(format!("{} processes started", count(n)));
    }
    if let Some(n) = k.interrupts {
        since.push(format!("{} interrupts", count(n)));
    }
    if !since.is_empty() {
        let label = k
            .uptime_secs
            .map(|s| format!("in the {} since boot: ", duration(s)))
            .unwrap_or_default();
        out.extend(row("Work", &format!("{label}{}", since.join(", ")), w));
    }
    if let (Some(r), Some(b)) = (k.running, k.blocked) {
        out.extend(row(
            "Now",
            &format!("{r} tasks running or ready, {b} waiting on I/O"),
            w,
        ));
    }
    if !k.filesystems.is_empty() {
        out.extend(row("Disk FS", &k.filesystems.join(", "), w));
    }
    out
}

fn irq(i: &Interrupts, w: usize) -> Vec<Line> {
    let list = |items: &[(String, u64)]| {
        items
            .iter()
            .map(|(name, n)| format!("{name} {}", count(*n)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut out = row("Hardware", &list(&i.top), w);
    if !i.softirqs.is_empty() {
        out.extend(row("Software", &list(&i.softirqs), w));
    }
    out.extend(row("CPUs", &format!("{} share the work", i.cpus), w));
    out
}

fn vm(v: &VirtualMemory, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if let Some(p) = v.page_size {
        out.extend(row("Page", &size(p), w));
    }
    if let (Some(minor), Some(major)) = (v.minor_faults, v.major_faults) {
        out.extend(row(
            "Faults",
            &format!(
                "{} served from RAM, {} that read the disk, since boot",
                count(minor),
                count(major)
            ),
            w,
        ));
    }
    if let (Some(i), Some(o)) = (v.swap_in, v.swap_out) {
        let bytes = |n: u64| {
            v.page_size
                .map_or(format!("{} pages", count(n)), |p| size(n * p))
        };
        out.extend(row(
            "Swap",
            &format!("{} read back, {} written out", bytes(i), bytes(o)),
            w,
        ));
    }
    if let Some(t) = &v.thp {
        out.extend(row(
            "Huge pages",
            &format!("transparent huge pages: {t}"),
            w,
        ));
    }
    out
}

fn layers(l: &NetLayers, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    let mut link = if l.links_up.is_empty() {
        "no interface up".to_string()
    } else {
        format!("{} up", l.links_up.join(", "))
    };
    for n in &l.neighbours {
        link.push_str(&format!(
            " · neighbour {} is {} on {}",
            n.ip, n.mac, n.iface
        ));
    }
    out.extend(row("L2 link", &link, w));
    let mut ip = if l.addresses.is_empty() {
        "no address".to_string()
    } else {
        l.addresses.join(", ")
    };
    if let Some(g) = &l.gateway {
        ip.push_str(&format!(" · gateway {}", g.gateway));
    }
    if l.forwarding == Some(true) {
        ip.push_str(" · forwards packets for others");
    }
    out.extend(row("L3 IP", &ip, w));
    if let Some(t) = &l.tcp {
        let resent = if t.segments_out > 0 {
            format!(
                " · {} of {} segments resent",
                count(t.retransmitted),
                count(t.segments_out)
            )
        } else {
            String::new()
        };
        out.extend(row(
            "L4 TCP",
            &format!(
                "{} connections open now; {} started and {} accepted since boot{resent}",
                t.established,
                count(t.opened),
                count(t.accepted)
            ),
            w,
        ));
    }
    if let Some((i, o)) = l.udp {
        out.extend(row(
            "L4 UDP",
            &format!("{} datagrams in, {} out, since boot", count(i), count(o)),
            w,
        ));
    }
    out.extend(row(
        "L7 DNS",
        &if l.dns_servers.is_empty() {
            "no server in /etc/resolv.conf".to_string()
        } else {
            l.dns_servers.join(", ")
        },
        w,
    ));
    out
}

fn features(f: &CpuFeatures, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    let arch = match (&f.arch, &f.vendor) {
        (Some(a), Some(v)) => format!("{a}, made by {v}"),
        (Some(a), None) => a.clone(),
        (None, Some(v)) => format!("made by {v}"),
        (None, None) => "unknown".to_string(),
    };
    out.extend(row("Arch", &arch, w));
    if let Some(l) = &f.level {
        out.extend(row(
            "Level",
            &format!("{l}: programs built for this level or below run here"),
            w,
        ));
    }
    if !f.notable.is_empty() {
        let names: Vec<&str> = f
            .notable
            .iter()
            .filter_map(|n| crate::probes::cpu_notable(n))
            .collect();
        out.extend(row("Has", &names.join(", "), w));
    }
    if f.flags > 0 {
        out.extend(row(
            "Flags",
            &format!("{} in all, in /proc/cpuinfo", f.flags),
            w,
        ));
    }
    out
}

fn topology(t: &CpuTopology, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    let plural = |n: u32, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut cpus = format!(
        "{} on {}",
        plural(t.logical, "logical CPU", "logical CPUs"),
        plural(t.cores, "core", "cores")
    );
    if t.threads_per_core > 1 {
        cpus.push_str(&format!(
            ", up to {} threads per core (SMT)",
            t.threads_per_core
        ));
    }
    if t.packages > 1 {
        cpus.push_str(&format!(", in {} sockets", t.packages));
    }
    out.extend(row("CPUs", &cpus, w));
    if !t.kinds.is_empty() {
        let kinds: Vec<String> = t
            .kinds
            .iter()
            .map(|k| format!("{} {}", k.name, k.cpus))
            .collect();
        out.extend(row("Kinds", &kinds.join(" · "), w));
    }
    if let Some(i) = &t.isolated {
        out.extend(row(
            "Isolated",
            &format!("{i}: the scheduler puts nothing there by itself"),
            w,
        ));
    }
    if let Some(o) = &t.offline {
        out.extend(row("Offline", o, w));
    }
    out
}

fn caches(c: &CpuCaches, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for cache in &c.caches {
        let label = match cache.kind.as_str() {
            "Data" => format!("L{} data", cache.level),
            "Instruction" => format!("L{} code", cache.level),
            _ => format!("L{}", cache.level),
        };
        let size = size(cache.size);
        let n = cache.instances;
        let text = if cache.shared_by <= 1 {
            format!("{n} × {size}, one for each CPU")
        } else if n == 1 {
            format!("{size}, shared by all {} CPUs", cache.shared_by)
        } else {
            format!("{n} × {size}, each shared by {} CPUs", cache.shared_by)
        };
        out.extend(row(&label, &text, w));
    }
    if let Some(line) = c.caches.iter().find_map(|c| c.line) {
        out.extend(row(
            "Line",
            &format!("{line} bytes: the block memory moves in"),
            w,
        ));
    }
    out
}

/// `400 MHz`, `4.7 GHz`.
fn khz(k: u64) -> String {
    if k < 1_000_000 {
        format!("{} MHz", k / 1000)
    } else {
        format!("{:.1} GHz", k as f64 / 1e6)
    }
}

fn freq(f: &CpuFreq, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if let Some(d) = &f.driver {
        let text = match &f.governor {
            Some(g) => format!("{d}, governor {g}"),
            None => d.clone(),
        };
        out.extend(row("Driver", &text, w));
    }
    if let (Some(lo), Some(hi)) = (f.min_khz, f.max_khz) {
        let text = if lo == hi {
            format!("fixed at {}", khz(hi))
        } else {
            format!("{} to {}", khz(lo), khz(hi))
        };
        out.extend(row("Range", &text, w));
    }
    if let (Some(lo), Some(hi)) = (f.current_khz.iter().min(), f.current_khz.iter().max()) {
        let n = f.current_khz.len();
        let text = if lo == hi {
            format!("{} on all {n} CPUs", khz(*hi))
        } else {
            format!("{} to {} across {n} CPUs", khz(*lo), khz(*hi))
        };
        out.extend(row("Now", &text, w));
    }
    if let Some(b) = f.boost {
        out.extend(row("Turbo", if b { "allowed" } else { "off" }, w));
    }
    if !f.idle_states.is_empty() {
        let states: Vec<String> = f
            .idle_states
            .iter()
            .map(|(name, n)| format!("{name} {}", count(*n)))
            .collect();
        let text = match &f.idle_driver {
            Some(d) => format!("{d}: {}", states.join(", ")),
            None => states.join(", "),
        };
        out.extend(row("Idle", &format!("{text} (CPU 0, times entered)"), w));
    }
    if let Some((zone, t)) = &f.hottest {
        out.extend(row("Hottest", &format!("{t:.0} °C ({zone})"), w));
    }
    out
}

fn vulns(v: &CpuVulns, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if v.mitigations_off {
        out.extend(row(
            "Warning",
            "mitigations=off: the kernel skips every workaround",
            w,
        ));
    }
    for (i, (name, state)) in v.vulnerable.iter().enumerate() {
        let label = if i == 0 { "Exposed" } else { "" };
        let text = if state == "Vulnerable" {
            name.clone()
        } else {
            format!("{name}: {state}")
        };
        out.extend(row(label, &text, w));
    }
    for (i, (name, how)) in v.mitigated.iter().enumerate() {
        let label = if i == 0 { "Mitigated" } else { "" };
        out.extend(row(label, &format!("{name}: {how}"), w));
    }
    if !v.not_affected.is_empty() {
        out.extend(row(
            "Immune",
            &format!(
                "{} others: this CPU doesn't have them",
                v.not_affected.len()
            ),
            w,
        ));
    }
    out
}

fn numa(n: &Numa, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if n.nodes.len() <= 1 {
        let what = if n.nodes.is_empty() {
            "no NUMA"
        } else {
            "one node"
        };
        out.extend(row(
            "Nodes",
            &format!("{what}: all RAM is equally close to every CPU"),
            w,
        ));
        return out;
    }
    for node in &n.nodes {
        let mut text = format!("CPUs {}", node.cpus);
        if let Some(t) = node.total {
            text.push_str(&format!(", {}", size(t)));
            if let Some(f) = node.free {
                text.push_str(&format!(" ({} free)", size(f)));
            }
        }
        if let (Some(h), Some(m)) = (node.hit, node.miss) {
            if h + m > 0 {
                text.push_str(&format!(
                    ", {}% of its allocations local",
                    percent(h, h + m)
                ));
            }
        }
        out.extend(row(&format!("Node {}", node.id), &text, w));
    }
    let far: Vec<String> = n
        .nodes
        .iter()
        .flat_map(|a| {
            a.distances
                .iter()
                .enumerate()
                .filter(move |(b, _)| *b as u32 > a.id)
                .map(move |(b, d)| format!("{}→{b} {d}", a.id))
        })
        .collect();
    if !far.is_empty() {
        out.extend(row(
            "Distance",
            &format!(
                "{} (10 is local: 21 costs about twice as much)",
                far.join(", ")
            ),
            w,
        ));
    }
    if let Some(b) = n.balancing {
        let text = if b {
            "on: the kernel moves memory near the CPUs that use it"
        } else {
            "off"
        };
        out.extend(row("Balancing", text, w));
    }
    out
}

fn mem_errors(e: &MemErrors, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if e.controllers.is_empty() {
        out.extend(row(
            "ECC",
            "no memory controller reports errors (EDAC): RAM without ECC, \
             no driver, or a virtual machine",
            w,
        ));
    }
    for c in &e.controllers {
        let mut what: Vec<String> = Vec::new();
        what.extend(c.mem_type.clone());
        what.extend(c.ecc_mode.as_ref().map(|m| format!("{m} ECC")));
        what.extend(c.size.map(size));
        let what = if what.is_empty() {
            String::new()
        } else {
            format!("{}: ", what.join(", "))
        };
        out.extend(row(
            &c.name,
            &format!(
                "{what}{} corrected, {} uncorrectable, since boot",
                count(c.corrected),
                count(c.uncorrected)
            ),
            w,
        ));
    }
    if let Some(b) = e.hardware_corrupted {
        let text = if b == 0 {
            "none: the kernel retired no RAM after an error".to_string()
        } else {
            format!("{} of RAM taken out of use after errors", size(b))
        };
        out.extend(row("Retired", &text, w));
    }
    out
}

fn disk_hardware(d: &DiskHardware, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if d.disks.is_empty() {
        out.extend(row("Disks", "none: no physical or virtual disk", w));
    }
    for disk in &d.disks {
        let kind = match (disk.rotational, disk.transport.as_str()) {
            (_, "virtio") => "virtual disk",
            // USB bridges often claim to spin, flash or not.
            (_, "USB") => "drive",
            (Some(true), _) => "spinning disk",
            (Some(false), _) => "flash",
            (None, _) => "disk",
        };
        let mut parts = Vec::new();
        let mut what = format!("{} {kind}, {}", disk.transport, size(disk.size));
        if let Some(m) = &disk.model {
            what = format!("{m}: {what}");
        }
        parts.push(what);
        if let Some(sched) = &disk.scheduler {
            parts.push(format!("scheduler {sched}"));
        }
        if disk.queues > 1 {
            parts.push(format!("{} hardware queues", disk.queues));
        }
        match disk.write_cache.as_deref() {
            Some("write back") => parts.push("volatile write cache".to_string()),
            Some("write through") => parts.push("no volatile write cache".to_string()),
            _ => {}
        }
        if disk.discard {
            parts.push("TRIM".to_string());
        }
        if let (Some(l), Some(p)) = (disk.logical_block, disk.physical_block) {
            if l != p {
                parts.push(format!("{l}-byte sectors, {p} inside"));
            }
        }
        out.extend(row(&disk.name, &parts.join(" · "), w));
    }
    out
}

/// Average milliseconds per request: `0.08 ms`.
fn per_request(ms: u64, n: u64) -> String {
    if n == 0 {
        return String::new();
    }
    format!(", {:.2} ms each", ms as f64 / n as f64)
}

fn disk_io(d: &DiskIo, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for s in &d.disks {
        let side = |verb: &str, bytes: u64, n: u64, ms: u64| {
            if n == 0 {
                format!("{verb} nothing")
            } else {
                format!(
                    "{verb} {} in {} requests{}",
                    size(bytes),
                    count(n),
                    per_request(ms, n)
                )
            }
        };
        let mut text = format!(
            "{}; {}",
            side("read", s.read_bytes, s.reads, s.read_ms),
            side("wrote", s.write_bytes, s.writes, s.write_ms)
        );
        if let Some(up) = d.uptime_secs.filter(|&u| u > 0) {
            let busy = match percent(s.busy_ms / 1000, up) {
                0 if s.busy_ms > 0 => "under 1".to_string(),
                p => p.to_string(),
            };
            text.push_str(&format!("; busy {busy}% of the time"));
        }
        if let Some(f) = s.flushes.filter(|&f| f > 0) {
            text.push_str(&format!("; {} cache flushes", count(f)));
        }
        out.extend(row(&s.name, &text, w));
    }
    if let Some(up) = d.uptime_secs {
        out.extend(row("Since", &format!("boot, {} ago", duration(up)), w));
    }
    if let (Some(dirty), Some(wb)) = (d.dirty, d.writeback) {
        let mut text = format!(
            "{} not yet written, {} being written",
            size(dirty),
            size(wb)
        );
        if let Some(e) = d.dirty_expire_secs {
            text.push_str(&format!("; data waits up to {e} s in RAM"));
        }
        out.extend(row("Pending", &text, w));
    }
    out
}

fn gpus(g: &Gpus, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if g.cards.is_empty() {
        out.extend(row(
            "GPUs",
            "none the kernel drives (DRM): a server, or a virtual machine \
             without a display",
            w,
        ));
    }
    for c in &g.cards {
        let mut parts: Vec<String> = Vec::new();
        parts.extend(c.vendor.clone());
        parts.extend(c.driver.as_ref().map(|d| format!("driver {d}")));
        parts.extend(c.vram.map(|v| format!("{} of video memory", size(v))));
        if c.boot_vga {
            parts.push("shows the boot screen".to_string());
        }
        out.extend(row(&c.name, &parts.join(", "), w));
        if !c.connectors.is_empty() {
            let on: Vec<String> = c
                .connectors
                .iter()
                .filter(|k| k.connected)
                .map(|k| match &k.mode {
                    Some(m) => format!("{} {m}", k.name),
                    None => k.name.clone(),
                })
                .collect();
            let off: Vec<&str> = c
                .connectors
                .iter()
                .filter(|k| !k.connected)
                .map(|k| k.name.as_str())
                .collect();
            let mut text = if on.is_empty() {
                "no screen connected".to_string()
            } else {
                on.join(", ")
            };
            if !off.is_empty() {
                text.push_str(&format!(" · free: {}", off.join(", ")));
            }
            out.extend(row("Screens", &text, w));
        }
    }
    let mut compute: Vec<String> = Vec::new();
    if !g.render_nodes.is_empty() {
        compute.push(format!("/dev/dri/{}", g.render_nodes.join(", /dev/dri/")));
    }
    if g.kfd {
        compute.push("/dev/kfd (ROCm)".to_string());
    }
    if let Some(v) = &g.nvidia {
        compute.push(format!("NVIDIA kernel module {v}"));
    }
    if !compute.is_empty() {
        out.extend(row("Compute", &compute.join(" · "), w));
    }
    out
}

fn ssh(x: &SshSetup, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if !x.dir {
        out.extend(row("Folder", "no ~/.ssh yet: no keys, no known hosts", w));
    } else if let Some(m) = x.dir_mode.filter(|m| m & 0o022 != 0) {
        out.extend(row(
            "Folder",
            &format!("~/.ssh is writable by others ({m:o}): ssh may refuse its files"),
            w,
        ));
    }
    if x.dir {
        let keys: Vec<String> = x
            .keys
            .iter()
            .map(|k| {
                let mut t = k.name.clone();
                if let Some(kind) = &k.kind {
                    t.push_str(&format!(" ({kind})"));
                }
                if let Some(m) = k.mode.filter(|m| m & 0o077 != 0) {
                    t.push_str(&format!(" open to others ({m:o}): ssh refuses it"));
                }
                t
            })
            .collect();
        let text = if keys.is_empty() {
            "no key pair in ~/.ssh".to_string()
        } else {
            keys.join(", ")
        };
        out.extend(row("Keys", &text, w));
        let mut config = match x.config_hosts {
            0 => "no host entries in ~/.ssh/config".to_string(),
            1 => "1 host entry in ~/.ssh/config".to_string(),
            n => format!("{n} host entries in ~/.ssh/config"),
        };
        if x.jump {
            config.push_str(", with jump hosts");
        }
        out.extend(row("Config", &config, w));
        if let Some(n) = x.known_hosts {
            let hashed = if x.hashed { ", names hashed" } else { "" };
            out.extend(row(
                "Known",
                &format!("{n} host keys trusted in known_hosts{hashed}"),
                w,
            ));
        }
        if let Some(n) = x.authorized_keys {
            out.extend(row(
                "Login",
                &format!(
                    "{n} {} may log in here as you (authorized_keys)",
                    if n == 1 { "key" } else { "keys" }
                ),
                w,
            ));
        }
    }
    out.extend(row(
        "Agent",
        if x.agent {
            "running for this session (SSH_AUTH_SOCK is set)"
        } else {
            "none in this session: keys with a passphrase ask for it every time"
        },
        w,
    ));
    if let Some(s) = &x.server {
        let val = |v: &Option<String>, default: &str| match v {
            Some(v) => v.clone(),
            None => format!("{default} (default)"),
        };
        out.extend(row(
            "Server",
            &format!(
                "port {} · password login {} · root login {}",
                val(&s.port, "22"),
                val(&s.password_auth, "yes"),
                val(&s.root_login, "prohibit-password")
            ),
            w,
        ));
        if s.unreadable {
            out.extend(row(
                "",
                "some sshd files need root to read: values may differ",
                w,
            ));
        }
    }
    out
}

fn git(g: &GitConfig, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    let files = if g.files.is_empty() {
        "none: git uses its defaults".to_string()
    } else {
        g.files.join(", ")
    };
    out.extend(row("Files", &files, w));
    let identity = match (g.name_set, g.email_set) {
        (true, true) => "name and email set".to_string(),
        (n, e) => {
            let missing: Vec<&str> = [(!n).then_some("name"), (!e).then_some("email")]
                .into_iter()
                .flatten()
                .collect();
            format!(
                "{} not set: commits get a guessed identity, or fail",
                missing.join(" and ")
            )
        }
    };
    out.extend(row("Identity", &identity, w));
    for (i, (k, v)) in g.settings.iter().enumerate() {
        let label = if i == 0 { "Settings" } else { "" };
        let mut text = format!("{k} = {v}");
        if k == "credential.helper" && v == "store" {
            text.push_str(" (passwords in plain text in ~/.git-credentials)");
        }
        out.extend(row(label, &text, w));
    }
    if !g.aliases.is_empty() {
        const SHOWN: usize = 8;
        let mut text = g
            .aliases
            .iter()
            .take(SHOWN)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if g.aliases.len() > SHOWN {
            text.push_str(&format!(" and {} more", g.aliases.len() - SHOWN));
        }
        out.extend(row("Aliases", &text, w));
    }
    if g.includes > 0 {
        out.extend(row(
            "Includes",
            &format!(
                "{} more {}, read under conditions (include, includeIf)",
                g.includes,
                if g.includes == 1 { "file" } else { "files" }
            ),
            w,
        ));
    }
    out
}

fn virt(v: &VirtInfo, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    let mut verdict = Vec::new();
    if let (Some(c), Some(h)) = (&v.container, &v.container_hint) {
        verdict.push(format!("a container: {c} (from {h})"));
    }
    if let (Some(m), Some(h)) = (&v.vm, &v.vm_hint) {
        let text = format!("a virtual machine: {m} (from {h})");
        verdict.push(if verdict.is_empty() {
            text
        } else {
            format!("inside {text}")
        });
    }
    let text = if verdict.is_empty() {
        "no container or virtual machine detected".to_string()
    } else {
        format!("this is {}", verdict.join(", "))
    };
    out.extend(row("Here", &text, w));
    if !v.namespaces.is_empty() {
        let own: Vec<&str> = v
            .namespaces
            .iter()
            .filter(|(_, o)| *o)
            .map(|(n, _)| n.as_str())
            .collect();
        let shared: Vec<&str> = v
            .namespaces
            .iter()
            .filter(|(_, o)| !*o)
            .map(|(n, _)| n.as_str())
            .collect();
        let text = match (own.is_empty(), shared.is_empty()) {
            (true, _) => format!("none: the host's namespaces ({})", shared.join(", ")),
            (false, true) => format!("own namespaces: {}", own.join(", ")),
            (false, false) => format!(
                "own namespaces: {} · the host's: {}",
                own.join(", "),
                shared.join(", ")
            ),
        };
        out.extend(row("Isolated", &text, w));
    }
    let mut can = Vec::new();
    if v.kvm {
        can.push("VMs with KVM (/dev/kvm)".to_string());
    }
    if !v.engines.is_empty() {
        can.push(format!("containers with {}", v.engines.join(", ")));
    }
    if !can.is_empty() {
        out.extend(row("Can run", &can.join(" · "), w));
    }
    if v.overlays > 0 {
        out.extend(row(
            "Overlays",
            &format!(
                "{} overlay {}: container layers, usually",
                v.overlays,
                if v.overlays == 1 { "mount" } else { "mounts" }
            ),
            w,
        ));
    }
    out
}

fn threads(t: &Threads, w: usize) -> Vec<Line> {
    let mut out = row(
        "Now",
        &format!(
            "{} threads in {} processes",
            count(t.threads),
            count(t.processes as u64)
        ),
        w,
    );
    if !t.top.is_empty() {
        let top: Vec<String> = t
            .top
            .iter()
            .map(|(pid, name, n)| format!("{name}, PID {pid}: {n}"))
            .collect();
        out.extend(row("Most", &top.join(" · "), w));
    }
    let mut ceil = Vec::new();
    if let Some(m) = t.threads_max {
        ceil.push(format!("{} threads", count(m)));
    }
    if let Some(p) = t.pid_max {
        ceil.push(format!("PIDs up to {}", count(p)));
    }
    if !ceil.is_empty() {
        out.extend(row("Ceiling", &ceil.join(", "), w));
    }
    out
}

/// `1,024 per process, can be raised to 524,288`.
fn limit(l: &Limit, what: &str, unit: impl Fn(u64) -> String) -> String {
    match (l.soft, l.hard) {
        (None, _) => format!("unlimited {what}"),
        (Some(s), Some(h)) if h > s => {
            format!("{} {what}, can be raised to {}", unit(s), unit(h))
        }
        (Some(s), None) => format!("{} {what}, can be raised without limit", unit(s)),
        (Some(s), _) => format!("{} {what}", unit(s)),
    }
}

fn limits(l: &Limits, w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    if let Some(f) = &l.open_files {
        out.extend(row("Files", &limit(f, "open per process", grouped), w));
    }
    if let Some(p) = &l.processes {
        out.extend(row(
            "Processes",
            &limit(p, "for your user, threads included", grouped),
            w,
        ));
    }
    if let Some(s) = &l.stack {
        out.extend(row("Stack", &limit(s, "per thread", size), w));
    }
    if let Some(n) = l.files_open {
        let max = match l.files_max {
            // 2^63 - 1: no ceiling in practice.
            Some(m) if m < 1 << 60 => format!(" of {} allowed", count(m)),
            _ => String::new(),
        };
        out.extend(row(
            "System",
            &format!("{} files open on the whole system{max}", count(n)),
            w,
        ));
    }
    out
}

fn ipc(i: &Ipc, w: usize) -> Vec<Line> {
    let mut out = row(
        "Unix",
        &format!(
            "{} sockets, {} waiting for connections",
            count(i.unix_sockets as u64),
            count(i.unix_listening as u64)
        ),
        w,
    );
    if !i.listening_paths.is_empty() {
        out.extend(row("Servers", &i.listening_paths.join(", "), w));
    }
    if let (Some(u), Some(t)) = (i.shm_used, i.shm_size) {
        out.extend(row(
            "/dev/shm",
            &format!(
                "{} used of {}: POSIX shared memory, in RAM",
                size(u),
                size(t)
            ),
            w,
        ));
    }
    if i.sysv_segments + i.sysv_semaphores + i.sysv_queues > 0 {
        out.extend(row(
            "System V",
            &format!(
                "{} shared memory segments ({}), {} semaphore sets, {} message queues",
                i.sysv_segments,
                size(i.sysv_bytes),
                i.sysv_semaphores,
                i.sysv_queues
            ),
            w,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(1023), "1023 B");
        assert_eq!(size(1536), "1.5 KiB");
        assert_eq!(size(16_264_712 * 1024), "15.5 GiB");
    }

    #[test]
    fn modes_and_durations() {
        assert_eq!(rwx(0o644), "rw-r--r--");
        assert_eq!(rwx(0o750), "rwxr-x---");
        assert_eq!(duration(42 * 60), "42 min");
        assert_eq!(duration(2 * 3600 + 300), "2 h 5 min");
        assert_eq!(duration(273_912), "3 days, 4 h");
    }

    #[test]
    fn numa_and_ecc_on_a_server() {
        let ctx = crate::probes::fixture("server");
        let text = |name: &str| {
            let r = crate::probes::find(name).unwrap().run(&ctx);
            result(&r, 80)
                .iter()
                .map(|l| l.text())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let numa = text("mem.numa");
        assert!(
            numa.contains("Node 1    CPUs 4-7, 62.8 GiB (19.3 GiB free), 100% of"),
            "{numa}"
        );
        assert!(numa.contains("0→1 20"), "{numa}");
        assert!(numa.contains("Balancing on"), "{numa}");
        let ecc = text("mem.errors");
        assert!(
            ecc.contains("mc0       Registered-DDR4, SECDED ECC, 64.0 GiB: 3 corrected"),
            "{ecc}"
        );
        assert!(ecc.contains("4.0 KiB of RAM taken out of use"), "{ecc}");
    }

    #[test]
    fn disks_and_gpus_on_a_server() {
        let ctx = crate::probes::fixture("server");
        let text = |name: &str| {
            let r = crate::probes::find(name).unwrap().run(&ctx);
            result(&r, 80)
                .iter()
                .map(|l| l.text())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let hw = text("disk.hardware");
        assert!(hw.contains("SATA/SAS spinning disk, 3.6 TiB"), "{hw}");
        assert!(hw.contains("512-byte sectors, 4096 inside"), "{hw}");
        let io = text("disk.io");
        assert!(io.contains("sdb       read nothing; wrote nothing"), "{io}");
        let gpu = text("gpu.devices");
        assert!(gpu.contains("card1     NVIDIA, driver nvidia"), "{gpu}");
        assert!(gpu.contains("NVIDIA kernel module 560.35.03"), "{gpu}");
    }

    #[test]
    fn counts() {
        assert_eq!(count(780_774), "780,774");
        assert_eq!(count(999), "999");
        assert_eq!(count(24_793_434), "24.8 million");
        assert_eq!(count(1_200_000_000), "1.2 billion");
    }

    #[test]
    fn percents() {
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(5, 0), 0);
    }
}
