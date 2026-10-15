//! `gpu.devices`: graphics cards the kernel drives (DRM), their drivers,
//! screens, and the device files compute stacks need.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "gpu.devices";

const DRM: &str = "/sys/class/drm";

/// PCI vendor ids of GPU makers and virtual GPUs.
const VENDORS: &[(&str, &str)] = &[
    ("0x8086", "Intel"),
    ("0x1002", "AMD"),
    ("0x10de", "NVIDIA"),
    ("0x1af4", "virtio"),
    ("0x1234", "QEMU"),
    ("0x15ad", "VMware"),
    ("0x1414", "Microsoft"),
    ("0x1a03", "ASPEED"),
    ("0x102b", "Matrox"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gpus {
    pub cards: Vec<GpuCard>,
    /// Render nodes (`renderD128`): what compute and offscreen drawing open.
    pub render_nodes: Vec<String>,
    /// /dev/kfd exists: AMD's compute driver, used by ROCm.
    pub kfd: bool,
    /// Version of NVIDIA's kernel module, when it's loaded.
    pub nvidia: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GpuCard {
    /// `card0`.
    pub name: String,
    pub vendor: Option<String>,
    /// Kernel driver: `i915`, `xe`, `amdgpu`, `nouveau`, `nvidia`…
    pub driver: Option<String>,
    /// The firmware used it to show the boot screen.
    pub boot_vga: bool,
    /// Dedicated video memory (amdgpu reports it).
    pub vram: Option<u64>,
    pub connectors: Vec<Connector>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Connector {
    /// `eDP-1`, `HDMI-A-1`, `DP-2`.
    pub name: String,
    pub connected: bool,
    /// Preferred mode of the screen: `1920x1200`.
    pub mode: Option<String>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let names = match ctx.list(DRM) {
        Ok(n) => n,
        Err(e) => return ProbeResult::failed(NAME, e),
    };
    let is_card = |n: &str| {
        n.strip_prefix("card")
            .is_some_and(|i| !i.is_empty() && i.chars().all(|c| c.is_ascii_digit()))
    };
    let mut cards: Vec<GpuCard> = names
        .iter()
        .filter(|n| is_card(n))
        .map(|card| {
            let dev = format!("{DRM}/{card}/device");
            let get = |f: &str| ctx.read_trimmed(&format!("{dev}/{f}"));
            let vendor = get("vendor").map(|v| {
                VENDORS
                    .iter()
                    .find(|(id, _)| id.eq_ignore_ascii_case(&v))
                    .map_or(v.clone(), |(_, n)| n.to_string())
            });
            let driver = ctx
                .read_link(&format!("{dev}/driver"))
                .and_then(|l| l.rsplit('/').next().map(str::to_string))
                .or_else(|| {
                    get("uevent")?
                        .lines()
                        .find_map(|l| l.strip_prefix("DRIVER=").map(str::to_string))
                });
            // Connectors are siblings named after the card: `card0-eDP-1`.
            let prefix = format!("{card}-");
            let connectors = names
                .iter()
                .filter_map(|n| {
                    let name = n.strip_prefix(&prefix)?;
                    let dir = format!("{DRM}/{n}");
                    let status = ctx.read_trimmed(&format!("{dir}/status"))?;
                    let mode = ctx
                        .read(&format!("{dir}/modes"))
                        .ok()
                        .and_then(|m| m.lines().next().map(str::to_string));
                    Some(Connector {
                        name: name.to_string(),
                        connected: status == "connected",
                        mode,
                    })
                })
                .collect();
            GpuCard {
                name: card.clone(),
                vendor,
                driver,
                boot_vga: get("boot_vga").as_deref() == Some("1"),
                vram: get("mem_info_vram_total").and_then(|v| v.parse().ok()),
                connectors,
            }
        })
        .collect();
    cards.sort_by_key(|c| c.name[4..].parse::<u32>().unwrap_or(0));
    let data = Gpus {
        cards,
        render_nodes: names
            .iter()
            .filter(|n| n.starts_with("renderD"))
            .cloned()
            .collect(),
        kfd: ctx.exists("/dev/kfd"),
        // `NVRM version: NVIDIA UNIX Open Kernel Module for x86_64  560.35.03  …`
        nvidia: ctx.read("/proc/driver/nvidia/version").ok().and_then(|v| {
            let line = v.lines().next()?;
            line.split_whitespace()
                .find(|w| w.contains('.') && w.chars().all(|c| c.is_ascii_digit() || c == '.'))
                .map(str::to_string)
        }),
    };
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Gpus(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    fn gpus(name: &str) -> Gpus {
        let Some(ProbeData::Gpus(g)) = probe(&fixture(name)).data else {
            panic!()
        };
        g
    }

    #[test]
    fn laptop_with_a_screen() {
        let g = gpus("laptop");
        assert_eq!(g.cards.len(), 1);
        let c = &g.cards[0];
        assert_eq!(c.vendor.as_deref(), Some("Intel"));
        assert_eq!(c.driver.as_deref(), Some("i915"));
        assert!(c.boot_vga);
        let screens: Vec<(&str, bool, Option<&str>)> = c
            .connectors
            .iter()
            .map(|k| (k.name.as_str(), k.connected, k.mode.as_deref()))
            .collect();
        assert_eq!(
            screens,
            [
                ("DP-1", false, None),
                ("HDMI-A-1", false, None),
                ("eDP-1", true, Some("1920x1200")),
            ]
        );
        assert_eq!(g.render_nodes, ["renderD128"]);
        assert!(!g.kfd);
        assert_eq!(g.nvidia, None);
    }

    #[test]
    fn server_with_bmc_and_nvidia() {
        let g = gpus("server");
        let got: Vec<(Option<&str>, Option<&str>)> = g
            .cards
            .iter()
            .map(|c| (c.vendor.as_deref(), c.driver.as_deref()))
            .collect();
        assert_eq!(
            got,
            [
                (Some("ASPEED"), Some("ast")),
                (Some("NVIDIA"), Some("nvidia"))
            ]
        );
        assert_eq!(g.nvidia.as_deref(), Some("560.35.03"));
    }

    #[test]
    fn no_drm() {
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
