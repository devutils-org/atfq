//! CPU probes: `cpu.summary` (model, count, load), and the hardware ones
//! for Part I: `cpu.features`, `cpu.topology`, `cpu.caches`, `cpu.freq`,
//! `cpu.vulns`.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "cpu.summary";
pub(crate) const FEATURES: &str = "cpu.features";
pub(crate) const TOPOLOGY: &str = "cpu.topology";
pub(crate) const CACHES: &str = "cpu.caches";
pub(crate) const FREQ: &str = "cpu.freq";
pub(crate) const VULNS: &str = "cpu.vulns";

const SYS_CPU: &str = "/sys/devices/system/cpu";

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CpuFeatures {
    /// `x86_64`, `aarch64`, `riscv64`…
    pub arch: Option<String>,
    /// `Intel`, `AMD`, or the ARM implementer's name.
    pub vendor: Option<String>,
    /// x86-64 microarchitecture level, `x86-64-v3`: what distros build for.
    pub level: Option<String>,
    /// Well-known extensions this CPU has, out of [`NOTABLE`].
    pub notable: Vec<String>,
    /// Every flag the kernel lists.
    pub flags: usize,
}

/// Extensions worth naming, with what they are for.
pub const NOTABLE: &[(&str, &str)] = &[
    ("sse4_2", "SSE 4.2"),
    ("avx2", "AVX2 (256-bit vectors)"),
    ("avx512f", "AVX-512"),
    ("amx_tile", "AMX (matrix units)"),
    ("aes", "AES instructions"),
    ("sha_ni", "SHA instructions"),
    ("vmx", "virtualization (Intel VT-x)"),
    ("svm", "virtualization (AMD-V)"),
    ("asimd", "NEON (128-bit vectors)"),
    ("sve", "SVE (scalable vectors)"),
    ("sve2", "SVE2"),
    ("sha2", "SHA-2 instructions"),
    ("atomics", "LSE atomics"),
];

/// x86-64 levels, as the psABI defines them: each adds to the one before.
const X86_LEVELS: &[(&str, &[&str])] = &[
    (
        "x86-64-v2",
        &["cx16", "lahf_lm", "popcnt", "sse4_1", "sse4_2", "ssse3"],
    ),
    (
        "x86-64-v3",
        &[
            "avx", "avx2", "bmi1", "bmi2", "f16c", "fma", "abm", "movbe", "xsave",
        ],
    ),
    (
        "x86-64-v4",
        &["avx512f", "avx512bw", "avx512cd", "avx512dq", "avx512vl"],
    ),
];

/// ARM `CPU implementer` codes.
const ARM_VENDORS: &[(&str, &str)] = &[
    ("0x41", "Arm"),
    ("0x42", "Broadcom"),
    ("0x43", "Cavium"),
    ("0x46", "Fujitsu"),
    ("0x48", "HiSilicon"),
    ("0x4e", "NVIDIA"),
    ("0x51", "Qualcomm"),
    ("0x61", "Apple"),
    ("0x6d", "Microsoft"),
    ("0xc0", "Ampere"),
];

