//! Phase 4 acceptance: `atfq mcp` driven over stdio with JSON-RPC, as an MCP
//! client would.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

struct Client {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: u64,
}

impl Client {
    fn start() -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_atfq"))
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start atfq mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut c = Client {
            child,
            stdin,
            lines,
            next_id: 1,
        };
        let init = c.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "atfq-test", "version": "0" }
            }),
        );
        assert_eq!(init["result"]["serverInfo"]["name"], "atfq");
        c.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        c
    }

    fn send(&mut self, msg: Value) {
        writeln!(self.stdin, "{msg}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(10))
                .expect("no answer from atfq mcp within 10 s");
            let msg: Value = serde_json::from_str(&line).expect("server sent invalid JSON");
            if msg["id"] == id {
                return msg;
            }
        }
    }

    /// Calls a tool; returns (is_error, text of the first content block).
    fn call(&mut self, tool: &str, args: Value) -> (bool, String) {
        let r = self.request("tools/call", json!({ "name": tool, "arguments": args }));
        let result = &r["result"];
        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("no text content: {r}"))
            .to_string();
        (result["isError"].as_bool().unwrap_or(false), text)
    }

    /// Calls a tool that must succeed with JSON.
    fn call_json(&mut self, tool: &str, args: Value) -> Value {
        let (is_error, text) = self.call(tool, args);
        assert!(!is_error, "{tool} failed: {text}");
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{tool}: invalid JSON ({e}): {text}"))
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn lists_five_read_only_tools() {
    let mut c = Client::start();
    let r = c.request("tools/list", json!({}));
    let tools = r["result"]["tools"].as_array().expect("tools");
    let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort();
    assert_eq!(
        names,
        ["list_probes", "lookup", "run_probe", "search", "toc"]
    );
    for t in tools {
        assert_eq!(t["annotations"]["readOnlyHint"], true, "{t}");
        let desc = t["description"].as_str().unwrap();
        assert!(desc.contains("Read-only"), "{desc}");
    }
}

#[test]
fn lookup_returns_card_and_live_data() {
    let mut c = Client::start();
    let v = c.call_json("lookup", json!({ "query": "free" }));
    assert_eq!(v["match"], "exact");
    assert_eq!(v["entry"]["kind"], "card");
    assert_eq!(v["entry"]["number"], "17.1");
    assert_eq!(v["probes"][0]["name"], "mem.summary");

    let v = c.call_json("lookup", json!({ "query": "memroy" }));
    assert_eq!(v["match"], "closest");
    assert_eq!(v["entry"]["id"], "memory");

    let v = c.call_json("lookup", json!({ "query": "qwertyuiop" }));
    assert_eq!(v["match"], "none");
    assert!(v["entry"].is_null());
}

#[test]
fn run_probe_returns_json_or_a_clear_error() {
    let mut c = Client::start();
    for name in ["mem.summary", "disk.tree", "proc.states"] {
        let v = c.call_json("run_probe", json!({ "name": name }));
        assert_eq!(v["name"], name);
        assert!(v["status"].is_string());
    }
    let (is_error, text) = c.call("run_probe", json!({ "name": "nope" }));
    assert!(is_error);
    assert!(text.contains("mem.summary"), "{text}");
}

#[test]
fn every_other_tool_returns_json() {
    let mut c = Client::start();
    let toc = c.call_json("toc", json!({}));
    assert_eq!(toc["chapters"][16]["title"], "Memory");
    assert_eq!(toc["chapters"][16]["cards"][0]["handle"], "17.1");

    let hits = c.call_json("search", json!({ "query": "zombie" }));
    assert_eq!(hits[0]["id"], "proc.states");

    let probes = c.call_json("list_probes", json!({}));
    assert_eq!(
        probes.as_array().unwrap().len(),
        atfq::probes::REGISTRY.len()
    );
    assert!(probes[0]["description"].is_string());
}

#[test]
fn bad_arguments_are_an_error_not_a_crash() {
    let mut c = Client::start();
    let r = c.request("tools/call", json!({ "name": "lookup", "arguments": {} }));
    assert!(
        r.get("error").is_some() || r["result"]["isError"] == true,
        "{r}"
    );
    // Still alive afterwards.
    let v = c.call_json("lookup", json!({ "query": "disk" }));
    assert_eq!(v["entry"]["id"], "disk");
}
