//! Proxy service
//!
//! Local HTTP proxy server for AI API requests

use crate::error::{AppError, AppResult};
use crate::proxy::{ActiveTarget, ProxyConfig, ProxyEvent, ProxyStatus};
use crate::services::usage::UsageService;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};

/// Shared proxy service
pub type SharedProxyService = Arc<RwLock<ProxyService>>;

/// A fully resolved forwarding target including the secret API key.
#[derive(Clone)]
struct ResolvedTarget {
    provider_id: String,
    base_url: String,
    api_key: String,
    api_format: String,
}

/// Context shared with each spawned connection handler.
struct ForwardCtx {
    target: Option<ResolvedTarget>,
    db_path: Option<PathBuf>,
    client: reqwest::Client,
}

/// Proxy service for managing the local HTTP proxy
pub struct ProxyService {
    config: ProxyConfig,
    status: ProxyStatus,
    shutdown_tx: Option<mpsc::Sender<()>>,
    event_tx: Option<mpsc::Sender<ProxyEvent>>,
    db_path: Option<PathBuf>,
    /// The target resolved at the last `start()`, for display purposes.
    resolved_target: Option<ActiveTarget>,
}

impl ProxyService {
    /// Create a new proxy service
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            status: ProxyStatus::Stopped,
            shutdown_tx: None,
            event_tx: None,
            db_path: None,
            resolved_target: None,
        }
    }

    /// Create a proxy service that records usage into the given database.
    pub fn new_with_db(config: ProxyConfig, db_path: PathBuf) -> Self {
        let mut s = Self::new(config);
        s.db_path = Some(db_path);
        s
    }

    /// Point the proxy at a database for usage recording / target resolution.
    pub fn set_db_path(&mut self, db_path: PathBuf) {
        self.db_path = Some(db_path);
    }

    /// The upstream provider resolved at the last successful `start()`, if any.
    pub fn resolved_target(&self) -> Option<&ActiveTarget> {
        self.resolved_target.as_ref()
    }

    /// The local proxy URL clients should point at, e.g. `http://127.0.0.1:15721`.
    pub fn proxy_url(&self) -> String {
        format!("http://{}:{}", self.config.address, self.config.port)
    }

    /// Enable or disable "takeover" for an app: point the app's live config at
    /// this local proxy (so its traffic is routed through it), backing up the
    /// previous endpoint so it can be restored on disable.
    ///
    /// Currently supported for Claude Code only.
    pub fn set_takeover(&self, app_type: crate::provider::AppType, enable: bool) -> AppResult<()> {
        use crate::provider::AppType;
        match app_type {
            AppType::Claude => claude_set_takeover(&self.proxy_url(), enable),
            other => Err(AppError::Proxy(format!(
                "Takeover for {:?} is not yet supported in the CLI (only Claude).",
                other
            ))),
        }
    }

    /// Whether the app's live config currently points at this proxy.
    pub fn is_taken_over(&self, app_type: crate::provider::AppType) -> bool {
        use crate::provider::AppType;
        match app_type {
            AppType::Claude => claude_is_taken_over(&self.proxy_url()),
            _ => false,
        }
    }

    /// Set event channel for proxy events
    #[allow(dead_code)]
    pub fn set_event_channel(&mut self, tx: mpsc::Sender<ProxyEvent>) {
        self.event_tx = Some(tx);
    }

    /// Start the proxy server
    pub async fn start(&mut self) -> AppResult<()> {
        if self.status == ProxyStatus::Running {
            return Ok(());
        }

        self.status = ProxyStatus::Starting;

        let addr = SocketAddr::from(([127, 0, 0, 1], self.config.port));
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| AppError::Proxy(format!("Failed to bind to {}: {}", addr, e)))?;

        // Resolve the upstream provider to forward to (first provider with a
        // base_url and a non-empty API key).
        let target = self.db_path.as_ref().and_then(|db| resolve_target(db));
        self.resolved_target = target.as_ref().map(|t| ActiveTarget {
            provider_id: t.provider_id.clone(),
            base_url: t.base_url.clone(),
            api_format: t.api_format.clone(),
        });

        let ctx = Arc::new(ForwardCtx {
            target,
            db_path: self.db_path.clone(),
            client: reqwest::Client::new(),
        });

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        self.status = ProxyStatus::Running;

        // Spawn the server task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                log::debug!("Proxy connection from {}", addr);
                                let ctx = ctx.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, ctx).await {
                                        log::debug!("Proxy connection error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                log::debug!("Accept error: {}", e);
                            }
                        }
                    }
                }
            }
        });

        log::debug!("Proxy server started on http://{}", addr);
        Ok(())
    }

    /// Stop the proxy server
    pub async fn stop(&mut self) -> AppResult<()> {
        if self.status != ProxyStatus::Running {
            return Ok(());
        }

        self.status = ProxyStatus::Stopping;

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        self.status = ProxyStatus::Stopped;
        log::debug!("Proxy server stopped");
        Ok(())
    }

    /// Get current status
    pub fn status(&self) -> ProxyStatus {
        self.status
    }

    /// Get current config
    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }
}

