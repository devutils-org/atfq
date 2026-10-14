//! The "On your machine" block: probe results as short lines.

use super::text::{wrap, Kind, Prefix, Span, Style};
use super::{INDENT, WIDTH};
use crate::probes::{
    BlockDevice, DiskTree, MemSummary, Mount, ProbeData, ProbeResult, ProcInfo, ProcStates, Status,
};

/// Width of the label column (`RAM`, `Swap`, `Zombies`…).
const LABEL: usize = 9;

/// Most processes listed under Zombies or Waiting.
const SHOW_PROCS: usize = 5;

pub(crate) fn block(results: &[ProbeResult], st: Style) -> Vec<String> {
    let mut out = Vec::new();
    for (i, r) in results.iter().enumerate() {
        if i > 0 {
            out.push(String::new());
        }
        out.extend(result(r, st));
    }
    out
}

pub(crate) fn result(r: &ProbeResult, st: Style) -> Vec<String> {
    let pad = " ".repeat(INDENT);
    let mut out = match (&r.status, &r.data) {
        (Status::NeedsRoot, _) => vec![format!("{pad}{}", st.warn("needs root"))],
        (Status::Unavailable, None) => {
            vec![format!("{pad}{}", st.dim("not available on this system"))]
        }
        (Status::Unsupported, _) => vec![format!("{pad}{}", st.dim("not supported on this OS"))],
        (_, Some(ProbeData::Mem(m))) => mem(m, st),
        (_, Some(ProbeData::Disk(d))) => disk(d, st),
        (_, Some(ProbeData::Proc(p))) => procs(p, st),
        (_, None) => Vec::new(),
    };
    for note in &r.notes {
        let spans = [Span::new(format!("note: {note}"), Kind::Plain)];
        let p = Prefix::spaces(INDENT);
        out.extend(wrap(&spans, &p, &p, WIDTH, st).iter().map(|l| st.dim(l)));
    }
    out
}

/// `  Label    text`, wrapped under the text.
fn row(label: &str, text: &str, st: Style) -> Vec<String> {
    let first = Prefix {
        text: format!(
            "{}{}",
            " ".repeat(INDENT),
            st.bold(&format!("{label:<LABEL$}"))
        ),
        width: INDENT + LABEL,
    };
    let rest = Prefix::spaces(INDENT + LABEL);
    let mut lines = wrap(&[Span::new(text, Kind::Plain)], &first, &rest, WIDTH, st);
    if lines.is_empty() {
        lines.push(first.text);
    }
    lines
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

fn mem(m: &MemSummary, st: Style) -> Vec<String> {
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
    out.extend(row("RAM", &ram, st));
    let cache = if m.shared > 0 {
        format!(
            "{} in buff/cache, {} of it shared (tmpfs)",
            size(m.buff_cache),
            size(m.shared)
        )
    } else {
        format!("{} in buff/cache", size(m.buff_cache))
    };
    out.extend(row("Cache", &cache, st));
    let mut swap = if m.swap_total == 0 {
        "none".to_string()
    } else {
        let used = m.swap_total.saturating_sub(m.swap_free);
        format!("{} used of {}", size(used), size(m.swap_total))
    };
    if let Some(s) = m.swappiness {
        swap.push_str(&format!(" · swappiness {s}"));
    }
    out.extend(row("Swap", &swap, st));
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

fn procs(p: &ProcStates, st: Style) -> Vec<String> {
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
        st,
    );

    let zombies = p.counts.get("Z").copied().unwrap_or(0) as usize;
    if zombies == 0 {
        out.extend(row("Zombies", "none", st));
    } else {
        out.extend(proc_list("Zombies", &p.zombies, zombies, st));
    }
    let waiting = p.counts.get("D").copied().unwrap_or(0) as usize;
    if waiting > 0 {
        out.extend(proc_list("Waiting", &p.waiting, waiting, st));
    }
    out
}

fn proc_list(label: &str, list: &[ProcInfo], total: usize, st: Style) -> Vec<String> {
    let mut out = Vec::new();
    for (i, p) in list.iter().take(SHOW_PROCS).enumerate() {
        let parent = match &p.parent {
            Some(name) => format!("parent {} {name}", p.ppid),
            None => format!("parent {}", p.ppid),
        };
        let text = format!("PID {} {}, {parent}", p.pid, p.comm);
        out.extend(row(if i == 0 { label } else { "" }, &text, st));
    }
    let shown = list.len().min(SHOW_PROCS);
    if total > shown {
        out.extend(row("", &format!("and {} more", total - shown), st));
    }
    out
}

/// One line of the disk table.
struct Line {
    depth: usize,
    name: String,
    size: String,
    fstype: String,
    mount: String,
    detail: String,
}

fn disk(d: &DiskTree, st: Style) -> Vec<String> {
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
                None => lines.push(Line {
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

    let w = |f: &dyn Fn(&Line) -> usize| lines.iter().map(f).max().unwrap_or(0);
    let name_w = w(&|l| l.depth * 2 + l.name.chars().count());
    let size_w = w(&|l| l.size.chars().count());
    let fs_w = w(&|l| l.fstype.chars().count());
    let mount_w = w(&|l| l.mount.chars().count());

    let pad = " ".repeat(INDENT);
    let mut out = Vec::new();
    for l in &lines {
        let name = format!("{:<name_w$}", format!("{}{}", "  ".repeat(l.depth), l.name));
        let styled = if l.depth == 0 {
            st.bold(&name)
        } else {
            name.clone()
        };
        let mut cells = format!("  {:>size_w$}", l.size);
        if !l.mount.is_empty() {
            cells.push_str(&format!("  {:<fs_w$}  {:<mount_w$}", l.fstype, l.mount));
        }
        let used = INDENT + name_w + cells.chars().count();
        let mut line = format!("{pad}{styled}{}", cells.trim_end());
        if !l.detail.is_empty() {
            if used + 2 + l.detail.chars().count() <= WIDTH {
                line = format!("{pad}{styled}{cells}  {}", l.detail);
            } else {
                out.push(line);
                line = format!("{}{}", " ".repeat(INDENT + name_w + 2), l.detail);
            }
        }
        out.push(line);
    }
    if d.hidden > 0 {
        let what = if d.hidden == 1 { "device" } else { "devices" };
        out.push(st.dim(&format!(
            "{}{} loop or empty {what} not shown",
            " ".repeat(INDENT),
            d.hidden
        )));
    }
    if out.is_empty() {
        out.push(format!("{}no disks found", " ".repeat(INDENT)));
    }
    out
}

fn device_line(dev: &BlockDevice) -> Line {
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
    Line {
        depth: 0,
        name: dev.name.clone(),
        size: size(dev.size),
        fstype: String::new(),
        mount: String::new(),
        detail: detail.join(" · "),
    }
}

fn mount_line(depth: usize, name: String, size_text: String, m: &Mount, count: usize) -> Line {
    let mut mount = m.point.clone();
    if count > 1 {
        mount.push_str(&format!(" (+{})", count - 1));
    }
    let detail = match m.space {
        Some(s) => format!(
            "{} free ({}%)",
            size(s.available),
            percent(s.available, s.total)
        ),
        None => String::new(),
    };
    Line {
        depth,
        name,
        size: size_text,
        fstype: m.fstype.clone(),
        mount,
        detail,
    }
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
    fn percents() {
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(5, 0), 0);
    }
}