pub(crate) fn features(ctx: &ProbeCtx) -> ProbeResult {
    let info = match ctx.read("/proc/cpuinfo") {
        Ok(t) => t,
        Err(e) => return ProbeResult::failed(FEATURES, e),
    };
    let field = |k: &str| info.lines().find(|l| key(l) == Some(k)).and_then(value);
    // x86 says `flags`, ARM `Features`, RISC-V `isa`.
    let flags: Vec<String> = field("flags")
        .or_else(|| field("Features"))
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    let has = |f: &str| flags.iter().any(|x| x == f);
    let arch = ctx
        .read_trimmed("/proc/sys/kernel/arch")
        .filter(|a| !a.is_empty())
        .or_else(|| {
            // Kernels before 6.1 don't have /proc/sys/kernel/arch.
            if field("vendor_id").is_some() {
                Some(if has("lm") { "x86_64" } else { "i686" }.to_string())
            } else if field("CPU architecture").as_deref() == Some("8") {
                Some("aarch64".to_string())
            } else {
                field("isa").map(|i| {
                    if i.starts_with("rv64") {
                        "riscv64"
                    } else {
                        "riscv32"
                    }
                    .to_string()
                })
            }
        });
    let vendor = field("vendor_id")
        .map(|v| match v.as_str() {
            "GenuineIntel" => "Intel".to_string(),
            "AuthenticAMD" => "AMD".to_string(),
            _ => v,
        })
        .or_else(|| {
            let code = field("CPU implementer")?;
            let name = ARM_VENDORS
                .iter()
                .find(|(c, _)| c.eq_ignore_ascii_case(&code))
                .map_or(code.clone(), |(_, n)| n.to_string());
            Some(name)
        });
    let level = (arch.as_deref() == Some("x86_64") && !flags.is_empty()).then(|| {
        X86_LEVELS
            .iter()
            .take_while(|(_, need)| need.iter().all(|f| has(f)))
            .last()
            .map_or("x86-64-v1", |(name, _)| name)
            .to_string()
    });
    let notable = NOTABLE
        .iter()
        .filter(|(f, _)| has(f))
        .map(|(f, _)| f.to_string())
        .collect();
    let data = CpuFeatures {
        arch,
        vendor,
        level,
        notable,
        flags: flags.len(),
    };
    let mut r = ProbeResult::new(FEATURES, Status::Ok, Some(ProbeData::Features(data)));
    if flags.is_empty() {
        r.status = Status::Partial;
        r.notes.push("/proc/cpuinfo lists no CPU flags".to_string());
    }
    r
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CpuTopology {
    /// Online logical CPUs: what `nproc` counts, before any limit.
    pub logical: u32,
    /// Physical cores.
    pub cores: u32,
    /// CPU packages (sockets).
    pub packages: u32,
    /// Hardware threads in the busiest core: 2 with SMT (Hyper-Threading).
    pub threads_per_core: u32,
    /// Kinds of cores, fastest first, when there is more than one
    /// (performance and efficiency cores, ARM big.LITTLE).
    pub kinds: Vec<CoreKind>,
    /// CPUs kept away from the scheduler (`isolcpus`), as a list: `2-3`.
    pub isolated: Option<String>,
    pub offline: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoreKind {
    pub name: String,
    /// Logical CPUs of this kind: `0-7`.
    pub cpus: String,
}

/// `cpu0`, `cpu12`… in numeric order.
fn cpu_dirs(ctx: &ProbeCtx) -> Vec<u32> {
    let mut ids: Vec<u32> = ctx
        .list(SYS_CPU)
        .unwrap_or_default()
        .iter()
        .filter_map(|n| n.strip_prefix("cpu")?.parse().ok())
        .collect();
    ids.sort_unstable();
    ids
}

pub(crate) fn topology(ctx: &ProbeCtx) -> ProbeResult {
    let ids = cpu_dirs(ctx);
    if ids.is_empty() {
        return ProbeResult::failed(TOPOLOGY, super::ReadError::Missing(SYS_CPU.to_string()));
    }
    let mut cores = BTreeSet::new();
    let mut packages = BTreeSet::new();
    let mut threads = BTreeMap::new();
    let mut logical = 0;
    let mut capacity: Vec<(u32, u64)> = Vec::new();
    for id in &ids {
        let dir = format!("{SYS_CPU}/cpu{id}/topology");
        // An offline CPU has no topology.
        let Some(package) = ctx.read_trimmed(&format!("{dir}/physical_package_id")) else {
            continue;
        };
        logical += 1;
        // core_cpus_list since 5.7; thread_siblings_list before.
        let core = ctx
            .read_trimmed(&format!("{dir}/core_cpus_list"))
            .or_else(|| ctx.read_trimmed(&format!("{dir}/thread_siblings_list")))
            .unwrap_or_else(|| id.to_string());
        *threads
            .entry((package.clone(), core.clone()))
            .or_insert(0u32) += 1;
        cores.insert((package.clone(), core));
        packages.insert(package);
        if let Some(c) = ctx
            .read_trimmed(&format!("{SYS_CPU}/cpu{id}/cpu_capacity"))
            .and_then(|c| c.parse().ok())
        {
            capacity.push((*id, c));
        }
    }
    // Intel hybrid CPUs list their two kinds of cores as two PMUs.
    let mut kinds: Vec<CoreKind> = [("cpu_core", "performance"), ("cpu_atom", "efficiency")]
        .iter()
        .filter_map(|(pmu, name)| {
            let cpus = ctx.read_trimmed(&format!("/sys/devices/{pmu}/cpus"))?;
            Some(CoreKind {
                name: name.to_string(),
                cpus,
            })
        })
        .collect();
    if kinds.len() < 2 {
        kinds = by_capacity(&capacity);
    }
    let data = CpuTopology {
        logical,
        cores: cores.len() as u32,
        packages: packages.len() as u32,
        threads_per_core: threads.values().copied().max().unwrap_or(1),
        kinds,
        isolated: ctx
            .read_trimmed(&format!("{SYS_CPU}/isolated"))
            .filter(|s| !s.is_empty()),
        offline: ctx
            .read_trimmed(&format!("{SYS_CPU}/offline"))
            .filter(|s| !s.is_empty()),
    };
    let mut r = ProbeResult::new(TOPOLOGY, Status::Ok, Some(ProbeData::Topology(data)));
    if logical == 0 {
        r.status = Status::Partial;
        r.notes.push(format!("{SYS_CPU}/cpu*/topology: not there"));
    }
    r
}

/// ARM chips with several kinds of cores give each a relative capacity
/// (1024 for the fastest). One kind: nothing to say.
fn by_capacity(cpus: &[(u32, u64)]) -> Vec<CoreKind> {
    let mut groups: BTreeMap<u64, Vec<u32>> = BTreeMap::new();
    for (id, c) in cpus {
        groups.entry(*c).or_default().push(*id);
    }
    if groups.len() < 2 {
        return Vec::new();
    }
    groups
        .into_iter()
        .rev()
        .map(|(c, ids)| CoreKind {
            name: format!("capacity {c}"),
            cpus: ranges(&ids),
        })
        .collect()
}

/// `[0, 1, 2, 3, 8]` → `0-3,8`.
pub(crate) fn ranges(ids: &[u32]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < ids.len() {
        let start = ids[i];
        while i + 1 < ids.len() && ids[i + 1] == ids[i] + 1 {
            i += 1;
        }
        out.push(if ids[i] == start {
            start.to_string()
        } else {
            format!("{start}-{}", ids[i])
        });
        i += 1;
    }
    out.join(",")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CpuCaches {
    /// Smallest and closest first.
    pub caches: Vec<Cache>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cache {
    pub level: u8,
    /// `Data`, `Instruction` or `Unified`.
    pub kind: String,
    /// Bytes, for one instance.
    pub size: u64,
    /// Logical CPUs sharing one instance.
    pub shared_by: u32,
    /// How many instances of this cache the chip has.
    pub instances: u32,
    /// Cache line size in bytes: the unit memory moves in.
    pub line: Option<u64>,
}

/// Level, kind, size and CPUs sharing one instance; the map's value holds the
/// line size and the distinct sets of CPUs.
type CacheKey = (u8, String, u64, u32);

pub(crate) fn caches(ctx: &ProbeCtx) -> ProbeResult {
    let mut found: BTreeMap<CacheKey, (Option<u64>, BTreeSet<String>)> = BTreeMap::new();
    for id in cpu_dirs(ctx) {
        let dir = format!("{SYS_CPU}/cpu{id}/cache");
        for index in ctx.list(&dir).unwrap_or_default() {
            if !index.starts_with("index") {
                continue;
            }
            let get = |f: &str| ctx.read_trimmed(&format!("{dir}/{index}/{f}"));
            let (Some(level), Some(kind), Some(size)) = (
                get("level").and_then(|l| l.parse().ok()),
                get("type"),
                get("size").as_deref().and_then(parse_size),
            ) else {
                continue;
            };
            let shared = get("shared_cpu_list").unwrap_or_else(|| id.to_string());
            let line = get("coherency_line_size").and_then(|l| l.parse().ok());
            let entry = found
                .entry((level, kind, size, count_range(&shared)))
                .or_insert((line, BTreeSet::new()));
            entry.1.insert(shared);
        }
    }
    if found.is_empty() {
        let mut r = ProbeResult::new(CACHES, Status::Unavailable, None);
        r.notes.push(format!("{SYS_CPU}/cpu*/cache: not there"));
        return r;
    }
    let mut caches: Vec<Cache> = found
        .into_iter()
        .map(|((level, kind, size, shared_by), (line, sets))| Cache {
            level,
            kind,
            size,
            shared_by,
            instances: sets.len() as u32,
            line,
        })
        .collect();
    // Data before instruction, as lscpu does.
    caches.sort_by(|a, b| (a.level, &a.kind).cmp(&(b.level, &b.kind)));
    ProbeResult::new(
        CACHES,
        Status::Ok,
        Some(ProbeData::Caches(CpuCaches { caches })),
    )
}

/// `48K` → 49152; sysfs writes cache sizes in KiB.
fn parse_size(s: &str) -> Option<u64> {
    let (n, unit) = match s.char_indices().find(|(_, c)| !c.is_ascii_digit()) {
        Some((i, _)) => s.split_at(i),
        None => (s, ""),
    };
    let n: u64 = n.parse().ok()?;
    match unit.trim() {
        "" => Some(n),
        "K" => Some(n * 1024),
        "M" => Some(n * 1024 * 1024),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CpuFreq {
    /// cpufreq driver: `intel_pstate`, `amd-pstate-epp`, `cppc_cpufreq`…
    pub driver: Option<String>,
    pub governor: Option<String>,
    /// The hardware's range, in kHz, over all CPUs.
    pub min_khz: Option<u64>,
    pub max_khz: Option<u64>,
    /// Current frequency of each CPU, in kHz.
    pub current_khz: Vec<u64>,
    /// Turbo (boost) allowed.
    pub boost: Option<bool>,
    /// cpuidle driver and CPU 0's idle states with how often each was used.
    pub idle_driver: Option<String>,
    pub idle_states: Vec<(String, u64)>,
    /// Hottest thermal zone: its type and degrees Celsius.
    pub hottest: Option<(String, f64)>,
}

pub(crate) fn freq(ctx: &ProbeCtx) -> ProbeResult {
    let ids = cpu_dirs(ctx);
    let get = |id: u32, f: &str| ctx.read_trimmed(&format!("{SYS_CPU}/cpu{id}/cpufreq/{f}"));
    let num = |id: u32, f: &str| get(id, f).and_then(|v| v.parse::<u64>().ok());
    let first = ids
        .iter()
        .copied()
        .find(|&id| get(id, "scaling_driver").is_some());
    let current_khz: Vec<u64> = ids
        .iter()
        .filter_map(|&id| num(id, "scaling_cur_freq"))
        .collect();
    let min_khz = ids
        .iter()
        .filter_map(|&id| num(id, "cpuinfo_min_freq"))
        .min();
    let max_khz = ids
        .iter()
        .filter_map(|&id| num(id, "cpuinfo_max_freq"))
        .max();
    // acpi-cpufreq and amd-pstate say `boost`; intel_pstate says the opposite.
    let boost = ctx
        .read_trimmed(&format!("{SYS_CPU}/cpufreq/boost"))
        .map(|b| b == "1")
        .or_else(|| {
            ctx.read_trimmed(&format!("{SYS_CPU}/intel_pstate/no_turbo"))
                .map(|b| b == "0")
        });
    let idle_driver = ctx
        .read_trimmed(&format!("{SYS_CPU}/cpuidle/current_driver"))
        .filter(|d| d != "none");
    let idle_dir = ids
        .first()
        .map(|id| format!("{SYS_CPU}/cpu{id}/cpuidle"))
        .unwrap_or_default();
    let mut idle_states: Vec<(u32, String, u64)> = ctx
        .list(&idle_dir)
        .unwrap_or_default()
        .iter()
        .filter_map(|s| {
            let n: u32 = s.strip_prefix("state")?.parse().ok()?;
            let name = ctx.read_trimmed(&format!("{idle_dir}/{s}/name"))?;
            let usage = ctx
                .read_trimmed(&format!("{idle_dir}/{s}/usage"))
                .and_then(|u| u.parse().ok())
                .unwrap_or(0);
            Some((n, name, usage))
        })
        .collect();
    idle_states.sort();
    let hottest = ctx
        .list("/sys/class/thermal")
        .unwrap_or_default()
        .iter()
        .filter(|z| z.starts_with("thermal_zone"))
        .filter_map(|z| {
            let dir = format!("/sys/class/thermal/{z}");
            let milli: i64 = ctx.read_trimmed(&format!("{dir}/temp"))?.parse().ok()?;
            let kind = ctx
                .read_trimmed(&format!("{dir}/type"))
                .unwrap_or(z.clone());
            // Some sensors report nonsense when absent.
            (milli > 0 && milli < 150_000).then(|| (kind, milli as f64 / 1000.0))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1));
    let data = CpuFreq {
        driver: first.and_then(|id| get(id, "scaling_driver")),
        governor: first.and_then(|id| get(id, "scaling_governor")),
        min_khz,
        max_khz,
        current_khz,
        boost,
        idle_driver,
        idle_states: idle_states.into_iter().map(|(_, n, u)| (n, u)).collect(),
        hottest,
    };
    let nothing = data.driver.is_none()
        && data.current_khz.is_empty()
        && data.idle_driver.is_none()
        && data.hottest.is_none();
    if nothing {
        let mut r = ProbeResult::new(FREQ, Status::Unavailable, None);
        r.notes.push(
            "no cpufreq, cpuidle or thermal data: a virtual machine, or the drivers aren't loaded"
                .to_string(),
        );
        return r;
    }
    ProbeResult::new(FREQ, Status::Ok, Some(ProbeData::Freq(data)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CpuVulns {
    /// Flaws this CPU has, with no mitigation: name and the kernel's words.
    pub vulnerable: Vec<(String, String)>,
    /// Flaws worked around: name and how.
    pub mitigated: Vec<(String, String)>,
    /// Flaws this CPU doesn't have.
    pub not_affected: Vec<String>,
    /// `mitigations=off` is on the kernel command line.
    pub mitigations_off: bool,
}

pub(crate) fn vulns(ctx: &ProbeCtx) -> ProbeResult {
    let dir = format!("{SYS_CPU}/vulnerabilities");
    let names = match ctx.list(&dir) {
        Ok(n) => n,
        Err(e) => return ProbeResult::failed(VULNS, e),
    };
    let mut data = CpuVulns {
        vulnerable: Vec::new(),
        mitigated: Vec::new(),
        not_affected: Vec::new(),
        mitigations_off: ctx
            .read_trimmed("/proc/cmdline")
            .is_some_and(|c| c.split_whitespace().any(|w| w == "mitigations=off")),
    };
    for name in names {
        let Some(state) = ctx.read_trimmed(&format!("{dir}/{name}")) else {
            continue;
        };
        if state == "Not affected" {
            data.not_affected.push(name);
        } else if let Some(how) = state.strip_prefix("Mitigation: ") {
            data.mitigated.push((name, how.to_string()));
        } else {
            data.vulnerable.push((name, state));
        }
    }
    ProbeResult::new(VULNS, Status::Ok, Some(ProbeData::Vulns(data)))
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
    fn x86_features_and_level() {
        let Some(ProbeData::Features(f)) = features(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(f.arch.as_deref(), Some("x86_64"));
        assert_eq!(f.vendor.as_deref(), Some("Intel"));
        assert_eq!(f.level.as_deref(), Some("x86-64-v4"));
        assert!(f.notable.contains(&"avx512f".to_string()));
        assert!(f.notable.contains(&"vmx".to_string()));
        // A Xeon fixture with only the v2 flags.
        let Some(ProbeData::Features(f)) = features(&fixture("server")).data else {
            panic!()
        };
        assert_eq!(f.level.as_deref(), Some("x86-64-v2"));
    }

    #[test]
    fn arm_features_without_arch_file() {
        let Some(ProbeData::Features(f)) = features(&fixture("old-kernel")).data else {
            panic!()
        };
        assert_eq!(f.arch.as_deref(), Some("aarch64"));
        assert_eq!(f.vendor.as_deref(), Some("Arm"));
        assert_eq!(f.level, None);
        assert_eq!(f.notable, ["asimd"]);
        assert_eq!(features(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn topology_counts_cores_and_threads() {
        let Some(ProbeData::Topology(t)) = topology(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(
            (t.logical, t.cores, t.packages, t.threads_per_core),
            (8, 4, 1, 2)
        );
        assert!(t.kinds.is_empty());
        assert_eq!((t.isolated, t.offline), (None, None));
        let Some(ProbeData::Topology(t)) = topology(&fixture("server")).data else {
            panic!()
        };
        assert_eq!(
            (t.logical, t.cores, t.packages, t.threads_per_core),
            (8, 4, 2, 2)
        );
        assert_eq!(topology(&fixture("old-kernel")).status, Status::Unavailable);
    }

    #[test]
    fn big_little_by_capacity() {
        let kinds = by_capacity(&[(0, 446), (1, 446), (2, 1024), (3, 1024)]);
        let got: Vec<(&str, &str)> = kinds
            .iter()
            .map(|k| (k.name.as_str(), k.cpus.as_str()))
            .collect();
        assert_eq!(got, [("capacity 1024", "2-3"), ("capacity 446", "0-1")]);
        assert!(by_capacity(&[(0, 1024), (1, 1024)]).is_empty());
    }

    #[test]
    fn caches_are_grouped() {
        let Some(ProbeData::Caches(c)) = caches(&fixture("laptop")).data else {
            panic!()
        };
        let got: Vec<(u8, &str, u64, u32, u32)> = c
            .caches
            .iter()
            .map(|c| (c.level, c.kind.as_str(), c.size, c.shared_by, c.instances))
            .collect();
        assert_eq!(
            got,
            [
                (1, "Data", 48 * 1024, 2, 4),
                (1, "Instruction", 32 * 1024, 2, 4),
                (2, "Unified", 1280 * 1024, 2, 4),
                (3, "Unified", 12 * 1024 * 1024, 8, 1),
            ]
        );
        assert_eq!(c.caches[0].line, Some(64));
        assert_eq!(caches(&fixture("empty")).status, Status::Unavailable);
        assert_eq!(parse_size("64M"), Some(64 << 20));
        assert_eq!(parse_size("1x"), None);
    }

    #[test]
    fn frequencies_idle_and_heat() {
        let Some(ProbeData::Freq(f)) = freq(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(f.driver.as_deref(), Some("intel_pstate"));
        assert_eq!((f.min_khz, f.max_khz), (Some(400_000), Some(4_700_000)));
        assert_eq!(f.current_khz.len(), 8);
        assert_eq!(f.boost, Some(true));
        assert_eq!(f.idle_driver.as_deref(), Some("intel_idle"));
        assert_eq!(f.idle_states[0].0, "POLL");
        assert_eq!(f.idle_states.len(), 5);
        assert_eq!(f.hottest, Some(("x86_pkg_temp".to_string(), 58.0)));
        assert_eq!(freq(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn vulnerabilities_sorted_by_state() {
        let Some(ProbeData::Vulns(v)) = vulns(&fixture("laptop")).data else {
            panic!()
        };
        assert!(v.vulnerable.is_empty());
        assert_eq!(v.mitigated.len(), 4);
        assert_eq!(v.not_affected.len(), 10);
        assert!(!v.mitigations_off);
        assert_eq!(vulns(&fixture("empty")).status, Status::Unavailable);
    }

    #[test]
    fn ranges() {
        assert_eq!(count_range("0-7"), 8);
        assert_eq!(count_range("0-3,8,10-11"), 7);
        assert_eq!(count_range(""), 0);
        assert_eq!(super::ranges(&[0, 1, 2, 3, 8, 10, 11]), "0-3,8,10-11");
        assert_eq!(super::ranges(&[]), "");
    }
}