/// A parsed HTTP/1.1 request.
struct ParsedRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// Read and parse a full HTTP/1.1 request (headers + Content-Length body).
async fn read_http_request(
    stream: &mut tokio::net::TcpStream,
) -> std::io::Result<Option<ParsedRequest>> {
    use tokio::io::AsyncReadExt;

    let mut buf: Vec<u8> = Vec::with_capacity(8192);
    let mut tmp = [0u8; 8192];

    // Read until we have the full header block.
    let header_end = loop {
        if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
            break pos;
        }
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return Ok(None); // connection closed before headers completed
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > 1024 * 1024 {
            return Ok(None); // header block too large; bail
        }
    };

    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let mut headers = Vec::new();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_string();
            let val = v.trim().to_string();
            if key.eq_ignore_ascii_case("content-length") {
                content_length = val.parse().unwrap_or(0);
            }
            headers.push((key, val));
        }
    }

    // Body: whatever followed the header block, plus the remaining bytes.
    let mut body = buf[header_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }

    Ok(Some(ParsedRequest {
        method,
        path,
        headers,
        body,
    }))
}

/// Whether a header is hop-by-hop and must not be forwarded.
fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
            | "host"
            | "content-length"
            // auth headers are re-injected from the provider config
            | "authorization"
            | "x-api-key"
            | "x-goog-api-key"
    )
}

/// Handle a proxy connection: forward to the resolved upstream provider and
/// record token usage from the response.
async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    ctx: Arc<ForwardCtx>,
) -> std::io::Result<()> {
    use tokio::io::AsyncWriteExt;

    let request = match read_http_request(&mut stream).await? {
        Some(r) => r,
        None => return Ok(()),
    };

    let target = match &ctx.target {
        Some(t) => t,
        None => {
            let msg = b"No upstream provider configured. Add a provider with an API key in Olenro.";
            let resp = format!(
                "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                msg.len()
            );
            stream.write_all(resp.as_bytes()).await?;
            stream.write_all(msg).await?;
            return Ok(());
        }
    };

    match forward_upstream(&ctx.client, target, &request).await {
        Ok((status, resp_headers, resp_body)) => {
            // Record usage (best-effort; never blocks the response).
            let (input, output, model) = extract_usage(&resp_body);
            if input > 0 || output > 0 {
                if let Some(db) = &ctx.db_path {
                    let cost = model
                        .as_deref()
                        .map(|m| crate::pricing::compute_cost(m, input, output))
                        .unwrap_or(0.0);
                    let svc = UsageService::new(db.clone());
                    if let Err(e) = svc.record(&target.provider_id, 1, input, output, cost) {
                        log::debug!("Failed to record usage: {}", e);
                    }
                }
            }

            write_response(&mut stream, status, &resp_headers, &resp_body).await?;
        }
        Err(e) => {
            let msg = format!("Upstream request failed: {}", e);
            let resp = format!(
                "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                msg.len()
            );
            stream.write_all(resp.as_bytes()).await?;
            stream.write_all(msg.as_bytes()).await?;
        }
    }

    Ok(())
}

