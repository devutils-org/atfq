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
use crate::probes::{Interrupts, KernelOverview, NetLayers, VirtualMemory};

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
        _ => {
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
    }
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
