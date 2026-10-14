//! `cpu.summary`: model, number of CPUs, load average, frequency governor.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "cpu.summary";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CpuSummary {
    /// `model name` on x86; ARM chips often don't report one.
    pub model: Option<String>,
    /// Logical CPUs the kernel uses (cores × threads per core).
    pub logical: u32,
    /// Online CPUs as the kernel lists them: `0-7`.
    pub online: Option<String>,
    /// Load average over 1, 5 and 15 minutes.
    pub load: Option<[f64; 3]>,
    /// Tasks (threads) running or ready to run right now.
    pub running: Option<u32>,
    /// All tasks (threads) on the system.
    pub tasks: Option<u32>,
    /// cpufreq governor of CPU 0: `powersave`, `performance`, `schedutil`…
    pub governor: Option<String>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let info = match ctx.read("/proc/cpuinfo") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(NAME, e),
    };
    let online = ctx.read_trimmed("/sys/devices/system/cpu/online");
    let processors = info.lines().filter(|l| key(l) == Some("processor")).count() as u32;
    let logical = if processors > 0 {
        processors
    } else {
        online.as_deref().map_or(0, count_range)
    };
    let model = ["model name", "Model", "Hardware", "cpu model"]
        .iter()
        .find_map(|k| info.lines().find(|l| key(l) == Some(*k)).and_then(value));

    let mut notes = Vec::new();
    let loadavg = ctx.read_trimmed("/proc/loadavg");
    let fields: Vec<&str> = loadavg
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .collect();
    let load = (fields.len() >= 3)
        .then(|| {
            let f = |i: usize| fields[i].parse::<f64>().ok();
            Some([f(0)?, f(1)?, f(2)?])
        })
        .flatten();
    if load.is_none() {
        notes.push("/proc/loadavg: can't read it".to_string());
    }
    let (running, tasks) = fields
        .get(3)
        .and_then(|rt| rt.split_once('/'))
        .map_or((None, None), |(r, t)| (r.parse().ok(), t.parse().ok()));

    let data = CpuSummary {
        model,
        logical,
        online,
        load,
        running,
        tasks,
        governor: ctx.read_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
    };
    let status = if notes.is_empty() {
        Status::Ok
    } else {
        Status::Partial
    };
    let mut r = ProbeResult::new(NAME, status, Some(ProbeData::Cpu(data)));
    r.notes = notes;
    r
}

/// `model name\t: Intel…` → `model name`.
fn key(line: &str) -> Option<&str> {
    line.split_once(':').map(|(k, _)| k.trim())
}

fn value(line: &str) -> Option<String> {
    let v = line.split_once(':')?.1.trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// `0-3,8` → 5.
fn count_range(list: &str) -> u32 {
    list.split(',')
        .filter_map(|part| match part.trim().split_once('-') {
            Some((a, b)) => Some(b.parse::<u32>().ok()? - a.parse::<u32>().ok()? + 1),
            None => part.trim().parse::<u32>().ok().map(|_| 1),
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn reads_x86_cpuinfo_and_load() {
        let r = probe(&fixture("laptop"));
        assert_eq!(r.status, Status::Ok, "{:?}", r.notes);
        let Some(ProbeData::Cpu(c)) = r.data else {
            panic!()
        };
        assert_eq!(
            c.model.as_deref(),
            Some("11th Gen Intel(R) Core(TM) i7-1165G7 @ 2.80GHz")
        );
        assert_eq!(c.logical, 8);
        assert_eq!(c.load, Some([1.25, 0.9, 0.62]));
        assert_eq!((c.running, c.tasks), (Some(2), Some(1043)));
        assert_eq!(c.governor.as_deref(), Some("powersave"));
    }

    #[test]
    fn arm_without_model_and_loadavg() {
        let r = probe(&fixture("old-kernel"));
        assert_eq!(r.status, Status::Partial);
        let Some(ProbeData::Cpu(c)) = r.data else {
            panic!()
        };
        assert_eq!(c.model, None);
        assert_eq!(c.logical, 4);
        assert_eq!(c.governor, None);
    }

    #[test]
    fn missing_cpuinfo() {
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn ranges() {
        assert_eq!(count_range("0-7"), 8);
        assert_eq!(count_range("0-3,8,10-11"), 7);
        assert_eq!(count_range(""), 0);
    }
}
