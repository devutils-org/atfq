//! `atfq mcp`: the same knowledge and probes for AI agents, as an MCP server
//! on stdio (ROADMAP.md 8.4). Every tool is read-only.

use std::io;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};

use crate::content::Library;
use crate::probes::{self, ProbeCtx};
use crate::search::search;
use crate::view::{lookup, toc};
use crate::BIN_NAME;

/// Results returned by the `search` tool.
const SEARCH_LIMIT: usize = 10;

const INSTRUCTIONS: &str = "atfq explains Linux in plain English and reads the live state of \
this machine (memory, disks, processes). It is strictly read-only: it never runs commands, \
never changes files or settings, and never uses the network. Use `lookup` for a topic, a \
command or a chapter number (`free`, `zombie`, `find`, `5.1`), `search` for free text, and \
`run_probe` for raw machine data.";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct QueryArgs {
    /// A hub, a topic, a command or a chapter number: `memory`, `zombie`, `5.1`
    pub query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ProbeArgs {
    /// A probe name from `list_probes`, such as `mem.summary`
    pub name: String,
}

#[derive(Clone)]
pub struct Server {
    lib: Arc<Library>,
    ctx: ProbeCtx,
    tool_router: ToolRouter<Server>,
}

fn json<T: Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::json(value)?]))
}

#[tool_router]
impl Server {
    pub fn new(lib: Library, ctx: ProbeCtx) -> Server {
        Server {
            lib: Arc::new(lib),
            ctx,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Look up a hub or a card by name, alias or chapter number, with live data \
                       from this machine. Read-only: never modifies the system. Returns JSON: \
                       `match` (exact, closest or none), `entry` (the card or hub), \
                       `suggestions`, `probes`.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn lookup(
        &self,
        Parameters(args): Parameters<QueryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json(&lookup(&self.lib, &args.query, &self.ctx))
    }

    #[tool(
        description = "Full-text search over every hub and card. Read-only. Returns a JSON list \
                       ranked best first, each with `handle` to pass to `lookup`.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn search(
        &self,
        Parameters(args): Parameters<QueryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json(&search(&self.lib, &args.query, SEARCH_LIMIT))
    }

    #[tool(
        description = "The table of contents: hubs, then numbered chapters and their cards. \
                       Read-only. Returns JSON.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn toc(&self) -> Result<CallToolResult, ErrorData> {
        json(&toc(&self.lib))
    }

    #[tool(
        description = "Run one probe: read live data from /proc and /sys on this machine. \
                       Read-only: it never runs commands or changes anything. Returns JSON \
                       with `status` (ok, partial, needs_root, unavailable, unsupported), \
                       `data` and `notes`.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn run_probe(
        &self,
        Parameters(args): Parameters<ProbeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        match probes::find(&args.name) {
            Some(p) => json(&p.run(&self.ctx)),
            None => {
                let names: Vec<&str> = probes::REGISTRY.iter().map(|p| p.name).collect();
                Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "No probe named \"{}\". Available: {}.",
                    args.name,
                    names.join(", ")
                ))]))
            }
        }
    }

    #[tool(
        description = "The available probes, with a one-line description each. Read-only. \
                       Returns JSON.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_probes(&self) -> Result<CallToolResult, ErrorData> {
        json(&probes::REGISTRY)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(BIN_NAME, env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}

/// Serves MCP on stdin/stdout until the client disconnects.
pub fn serve(lib: Library, ctx: ProbeCtx) -> io::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let service = Server::new(lib, ctx)
            .serve(rmcp::transport::stdio())
            .await
            .map_err(io::Error::other)?;
        service.waiting().await.map_err(io::Error::other)?;
        Ok(())
    })
}
