use super::types::{OAuthError, OAuthResult, PkceCodes};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use tracing::{debug, error, info, warn};
use url::form_urlencoded;

const OAUTH_PORT: u16 = 1455;
const OAUTH_CALLBACK_PATH: &str = "/auth/callback";

/// OAuth 回调等待者
struct PendingOAuthFlow {
    pub pkce: PkceCodes,
    pub sender: oneshot::Sender<OAuthResult<(String, PkceCodes)>>,
}

/// OAuth 回调服务器
pub struct OAuthCallbackServer {
    pending_flows: Arc<Mutex<HashMap<String, PendingOAuthFlow>>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl OAuthCallbackServer {
    /// 创建并启动服务器
    pub fn new() -> Arc<Mutex<Self>> {
        let server = Arc::new(Mutex::new(Self {
            pending_flows: Arc::new(Mutex::new(HashMap::new())),
            server_handle: None,
        }));

        let server_clone = server.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::start_server(server_clone).await {
                error!("Failed to start OAuth callback server: {}", e);
            }
        });

        server
    }

    /// 启动 HTTP 服务器
    async fn start_server(server: Arc<Mutex<Self>>) -> OAuthResult<()> {
        use tiny_http::{Response, Server};

        let http_server = Server::http(format!("127.0.0.1:{}", OAUTH_PORT))
            .map_err(|e| OAuthError::Other(format!("Failed to bind server: {}", e)))?;

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
                                let _ = pending.sender.send(Err(OAuthError::Other(format!("OAuth error: {}", err))));
                                let _ = request.respond(Response::from_string(Self::html_error(err)));
                            } else if let Some(code_val) = code {
                                let _ = pending.sender.send(Ok((code_val.to_string(), pending.pkce)));
                                let _ = request.respond(Response::from_string(Self::html_success()));
                            } else {
                                let _ = pending.sender.send(Err(OAuthError::Other("Missing code".to_string())));
                                let _ = request.respond(Response::from_string(Self::html_error("Missing code")));
                            }
                            continue;
                        }
                    }

                    warn!("OAuth callback with invalid or missing state");
                    let _ = request.respond(Response::from_string(Self::html_error("Invalid state")));
                } else {
                    let _ = request.respond(Response::from_string(Self::html_error("Invalid request")));
                }
            } else {
                let _ = request.respond(Response::from_string("Not found").with_status_code(404));
            }
        }

        Ok(())
    }

    /// 注册等待回调
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

    /// 取消流程
    pub async fn cancel_flow(&mut self, state: &str) {
        self.pending_flows.lock().await.remove(state);
    }

    /// 成功页面 HTML
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

    /// 错误页面 HTML
    fn html_error(error: &str) -> String {
        format!(r#"<!DOCTYPE html>
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
        <div class="error">{}</div>
    </div>
</body>
</html>"#, error)
    }

    /// 获取回调 URL
    pub fn callback_url() -> String {
        format!("http://localhost:{}{}", OAUTH_PORT, OAUTH_CALLBACK_PATH)
    }
}

impl Drop for OAuthCallbackServer {
    fn drop(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}
