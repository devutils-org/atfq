//! `boot.info`: firmware type, Secure Boot, kernel, command line, init,
//! default systemd target and uptime.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "boot.info";

/// The EFI global variable holding the Secure Boot state.
const SECURE_BOOT: &str =
    "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c";

/// systemd-boot writes its name and version in this EFI variable.
const LOADER_INFO: &str =
    "/sys/firmware/efi/efivars/LoaderInfo-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f";

const GRUB_CONFIGS: &[&str] = &["/boot/grub/grub.cfg", "/boot/grub2/grub.cfg"];

/// Where unified kernel images live, on the EFI System Partition.
const UKI_DIRS: &[&str] = &["/boot/efi/EFI/Linux", "/efi/EFI/Linux", "/boot/EFI/Linux"];

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
    /// `systemd-boot 256.4`, `GRUB`: what started this kernel, best guess.
    pub bootloader: Option<String>,
    /// Unified kernel images (`.efi`) found on the EFI partition.
    pub ukis: u32,
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
        bootloader: ctx
            .read_bytes(LOADER_INFO)
            .ok()
            .and_then(|b| utf16_var(&b))
            .or_else(|| {
                GRUB_CONFIGS
                    .iter()
                    .any(|c| ctx.exists(c))
                    .then(|| "GRUB".to_string())
            }),
        ukis: UKI_DIRS
            .iter()
            .filter_map(|d| ctx.list(d).ok())
            .flatten()
            .filter(|f| f.ends_with(".efi"))
            .count() as u32,
    };
    if data.kernel.is_none() && data.cmdline.is_none() && data.uptime_secs.is_none() {
        let mut r = ProbeResult::new(NAME, Status::Unavailable, None);
        r.notes
            .push("/proc: no kernel, command line or uptime".to_string());
        return r;
    }
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Boot(data)))
}

/// An EFI string variable: 4 bytes of attributes, then UTF-16LE up to a NUL.
fn utf16_var(bytes: &[u8]) -> Option<String> {
    let units: Vec<u16> = bytes
        .get(4..)?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .take_while(|&u| u != 0)
        .collect();
    let s = String::from_utf16(&units).ok()?;
    (!s.is_empty()).then_some(s)
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
        assert_eq!(b.bootloader.as_deref(), Some("systemd-boot 256.4"));
        assert_eq!(b.ukis, 1);
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
