use std::borrow::Cow;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use rmcp::handler::server::ServerHandler;
use rmcp::model::CallToolRequestParam;
use rmcp::model::CallToolResult;
use rmcp::model::JsonObject;
use rmcp::model::ListToolsResult;
use rmcp::model::PaginatedRequestParam;
use rmcp::model::ServerCapabilities;
use rmcp::model::ServerInfo;
use rmcp::model::Tool;
use rmcp::ErrorData as McpError;
use rmcp::ServiceExt;
use serde::Deserialize;
use serde_json::json;
use tokio::task;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7_2) AppleWebKit/537.36";
const MAX_REDIRECTS: usize = 5;
const MAX_FETCH_BYTES: usize = 2_500_000; // hard limit for safety regardless of maxChars

fn stdio() -> (tokio::io::Stdin, tokio::io::Stdout) {
    (tokio::io::stdin(), tokio::io::stdout())
}

#[derive(Clone)]
struct WebMcpServer {
    tools: Arc<Vec<Tool>>,
    http: reqwest::Client,
}

impl WebMcpServer {
    fn new() -> anyhow::Result<Self> {
        let tools = vec![Self::tool_web_search()?, Self::tool_web_fetch()?];

        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .context("build http client")?;

        Ok(Self {
            tools: Arc::new(tools),
            http,
        })
    }