/// Forward the request to the upstream provider, returning (status, headers, body).
async fn forward_upstream(
    client: &reqwest::Client,
    target: &ResolvedTarget,
    request: &ParsedRequest,
) -> AppResult<(u16, Vec<(String, String)>, Vec<u8>)> {
    let url = format!("{}{}", target.base_url.trim_end_matches('/'), request.path);

    let method = reqwest::Method::from_bytes(request.method.as_bytes())
        .map_err(|e| AppError::Proxy(format!("Invalid method: {}", e)))?;

    let mut builder = client.request(method, &url);

    // Forward all non-hop-by-hop headers from the client.
    for (k, v) in &request.headers {
        if !is_hop_by_hop(k) {
            builder = builder.header(k, v);
        }
    }

    // Inject provider auth based on API format.
    match target.api_format.as_str() {
        "anthropic" => {
            builder = builder.header("x-api-key", &target.api_key);
            if !request
                .headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("anthropic-version"))
            {
                builder = builder.header("anthropic-version", "2023-06-01");
            }
        }
        "gemini_native" => {
            builder = builder.header("x-goog-api-key", &target.api_key);
        }
        // openai_chat / openai_responses / anything else
        _ => {
            builder = builder.header("authorization", format!("Bearer {}", target.api_key));
        }
    }

    if !request.body.is_empty() {
        builder = builder.body(request.body.clone());
    }

    let resp = builder
        .send()
        .await
        .map_err(|e| AppError::Proxy(format!("send: {}", e)))?;

    let status = resp.status().as_u16();
    let mut headers = Vec::new();
    for (k, v) in resp.headers().iter() {
        if let Ok(val) = v.to_str() {
            headers.push((k.as_str().to_string(), val.to_string()));
        }
    }

    let body = resp
        .bytes()
        .await
        .map_err(|e| AppError::Proxy(format!("read body: {}", e)))?
        .to_vec();

    Ok((status, headers, body))
}

/// Write an HTTP/1.1 response back to the client, recomputing Content-Length.
async fn write_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    headers: &[(String, String)],
    body: &[u8],
) -> std::io::Result<()> {
    use tokio::io::AsyncWriteExt;

    let mut head = format!("HTTP/1.1 {} \r\n", status);
    for (k, v) in headers {
        // Drop framing headers; we set our own Content-Length and close.
        if is_hop_by_hop(k) || k.eq_ignore_ascii_case("content-length") {
            continue;
        }
        head.push_str(&format!("{}: {}\r\n", k, v));
    }
    head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    head.push_str("Connection: close\r\n\r\n");

    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

/// Extract (input_tokens, output_tokens, model) from a provider response body.
///
/// Handles both non-streaming JSON and SSE (`text/event-stream`) bodies, and
/// both Anthropic (`input_tokens`/`output_tokens`) and OpenAI
/// (`prompt_tokens`/`completion_tokens`) field names. The model name (used for
/// cost lookup) is taken from the first `model` field encountered.
fn extract_usage(body: &[u8]) -> (i64, i64, Option<String>) {
    let mut input = 0i64;
    let mut output = 0i64;
    let mut model: Option<String> = None;

    // Try the whole body as a single JSON document first.
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
        collect_usage(&value, &mut input, &mut output, &mut model);
        if input > 0 || output > 0 {
            return (input, output, model);
        }
    }

    // Otherwise scan SSE `data:` lines (streaming responses).
    let text = String::from_utf8_lossy(body);
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(payload) = line.strip_prefix("data:") {
            let payload = payload.trim();
            if payload.is_empty() || payload == "[DONE]" {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
                collect_usage(&value, &mut input, &mut output, &mut model);
            }
        }
    }

    (input, output, model)
}

/// Recursively look for `usage` (token counts) and `model` (name) anywhere in
/// the JSON, accumulating the largest input/output counts and the first model.
fn collect_usage(
    value: &serde_json::Value,
    input: &mut i64,
    output: &mut i64,
    model: &mut Option<String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            if model.is_none() {
                if let Some(m) = map.get("model").and_then(|v| v.as_str()) {
                    if !m.is_empty() {
                        *model = Some(m.to_string());
                    }
                }
            }
            if let Some(usage) = map.get("usage").and_then(|u| u.as_object()) {
                let in_tok = usage
                    .get("input_tokens")
                    .or_else(|| usage.get("prompt_tokens"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let out_tok = usage
                    .get("output_tokens")
                    .or_else(|| usage.get("completion_tokens"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                *input = (*input).max(in_tok);
                *output = (*output).max(out_tok);
            }
            // Recurse (covers nested shapes like message_start.message.usage).
            for v in map.values() {
                collect_usage(v, input, output, model);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_usage(v, input, output, model);
            }
        }
        _ => {}
    }
}

/// Find the first index of `needle` within `haystack`.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Path to the takeover backup sidecar that remembers the pre-takeover endpoint.
fn claude_takeover_backup_path() -> PathBuf {
    crate::config::get_claude_config_dir().join(".olenro-takeover-backup.json")
}

/// Read `~/.claude/settings.json` as a JSON object (or `{}`).
fn read_claude_settings() -> serde_json::Value {
    let path = crate::config::get_claude_settings_path();
    if path.exists() {
        crate::config::read_json_file(&path).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    }
}

fn write_claude_settings(value: &serde_json::Value) -> AppResult<()> {
    let path = crate::config::get_claude_settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AppError::Io)?;
    }
    crate::config::write_json_file(&path, value)
}

