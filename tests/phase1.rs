//! Phase 1 acceptance: rendered pages with live probe data from fixtures,
//! fuzzy matching, search, and JSON output.

mod common;

use std::process::Command;

use atfq::probes::{self, run_all};
use atfq::render;
use atfq::search::search;
use atfq::view::{lookup, Match};
use common::{fixture, laptop, lib, show, PLAIN, WHY};
use insta::assert_snapshot;

#[test]
fn snapshot_every_hub() {
    let lib = lib();
    let ctx = laptop();
    for h in lib.hubs() {
        let page = render::hub(&lib, h, None, &run_all(&h.probes, &ctx), PLAIN);
        assert_snapshot!(format!("hub_{}", h.id), page);
    }
}

#[test]
fn snapshot_every_card() {
    let lib = lib();
    let ctx = laptop();
    for c in lib.cards() {
        let live = run_all(&c.probes, &ctx);
        assert_snapshot!(
            format!("card_{}", c.id),
            render::card(&lib, c, &live, PLAIN)
        );
        assert_snapshot!(
            format!("card_{}_why", c.id),
            render::card(&lib, c, &live, WHY)
        );
    }
}

#[test]
fn snapshot_probes_when_data_is_missing() {
    for name in ["old-kernel", "empty", "garbage"] {
        let ctx = fixture(name);
        let mut out = String::new();
        for info in probes::REGISTRY {
            out.push_str(&render::probe(info, &info.run(&ctx), PLAIN));
            out.push('\n');
        }
        assert_snapshot!(format!("probes_{name}"), out);
    }
}

#[test]
fn snapshot_toc() {
    assert_snapshot!("toc", render::toc(&lib(), PLAIN));
}

#[test]
fn typo_suggests_the_closest_page() {
    let lib = lib();
    let l = lookup(&lib, "memroy", &laptop());
    assert_eq!(l.matched, Match::Closest);
    assert_eq!(l.entry.map(|t| t.id()), Some("memory"));
    let out = show(&lib, "memroy", PLAIN);
    assert!(
        out.starts_with("No exact match for \"memroy\", showing \"Memory\"."),
        "{out}"
    );
    assert!(out.contains("17. Memory  hub"));
}

#[test]
fn fuzzy_matches_aliases_and_prefixes() {
    let lib = lib();
    for (query, id) in [
        ("zombi", "proc.states"),
        ("zomb", "proc.states"),
        ("dsik", "disk"),
        ("free vs avialable", "memory.free-vs-available"),
        ("swapiness", "memory.swap"),
    ] {
        let l = lookup(&lib, query, &laptop());
        assert_eq!(l.matched, Match::Closest, "{query}");
        assert_eq!(l.entry.map(|t| t.id()), Some(id), "{query}");
    }
}

#[test]
fn no_match_suggests_from_search() {
    let lib = lib();
    // Not an id, alias or title, but one typo away from a word of the
    // OOM killer card.
    let l = lookup(&lib, "overcomit", &laptop());
    assert_eq!(l.matched, Match::None);
    assert!(l.entry.is_none() && l.probes.is_empty());
    let ids: Vec<&str> = l.suggestions.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids[0], "memory.oom-killer", "{ids:?}");
    assert!(ids.contains(&"kernel.sysctl"), "{ids:?}");
    assert_snapshot!("not_found_overcomit", render::not_found(&l, PLAIN));
}

#[test]
fn search_ranks_and_snippets() {
    let lib = lib();
    let hits = search(&lib, "delete matches", 10);
    assert_eq!(hits[0].id, "cmd.find");
    assert_snapshot!(
        "search_swap",
        render::search("swap", &search(&lib, "swap", 10), PLAIN)
    );
    assert!(search(&lib, "", 10).is_empty());
    assert!(search(&lib, "qwertyuiop", 10).is_empty());
}

#[test]
fn lookup_json_has_card_and_probes() {
    let lib = lib();
    let l = lookup(&lib, "free", &laptop());
    let v: serde_json::Value = serde_json::to_value(&l).unwrap();
    assert_eq!(v["match"], "exact");
    assert_eq!(v["entry"]["kind"], "card");
    assert_eq!(v["entry"]["id"], "memory.free-vs-available");
    assert_eq!(v["entry"]["number"], "17.1");
    assert_eq!(v["entry"]["deep_dive"].as_array().unwrap().len(), 3);
    assert_eq!(v["probes"][0]["name"], "mem.summary");
    assert_eq!(v["probes"][0]["status"], "ok");
    assert_eq!(v["probes"][0]["data"]["total"], 16_264_712u64 * 1024);
}

fn idk(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_idk"))
        .args(args)
        .output()
        .expect("run idk");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn cli_typo_and_search() {
    let (code, out) = idk(&["memroy"]);
    assert_eq!(code, 0);
    assert!(out.contains("showing \"Memory\""));

    let (code, out) = idk(&["search", "zombie"]);
    assert_eq!(code, 0);
    assert!(out.contains("idk 16.1"));
    assert_eq!(idk(&["search", "qwertyuiop"]).0, 1);
    assert_eq!(idk(&["search"]).0, 2);
}

#[test]
fn cli_probe_and_json() {
    // Real machine: whatever the values, it must not crash and must be JSON.
    for p in probes::REGISTRY {
        let (code, out) = idk(&["probe", p.name, "--json"]);
        assert_eq!(code, 0, "{}", p.name);
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(v["name"], p.name);
    }
    assert_eq!(idk(&["probe", "nope"]).0, 1);
    let (code, out) = idk(&["probe"]);
    assert_eq!(code, 0);
    assert!(out.contains("idk probe mem.summary"));

    for args in [
        &["free", "--json"][..],
        &["toc", "--json"],
        &["search", "swap", "--json"],
    ] {
        let (code, out) = idk(args);
        assert_eq!(code, 0, "{args:?}");
        serde_json::from_str::<serde_json::Value>(&out).expect("valid JSON");
    }
    let (code, out) = idk(&["nothing", "--json"]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["match"], "none");
}