    fn tool_web_search() -> anyhow::Result<Tool> {
        let schema: JsonObject = serde_json::from_value(json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query." },
                "count": { "type": "integer", "minimum": 1, "maximum": 10, "default": 5 }
            },
            "required": ["query"],
            "additionalProperties": false
        }))
        .context("deserialize web_search schema")?;

        Ok(Tool::new(
            Cow::Borrowed("web_search"),
            Cow::Borrowed(
                "Search the web via Brave Search API. Returns titles, URLs, and snippets.",
            ),
            Arc::new(schema),
        ))
    }

    fn tool_web_fetch() -> anyhow::Result<Tool> {
        let schema: JsonObject = serde_json::from_value(json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to fetch (http/https only)." },
                "extractMode": { "type": "string", "enum": ["markdown", "text"], "default": "markdown" },
                "maxChars": { "type": "integer", "minimum": 100, "maximum": 200000, "default": 50000 }
            },
            "required": ["url"],
            "additionalProperties": false
        }))
        .context("deserialize web_fetch schema")?;

        Ok(Tool::new(
            Cow::Borrowed("web_fetch"),
            Cow::Borrowed("Fetch a URL and extract readable content. Returns JSON with text."),
            Arc::new(schema),
        ))
    }

    fn brave_api_key() -> Result<String, McpError> {
        // Prefer our env var name; accept nanobot-compatible BRAVE_API_KEY too.
        if let Ok(v) = std::env::var("BRAVE_SEARCH_API_KEY") {
            if !v.trim().is_empty() {
                return Ok(v);
            }
        }
        if let Ok(v) = std::env::var("BRAVE_API_KEY") {
            if !v.trim().is_empty() {
                return Ok(v);
            }
        }
        Err(McpError::invalid_params(
            "missing BRAVE_SEARCH_API_KEY (or BRAVE_API_KEY) env var",
            Some(json!({})),
        ))
    }

    async fn brave_search(&self, query: &str, count: i64) -> Result<serde_json::Value, McpError> {
        let key = Self::brave_api_key()?;

        let resp = self
            .http
            .get("https://api.search.brave.com/res/v1/web/search")
            .query(&[("q", query), ("count", &count.to_string())])
            .header("Accept", "application/json")
            .header("X-Subscription-Token", key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let status = resp.status();
        let value = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if !status.is_success() {
            return Err(McpError::internal_error(
                format!("brave search http error: {}", status.as_u16()),
                Some(value),
            ));
        }

        Ok(value)
    }

    async fn validate_fetch_url(&self, url: &reqwest::Url) -> Result<(), McpError> {
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(McpError::invalid_params(
                format!("only http/https urls allowed (got {scheme})"),
                None,
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(McpError::invalid_params(
                "userinfo in URL is not allowed",
                None,
            ));
        }

        let host = url.host_str().unwrap_or("");
        if host.is_empty() {
            return Err(McpError::invalid_params("missing host", None));
        }

        // Block common local hostnames early.
        let h = host.to_ascii_lowercase();
        if h == "localhost" || h.ends_with(".localhost") || h.ends_with(".local") {
            return Err(McpError::invalid_params(
                "local hostnames are not allowed",
                None,
            ));
        }

        // Optional allow/deny domain lists (role-based restrictions).
        // Deny takes precedence over allow.
        let deny = parse_domain_list_env("GRAIL_WEB_DENY_DOMAINS");
        if deny.iter().any(|d| domain_matches(&h, d)) {
            return Err(McpError::invalid_params(
                "domain blocked by GRAIL_WEB_DENY_DOMAINS",
                Some(json!({ "host": h })),
            ));
        }
        let allow = parse_domain_list_env("GRAIL_WEB_ALLOW_DOMAINS");
        if !allow.is_empty() && !allow.iter().any(|d| domain_matches(&h, d)) {
            return Err(McpError::invalid_params(
                "domain not allowed by GRAIL_WEB_ALLOW_DOMAINS",
                Some(json!({ "host": h })),
            ));
        }

        let port = url.port_or_known_default().unwrap_or(0);
        let expected = match scheme {
            "http" => 80,
            "https" => 443,
            _ => 0,
        };
        if port != expected {
            return Err(McpError::invalid_params(
                format!("only default ports are allowed (expected {expected}, got {port})"),
                None,
            ));
        }

        // Resolve and block private/reserved IPs to mitigate SSRF.
        if let Ok(ip) = host.parse::<IpAddr>() {
            if !is_public_ip(&ip) {
                return Err(McpError::invalid_params(
                    "private/reserved IPs are not allowed",
                    None,
                ));
            }
            return Ok(());
        }

        let addrs = tokio::net::lookup_host((host, port))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        for addr in addrs {
            if !is_public_ip(&addr.ip()) {
                return Err(McpError::invalid_params(
                    "host resolves to private/reserved IP; blocked for safety",
                    None,
                ));
            }
        }

        Ok(())
    }

    async fn fetch_url(
        &self,
        url: &reqwest::Url,
        extract_mode: &str,
        max_chars: usize,
    ) -> Result<serde_json::Value, McpError> {
        self.validate_fetch_url(url).await?;

        let mut resp = self
            .http
            .get(url.clone())
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let status = resp.status().as_u16();
        let final_url = resp.url().to_string();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let mut buf: Vec<u8> = Vec::new();
        let mut truncated_bytes = false;
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
        {
            if buf.len() + chunk.len() > MAX_FETCH_BYTES {
                let remaining = MAX_FETCH_BYTES.saturating_sub(buf.len());
                buf.extend_from_slice(&chunk[..remaining]);
                truncated_bytes = true;
                break;
            }
            buf.extend_from_slice(&chunk);
        }

        let (extractor, mut text) = extract_bytes(&buf, &content_type, extract_mode)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut truncated = truncated_bytes;
        if text.chars().count() > max_chars {
            text = text.chars().take(max_chars).collect();
            truncated = true;
        }

        Ok(json!({
            "url": url.to_string(),
            "finalUrl": final_url,
            "status": status,
            "contentType": content_type,
            "extractMode": extract_mode,
            "extractor": extractor,
            "truncated": truncated,
            "length": text.chars().count(),
            "text": text,
        }))
    }
}

#[derive(Deserialize)]
struct ArgsWebSearch {
    query: String,
    #[serde(default)]
    count: Option<i64>,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct ArgsWebFetch {
    url: String,
    #[serde(default)]
    extractMode: Option<String>,
    #[serde(default)]
    maxChars: Option<usize>,
}

impl ServerHandler for WebMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
            ..ServerInfo::default()
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let tools = self.tools.clone();
        async move {
            Ok(ListToolsResult {
                tools: (*tools).clone(),
                next_cursor: None,
                meta: None,
            })
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "web_search" => {
                let args = parse_args::<ArgsWebSearch>(&request, "web_search")?;
                let q = args.query.trim();
                if q.is_empty() {
                    return Err(McpError::invalid_params("query is required", None));
                }
                let count = args.count.unwrap_or(5).clamp(1, 10);

                let value = self.brave_search(q, count).await?;
                let results = value
                    .get("web")
                    .and_then(|v| v.get("results"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                let simplified: Vec<serde_json::Value> = results
                    .into_iter()
                    .take(count as usize)
                    .map(|item| {
                        json!({
                            "title": item.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                            "url": item.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                            "description": item.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                        })
                    })
                    .collect();

                Ok(CallToolResult {
                    content: Vec::new(),
                    structured_content: Some(json!({
                        "query": q,
                        "count": count,
                        "results": simplified,
                    })),
                    is_error: Some(false),
                    meta: None,
                })
            }
            "web_fetch" => {
                let args = parse_args::<ArgsWebFetch>(&request, "web_fetch")?;
                let url = reqwest::Url::parse(args.url.trim())
                    .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let extract_mode = args
                    .extractMode
                    .as_deref()
                    .unwrap_or("markdown")
                    .trim()
                    .to_string();
                let max_chars = args.maxChars.unwrap_or(50_000).clamp(100, 200_000);

                let data = self.fetch_url(&url, &extract_mode, max_chars).await?;
                Ok(CallToolResult {
                    content: Vec::new(),
                    structured_content: Some(data),
                    is_error: Some(false),
                    meta: None,
                })
            }
            other => Err(McpError::invalid_params(
                format!("unknown tool: {other}"),
                None,
            )),
        }
    }
}

fn parse_args<T: for<'de> Deserialize<'de>>(
    request: &CallToolRequestParam,
    tool_name: &'static str,
) -> Result<T, McpError> {
    match request.arguments.as_ref() {
        Some(arguments) => serde_json::from_value(serde_json::Value::Object(
            arguments.clone().into_iter().collect(),
        ))
        .map_err(|err| McpError::invalid_params(err.to_string(), None)),
        None => Err(McpError::invalid_params(
            format!("missing arguments for {tool_name} tool"),
            None,
        )),
    }
}

fn extract_bytes(
    body: &[u8],
    content_type: &str,
    _extract_mode: &str,
) -> anyhow::Result<(&'static str, String)> {
    let ct = content_type.to_ascii_lowercase();
    if ct.contains("application/json") {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) {
            let pretty = serde_json::to_string_pretty(&v)?;
            return Ok(("json", pretty));
        }
    }

    let s = String::from_utf8_lossy(body).to_string();
    let head = s.chars().take(256).collect::<String>().to_ascii_lowercase();
    if ct.contains("text/html")
        || head.trim_start().starts_with("<!doctype")
        || head.contains("<html")
    {
        let txt = html2text::from_read(s.as_bytes(), 120)?;
        return Ok(("html2text", normalize_whitespace(&txt)));
    }

    Ok(("raw", normalize_whitespace(&s)))
}

fn normalize_whitespace(input: &str) -> String {
    let s = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(s.len());
    let mut last_was_nl = false;
    let mut nl_run = 0usize;
    for ch in s.chars() {
        if ch == '\n' {
            if last_was_nl {
                nl_run += 1;
            } else {
                nl_run = 1;
            }
            // cap newlines at 2
            if nl_run <= 2 {
                out.push('\n');
            }
            last_was_nl = true;
            continue;
        }
        last_was_nl = false;
        nl_run = 0;
        out.push(ch);
    }
    out.trim().to_string()
}

fn is_public_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified())
        }
        IpAddr::V6(v6) => {
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || is_ipv6_documentation(v6))
        }
    }
}

fn parse_domain_list_env(key: &str) -> Vec<String> {
    let Ok(v) = std::env::var(key) else {
        return Vec::new();
    };
    v.split(|c: char| c == ',' || c == '\n' || c == '\r' || c == '\t' || c == ' ')
        .map(|s| s.trim().trim_matches('.').to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn domain_matches(host: &str, domain: &str) -> bool {
    if host == domain {
        return true;
    }
    // Allow subdomains.
    host.ends_with(&format!(".{domain}"))
}

fn is_ipv6_documentation(v6: &std::net::Ipv6Addr) -> bool {
    // 2001:db8::/32 is reserved for documentation.
    let seg = v6.segments();
    seg[0] == 0x2001 && seg[1] == 0x0db8
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let service = WebMcpServer::new()?;
    info!("starting grail-web-mcp (stdio)");

    let running = service.serve(stdio()).await?;
    if let Err(err) = running.waiting().await {
        error!(error = %err, "mcp server exiting");
        return Err(anyhow::Error::new(err));
    }

    task::yield_now().await;
    Ok(())
}
