//! `boot.info`: firmware type, Secure Boot, kernel, command line, init,
//! default systemd target and uptime.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "boot.info";

/// The EFI global variable holding the Secure Boot state.
const SECURE_BOOT: &str =
    "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c";

const DEFAULT_TARGET: &[&str] = &[
    "/etc/systemd/system/default.target",
    "/usr/lib/systemd/system/default.target",
    "/lib/systemd/system/default.target",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Firmware {
    Uefi,
    /// Legacy BIOS: /sys/firmware exists but has no `efi`.
    Bios,
    /// Many ARM boards and VMs describe their hardware with a device tree.
    DeviceTree,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BootInfo {
    pub firmware: Option<Firmware>,
    /// `None` when not UEFI, or when the variable can't be read.
    pub secure_boot: Option<bool>,
    pub kernel: Option<String>,
    pub cmdline: Option<String>,
    pub init: Option<String>,
    pub default_target: Option<String>,
    pub uptime_secs: Option<u64>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let firmware = if ctx.exists("/sys/firmware/efi") {
        Some(Firmware::Uefi)
    } else if ctx.exists("/sys/firmware/devicetree") {
        Some(Firmware::DeviceTree)
    } else if ctx.exists("/sys/firmware") {
        Some(Firmware::Bios)
    } else {
        None
    };
    // efivars files start with 4 bytes of attributes, then the value.
    let secure_boot = ctx
        .read_bytes(SECURE_BOOT)
        .ok()
        .and_then(|b| b.get(4).map(|v| *v == 1));
    let data = BootInfo {
        firmware,
        secure_boot,
        kernel: ctx.read_trimmed("/proc/sys/kernel/osrelease"),
        cmdline: ctx.read_trimmed("/proc/cmdline").filter(|c| !c.is_empty()),
        init: ctx.read_trimmed("/proc/1/comm"),
        default_target: DEFAULT_TARGET
            .iter()
            .find_map(|p| ctx.read_link(p))
            .map(|t| t.rsplit('/').next().unwrap_or(&t).to_string()),
        uptime_secs: ctx
            .read_trimmed("/proc/uptime")
            .and_then(|u| u.split('.').next().and_then(|s| s.parse().ok())),
    };
    if data.kernel.is_none() && data.cmdline.is_none() && data.uptime_secs.is_none() {
        let mut r = ProbeResult::new(NAME, Status::Unavailable, None);
        r.notes
            .push("/proc: no kernel, command line or uptime".to_string());
        return r;
    }
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Boot(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn uefi_laptop() {
        let Some(ProbeData::Boot(b)) = probe(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(b.firmware, Some(Firmware::Uefi));
        assert_eq!(b.secure_boot, Some(true));
        assert_eq!(b.init.as_deref(), Some("systemd"));
        assert_eq!(b.default_target.as_deref(), Some("graphical.target"));
        assert_eq!(b.uptime_secs, Some(273_912));
        assert!(b.cmdline.unwrap().contains("root=UUID="));
    }

    #[test]
    fn device_tree_without_systemd() {
        let Some(ProbeData::Boot(b)) = probe(&fixture("old-kernel")).data else {
            panic!()
        };
        assert_eq!(b.firmware, Some(Firmware::DeviceTree));
        assert_eq!(b.secure_boot, None);
        assert_eq!(b.default_target, None);
    }

    #[test]
    fn nothing_readable() {
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
