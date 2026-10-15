//! `crypto.setup`: the certificate authorities this system trusts, the
//! kernel's AES and SHA-256 implementations, the hardware random number
//! generator, the TPM, and the system-wide crypto policy.

use serde::Serialize;

use super::{ProbeCtx, ProbeData, ProbeResult, Status};

pub(crate) const NAME: &str = "crypto.setup";

/// CA bundles, by distro family.
const BUNDLES: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/ssl/ca-bundle.pem",
    "/etc/ssl/cert.pem",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CryptoInfo {
    /// The CA bundle programs read, and how many certificates it holds.
    pub ca_bundle: Option<String>,
    pub ca_count: Option<u32>,
    /// Kernel implementations picked for AES and SHA-256 (highest priority),
    /// and whether they use CPU instructions made for it.
    pub aes: Option<String>,
    pub aes_hw: bool,
    pub sha256: Option<String>,
    /// Hardware random number generator feeding the kernel, if any.
    pub hw_rng: Option<String>,
    /// TPM version: `2`.
    pub tpm: Option<String>,
    /// Fedora and RHEL's system-wide policy: `DEFAULT`, `FUTURE`, `LEGACY`.
    pub policy: Option<String>,
}

pub(crate) fn probe(ctx: &ProbeCtx) -> ProbeResult {
    let bundle = BUNDLES.iter().find(|b| ctx.exists(b));
    let ca_count = bundle.and_then(|b| {
        ctx.read(b)
            .ok()
            .map(|t| t.matches("-----BEGIN CERTIFICATE-----").count() as u32)
    });
    let crypto = ctx.read("/proc/crypto").unwrap_or_default();
    let aes = best(&crypto, "aes");
    let aes_hw = aes.as_deref().is_some_and(|d| {
        ["aesni", "-ce", "vaes", "aes-s390", "p8"]
            .iter()
            .any(|k| d.contains(k))
    });
    let data = CryptoInfo {
        ca_bundle: bundle.map(|b| b.to_string()),
        ca_count,
        aes,
        aes_hw,
        sha256: best(&crypto, "sha256"),
        hw_rng: ctx
            .read_trimmed("/sys/class/misc/hw_random/rng_current")
            .filter(|r| !r.is_empty() && r != "none"),
        tpm: ctx.read_trimmed("/sys/class/tpm/tpm0/tpm_version_major"),
        policy: ctx
            .read_trimmed("/etc/crypto-policies/config")
            .and_then(|c| {
                c.lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty() && !l.starts_with('#'))
                    .map(str::to_string)
            }),
    };
    if data.ca_bundle.is_none() && crypto.is_empty() {
        let mut r = ProbeResult::new(NAME, Status::Unavailable, None);
        r.notes.push("no CA bundle and no /proc/crypto".to_string());
        return r;
    }
    ProbeResult::new(NAME, Status::Ok, Some(ProbeData::Crypto(data)))
}

/// The driver with the highest priority for an algorithm in /proc/crypto:
/// the one the kernel uses.
fn best(text: &str, name: &str) -> Option<String> {
    text.split("\n\n")
        .filter_map(|block| {
            let field = |k: &str| {
                block.lines().find_map(|l| {
                    let (key, v) = l.split_once(':')?;
                    (key.trim() == k).then(|| v.trim().to_string())
                })
            };
            if field("name")? != name {
                return None;
            }
            Some((field("priority")?.parse::<i64>().ok()?, field("driver")?))
        })
        .max_by_key(|(p, _)| *p)
        .map(|(_, d)| d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::fixture;

    #[test]
    fn laptop_crypto() {
        let Some(ProbeData::Crypto(c)) = probe(&fixture("laptop")).data else {
            panic!()
        };
        assert_eq!(
            c.ca_bundle.as_deref(),
            Some("/etc/ssl/certs/ca-certificates.crt")
        );
        assert_eq!(c.ca_count, Some(3));
        assert_eq!(c.aes.as_deref(), Some("aes-aesni"));
        assert!(c.aes_hw);
        assert_eq!(c.sha256.as_deref(), Some("sha256-ni"));
        assert_eq!(c.hw_rng.as_deref(), Some("tpm-rng-0"));
        assert_eq!(c.tpm.as_deref(), Some("2"));
        assert_eq!(c.policy, None);
        assert_eq!(probe(&fixture("empty")).status, Status::Unavailable);
    }
}