/// Whether Claude's live `ANTHROPIC_BASE_URL` currently points at `proxy_url`.
fn claude_is_taken_over(proxy_url: &str) -> bool {
    read_claude_settings()
        .get("env")
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .and_then(|v| v.as_str())
        .map(|u| u == proxy_url)
        .unwrap_or(false)
}

/// Enable/disable Claude takeover by swapping `env.ANTHROPIC_BASE_URL` to/from
/// the local proxy, preserving the original endpoint in a backup sidecar.
fn claude_set_takeover(proxy_url: &str, enable: bool) -> AppResult<()> {
    let mut settings = read_claude_settings();
    if !settings.is_object() {
        settings = serde_json::json!({});
    }
    let backup_path = claude_takeover_backup_path();

    if enable {
        if claude_is_taken_over(proxy_url) {
            return Ok(()); // already taken over
        }
        // Record the current endpoint (may be absent) before overwriting.
        let current = settings
            .get("env")
            .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let _ = crate::config::write_json_file(
            &backup_path,
            &serde_json::json!({ "ANTHROPIC_BASE_URL": current }),
        );

        set_claude_env_key(&mut settings, "ANTHROPIC_BASE_URL", Some(proxy_url));
        write_claude_settings(&settings)?;
    } else {
        // Restore the backed-up endpoint (or remove the key if there was none).
        let restored = if backup_path.exists() {
            crate::config::read_json_file::<serde_json::Value>(&backup_path)
                .ok()
                .and_then(|v| v.get("ANTHROPIC_BASE_URL").cloned())
        } else {
            None
        };
        match restored {
            Some(serde_json::Value::String(url)) => {
                set_claude_env_key(&mut settings, "ANTHROPIC_BASE_URL", Some(&url));
            }
            _ => {
                set_claude_env_key(&mut settings, "ANTHROPIC_BASE_URL", None);
            }
        }
        write_claude_settings(&settings)?;
        let _ = std::fs::remove_file(&backup_path);
    }
    Ok(())
}

/// Set or remove a key in `settings.env`, creating the object if needed.
fn set_claude_env_key(settings: &mut serde_json::Value, key: &str, value: Option<&str>) {
    let obj = match settings.as_object_mut() {
        Some(o) => o,
        None => return,
    };
    let env = obj.entry("env").or_insert_with(|| serde_json::json!({}));
    if !env.is_object() {
        *env = serde_json::json!({});
    }
    let env_obj = env.as_object_mut().unwrap();
    match value {
        Some(v) => {
            env_obj.insert(key.to_string(), serde_json::Value::String(v.to_string()));
        }
        None => {
            env_obj.remove(key);
        }
    }
}

