use super::types::{OAuthError, OAuthResult, PkceCodes};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, error, info, warn};
use url::form_urlencoded;

const OAUTH_PORT: u16 = 1455;
const OAUTH_CALLBACK_PATH: &str = "/auth/callback";

/// OAuth callback waiter
struct PendingOAuthFlow {
    pub pkce: PkceCodes,
    pub sender: oneshot::Sender<OAuthResult<(String, PkceCodes)>>,
}

/// OAuth callback server
pub struct OAuthCallbackServer {
    pending_flows: Arc<Mutex<HashMap<String, PendingOAuthFlow>>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    started: bool,
}

impl OAuthCallbackServer {
    /// Create server (does not start immediately, deferred until first use)
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            pending_flows: Arc::new(Mutex::new(HashMap::new())),
            server_handle: None,
            started: false,
        }))
    }

    /// Ensure server is started
    pub async fn ensure_started(server: Arc<Mutex<Self>>) {
        let mut guard = server.lock().await;
        if guard.started {
            return;
        }
        guard.started = true;
        drop(guard);

        let server_clone = server.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::start_server(server_clone).await {
                error!("Failed to start OAuth callback server: {}", e);
            }
        });
    }

    /// Start HTTP server
    async fn start_server(server: Arc<Mutex<Self>>) -> OAuthResult<()> {
        use tiny_http::{Response, Server};

        let http_server = Server::http(format!("127.0.0.1:{OAUTH_PORT}"))
            .map_err(|e| OAuthError::Other(format!("Failed to bind server: {e}")))?;

        info!("OAuth callback server started on port {}", OAUTH_PORT);

        let server_ref = server.clone();

        for request in http_server.incoming_requests() {
            let url_str = request.url();
            debug!("OAuth callback received: {}", url_str);

            if url_str.starts_with(OAUTH_CALLBACK_PATH) {
                let query_start = url_str.find('?');
                if let Some(start) = query_start {
                    let query = &url_str[start + 1..];
                    let params: HashMap<String, String> = form_urlencoded::parse(query.as_bytes())
                        .into_owned()
                        .collect();

                    let state = params.get("state").map(|s| s.as_str());
                    let code = params.get("code").map(|s| s.as_str());
                    let error = params.get("error").map(|s| s.as_str());

                    let server_lock = server_ref.lock().await;
                    let mut flows = server_lock.pending_flows.lock().await;

                    if let Some(state_val) = state {
                        if let Some(pending) = flows.remove(state_val) {
                            if let Some(err) = error {
                                if pending
                                    .sender
                                    .send(Err(OAuthError::Other(format!("OAuth error: {err}"))))
                                    .is_err()
                                {
                                    warn!("Failed to deliver OAuth error result to waiting flow");
                                }
                                if let Err(respond_err) =
                                    request.respond(Response::from_string(Self::html_error(err)))
                                {
                                    warn!(
                                        "Failed to respond to OAuth callback with provider error: {}",
                                        respond_err
                                    );
                                }
                            } else if let Some(code_val) = code {
                                if pending
                                    .sender
                                    .send(Ok((code_val.to_string(), pending.pkce)))
                                    .is_err()
                                {
                                    warn!("Failed to deliver OAuth authorization code to waiting flow");
                                }
                                if let Err(respond_err) =
                                    request.respond(Response::from_string(Self::html_success()))
                                {
                                    warn!(
                                        "Failed to respond to successful OAuth callback: {}",
                                        respond_err
                                    );
                                }
                            } else {
                                if pending
                                    .sender
                                    .send(Err(OAuthError::Other("Missing code".to_string())))
                                    .is_err()
                                {
                                    warn!("Failed to deliver OAuth missing-code error to waiting flow");
                                }
                                if let Err(respond_err) = request.respond(Response::from_string(
                                    Self::html_error("Missing code"),
                                )) {
                                    warn!(
                                        "Failed to respond to OAuth callback with missing-code error: {}",
                                        respond_err
                                    );
                                }
                            }
                            continue;
                        }
                    }

                    warn!("OAuth callback with invalid or missing state");
                    if let Err(err) =
                        request.respond(Response::from_string(Self::html_error("Invalid state")))
                    {
                        warn!(
                            "Failed to respond to OAuth callback with invalid state: {}",
                            err
                        );
                    }
                } else if let Err(err) =
                    request.respond(Response::from_string(Self::html_error("Invalid request")))
                {
                    warn!("Failed to respond to invalid OAuth request: {}", err);
                }
            } else if let Err(err) =
                request.respond(Response::from_string("Not found").with_status_code(404))
            {
                warn!("Failed to respond to unknown OAuth route: {}", err);
            }
        }

        Ok(())
    }

    /// Register callback waiter
    pub async fn register_flow(
        &mut self,
        state: String,
        pkce: PkceCodes,
    ) -> oneshot::Receiver<OAuthResult<(String, PkceCodes)>> {
        let (sender, receiver) = oneshot::channel();

        let pending = PendingOAuthFlow { pkce, sender };

        self.pending_flows.lock().await.insert(state, pending);

        receiver
    }

    /// Cancel flow
    pub async fn cancel_flow(&mut self, state: &str) {
        self.pending_flows.lock().await.remove(state);
    }

    /// Success page HTML
    fn html_success() -> String {
        r#"<!DOCTYPE html>
<html>
<head>
    <title>OrbitX - Authorization Successful</title>
    <style>
        body { font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #0f172a; color: #e2e8f0; }
        .container { text-align: center; padding: 2rem; }
        h1 { color: #10b981; margin-bottom: 1rem; }
        p { color: #94a3b8; }
    </style>
</head>
<body>
    <div class="container">
        <h1>✓ Authorization Successful</h1>
        <p>You can close this window and return to OrbitX.</p>
    </div>
    <script>setTimeout(() => window.close(), 2000);</script>
</body>
</html>"#.to_string()
    }

    /// Error page HTML
    fn html_error(error: &str) -> String {
        let template = r#"<!DOCTYPE html>
<html>
<head>
    <title>OrbitX - Authorization Failed</title>
    <style>
        body {{ font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #0f172a; color: #e2e8f0; }}
        .container {{ text-align: center; padding: 2rem; }}
        h1 {{ color: #ef4444; margin-bottom: 1rem; }}
        p {{ color: #94a3b8; }}
        .error {{ color: #fca5a5; font-family: monospace; margin-top: 1rem; padding: 1rem; background: rgba(239,68,68,0.1); border-radius: 0.5rem; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>✗ Authorization Failed</h1>
        <p>An error occurred during authorization.</p>
        <div class="error">{error}</div>
    </div>
</body>
</html>"#;
        template.replace("{error}", error)
    }

    /// Get callback URL
    pub fn callback_url() -> String {
        format!("http://localhost:{OAUTH_PORT}{OAUTH_CALLBACK_PATH}")
    }
}

impl Drop for OAuthCallbackServer {
    fn drop(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}