/// Resolve a forwarding target from the database: the first provider that has a
/// `base_url` and a non-empty `api_key` in its settings.
fn resolve_target(db_path: &std::path::Path) -> Option<ResolvedTarget> {
    use crate::services::provider::ProviderService;

    let svc = ProviderService::new(db_path.to_path_buf());
    let providers = svc.list_providers().ok()?;

    for p in providers {
        let base_url = p
            .settings_config
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let api_key = p
            .settings_config
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if base_url.is_empty() || api_key.is_empty() {
            continue;
        }

        let api_format = p
            .meta
            .api_format
            .map(|f| {
                serde_json::to_value(f)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "anthropic".to_string())
            })
            .unwrap_or_else(|| "anthropic".to_string());

        return Some(ResolvedTarget {
            provider_id: p.id,
            base_url,
            api_key,
            api_format,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::ProxyConfig;

    #[test]
    fn start_stop_via_handle_block_on() {
        // Mirrors how the TUI drives the proxy: a dedicated runtime whose Handle
        // is used to block_on the async start/stop from a non-worker thread.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.handle().clone();

        let mut proxy = ProxyService::new(ProxyConfig {
            enabled: true,
            port: 38219,
            address: "127.0.0.1".to_string(),
        });

        handle.block_on(proxy.start()).unwrap();
        assert_eq!(proxy.status(), ProxyStatus::Running);

        handle.block_on(proxy.stop()).unwrap();
        assert_eq!(proxy.status(), ProxyStatus::Stopped);
    }

    #[tokio::test]
    async fn forwards_request_and_records_usage() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // 1. Mock upstream that returns a JSON body carrying usage tokens.
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let up_addr = upstream.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut s, _)) = upstream.accept().await {
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf).await;
                let body = br#"{"model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":10,"output_tokens":5}}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = s.write_all(resp.as_bytes()).await;
                let _ = s.write_all(body).await;
                let _ = s.flush().await;
            }
        });

        // 2. Temp DB seeded with a provider pointing at the mock upstream.
        let db = std::env::temp_dir().join(format!("olenro-fwd-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&db);
        let psvc = crate::services::provider::ProviderService::new(db.clone());
        psvc.add_provider(
            "Mock",
            &format!("http://{}", up_addr),
            crate::provider::ProviderCategory::Custom,
            Some("test-key"),
        )
        .unwrap();

        // 3. Find a free port for the proxy.
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_port = probe.local_addr().unwrap().port();
        drop(probe);

        let mut proxy = ProxyService::new_with_db(
            ProxyConfig {
                enabled: true,
                port: proxy_port,
                address: "127.0.0.1".to_string(),
            },
            db.clone(),
        );
        proxy.start().await.unwrap();
        assert!(proxy.resolved_target().is_some());
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // 4. Send a request through the proxy.
        let mut client = tokio::net::TcpStream::connect(("127.0.0.1", proxy_port))
            .await
            .unwrap();
        let req = "POST /v1/messages HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n{}";
        client.write_all(req.as_bytes()).await.unwrap();
        let mut resp = Vec::new();
        client.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("input_tokens"), "resp: {}", resp_str);

        proxy.stop().await.unwrap();

        // 5. Usage was recorded from the upstream response.
        let usage = crate::services::usage::UsageService::new(db.clone());
        let s = usage.summary(30).unwrap();
        assert_eq!(s.requests, 1);
        assert_eq!(s.input_tokens, 10);
        assert_eq!(s.output_tokens, 5);
        // Cost computed from the sonnet pricing: 10@$3/Mtok + 5@$15/Mtok.
        let expected = 10.0 / 1e6 * 3.0 + 5.0 / 1e6 * 15.0;
        assert!((s.cost - expected).abs() < 1e-12, "cost was {}", s.cost);

        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn claude_takeover_roundtrip() {
        use crate::provider::AppType;

        let home = std::env::temp_dir().join(format!("olenro-takeover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        // Pre-existing endpoint that must be restored on disable.
        std::fs::write(
            home.join(".claude").join("settings.json"),
            r#"{"env":{"ANTHROPIC_BASE_URL":"https://api.anthropic.com"}}"#,
        )
        .unwrap();
        std::env::set_var("OLENRO_TEST_HOME", &home);

        let proxy = ProxyService::new(ProxyConfig {
            enabled: true,
            port: 15721,
            address: "127.0.0.1".to_string(),
        });

        assert!(!proxy.is_taken_over(AppType::Claude));

        // Enable: points at the proxy.
        proxy.set_takeover(AppType::Claude, true).unwrap();
        assert!(proxy.is_taken_over(AppType::Claude));
        let v: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.join(".claude").join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            v["env"]["ANTHROPIC_BASE_URL"].as_str(),
            Some("http://127.0.0.1:15721")
        );

        // Disable: restores the original endpoint.
        proxy.set_takeover(AppType::Claude, false).unwrap();
        assert!(!proxy.is_taken_over(AppType::Claude));
        let v: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.join(".claude").join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            v["env"]["ANTHROPIC_BASE_URL"].as_str(),
            Some("https://api.anthropic.com")
        );

        std::env::remove_var("OLENRO_TEST_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }
}
