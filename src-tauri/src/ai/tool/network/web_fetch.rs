/*!
 * 网络请求命令模块
 *
 * 提供无头 HTTP 请求功能，绕过浏览器的 CORS 限制
 */

use html2text::from_read;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tauri::command;

type WebFetchResult<T> = std::result::Result<T, String>;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebFetchRequest {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub timeout: Option<u64>,
    pub follow_redirects: Option<bool>,
    pub response_format: Option<String>,
    pub extract_content: Option<bool>,
    pub max_content_length: Option<usize>,
    // 智能内容提取参数
    pub use_jina_reader: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebFetchResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub data: String,
    pub response_time: u64,
    pub final_url: String,
    pub success: bool,
    pub error: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub extracted_text: Option<String>,
    pub page_title: Option<String>,
}

/// 执行无头 HTTP 请求
#[command]
pub async fn network_web_fetch_headless(
    request: WebFetchRequest,
) -> WebFetchResult<WebFetchResponse> {
    let start_time = std::time::Instant::now();

    // 验证 URL
    let url = match reqwest::Url::parse(&request.url) {
        Ok(url) => url,
        Err(e) => {
            let error_msg = format!("无效的 URL: {}", e);
            tracing::error!("❌ [WebFetch] {}", error_msg);
            return Ok(WebFetchResponse {
                status: 0,
                status_text: "Invalid URL".to_string(),
                headers: HashMap::new(),
                data: String::new(),
                response_time: start_time.elapsed().as_millis() as u64,
                final_url: request.url,
                success: false,
                error: Some(error_msg),
                content_type: None,
                content_length: None,
                extracted_text: None,
                page_title: None,
            });
        }
    };

    // 构建 HTTP 客户端
    #[cfg(debug_assertions)]
    let client_builder = reqwest::Client::builder()
        .timeout(Duration::from_millis(request.timeout.unwrap_or(10000)))
        .redirect(if request.follow_redirects.unwrap_or(true) {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        })
        .user_agent("OrbitX-Agent/1.0")
        .danger_accept_invalid_certs(true);

    #[cfg(not(debug_assertions))]
    let client_builder = reqwest::Client::builder()
        .timeout(Duration::from_millis(request.timeout.unwrap_or(10000)))
        .redirect(if request.follow_redirects.unwrap_or(true) {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        })
        .user_agent("OrbitX-Agent/1.0");

    let client = match client_builder.build() {
        Ok(client) => client,
        Err(e) => {
            let error_msg = format!("创建 HTTP 客户端失败: {}", e);
            tracing::error!("❌ [WebFetch] {}", error_msg);
            return Ok(WebFetchResponse {
                status: 0,
                status_text: "Client Error".to_string(),
                headers: HashMap::new(),
                data: String::new(),
                response_time: start_time.elapsed().as_millis() as u64,
                final_url: request.url,
                success: false,
                error: Some(error_msg),
                content_type: None,
                content_length: None,
                extracted_text: None,
                page_title: None,
            });
        }
    };

    // 构建请求
    let method = match request
        .method
        .as_deref()
        .unwrap_or("GET")
        .to_uppercase()
        .as_str()
    {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        _ => reqwest::Method::GET,
    };

    let mut request_builder = client.request(method, url);

    // 添加请求头
    if let Some(headers) = &request.headers {
        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
        }
    }

    // 添加请求体
    if let Some(body) = &request.body {
        request_builder = request_builder.body(body.clone());
    }

    // 发送请求
    match request_builder.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let status_text = response
                .status()
                .canonical_reason()
                .unwrap_or("Unknown")
                .to_string();
            let final_url = response.url().to_string();

            let mut headers = HashMap::new();
            for (key, value) in response.headers() {
                if let Ok(value_str) = value.to_str() {
                    headers.insert(key.to_string(), value_str.to_string());
                }
            }

            let content_type = headers.get("content-type").cloned();

            let raw_data = match response.text().await {
                Ok(text) => text,
                Err(e) => {
                    return Ok(WebFetchResponse {
                        status: 0,
                        status_text: "Read Error".to_string(),
                        headers: HashMap::new(),
                        data: format!("读取响应失败: {}", e),
                        response_time: start_time.elapsed().as_millis() as u64,
                        final_url,
                        success: false,
                        error: Some(format!("读取响应失败: {}", e)),
                        content_type: None,
                        content_length: None,
                        extracted_text: None,
                        page_title: None,
                    })
                }
            };

            let content_length = Some(raw_data.len());
            let extract_content = request.extract_content.unwrap_or(true);
            let max_length = request.max_content_length.unwrap_or(2000);

            // 内容提取（仅对 HTML 内容）
            let (extracted_text, page_title) = if extract_content
                && content_type
                    .as_ref()
                    .is_some_and(|ct| ct.contains("text/html"))
            {
                // 使用改进的内容提取算法
                let (text, title) = extract_content_from_html_improved(&raw_data, max_length);
                (Some(text), title)
            } else {
                (None, None)
            };

            let final_data = if extract_content && extracted_text.is_some() {
                create_content_summary(extracted_text.as_ref().unwrap())
            } else {
                match request.response_format.as_deref().unwrap_or("text") {
                    "json" => {
                        match serde_json::from_str::<serde_json::Value>(&raw_data) {
                            Ok(json) => serde_json::to_string_pretty(&json).unwrap_or(raw_data),
                            Err(_) => raw_data, // 如果不是有效的 JSON，返回原始文本
                        }
                    }
                    _ => {
                        // 限制原始数据长度
                        if raw_data.len() > max_length {
                            format!(
                                "{}...\n\n[内容被截断，总长度: {} 字符]",
                                &raw_data[..max_length],
                                raw_data.len()
                            )
                        } else {
                            raw_data
                        }
                    }
                }
            };

            let response_time = start_time.elapsed().as_millis() as u64;

            Ok(WebFetchResponse {
                status,
                status_text,
                headers,
                data: final_data,
                response_time,
                final_url,
                success: (200..400).contains(&status),
                error: None,
                content_type,
                content_length,
                extracted_text,
                page_title,
            })
        }
        Err(e) => {
            let error_msg = format!("请求失败: {}", e);
            tracing::error!("❌ [WebFetch] {}", error_msg);

            Ok(WebFetchResponse {
                status: 0,
                status_text: "Request Failed".to_string(),
                headers: HashMap::new(),
                data: String::new(),
                response_time: start_time.elapsed().as_millis() as u64,
                final_url: request.url,
                success: false,
                error: Some(error_msg),
                content_type: None,
                content_length: None,
                extracted_text: None,
                page_title: None,
            })
        }
    }
}

/// 简化的网络请求命令（只需要 URL）
#[command]
pub async fn network_simple_web_fetch(url: String) -> WebFetchResult<WebFetchResponse> {
    let request = WebFetchRequest {
        url,
        method: Some("GET".to_string()),
        headers: None,
        body: None,
        timeout: Some(15000), // 15秒超时
        follow_redirects: Some(true),
        response_format: Some("text".to_string()),
        extract_content: Some(true),
        max_content_length: Some(2000),
        use_jina_reader: Some(false), // 不使用jina_reader
    };

    network_web_fetch_headless(request).await
}

/// 提取 HTML 内容的主要文本（改进版）
fn extract_content_from_html_improved(html: &str, max_length: usize) -> (String, Option<String>) {
    let document = Html::parse_document(html);

    // 提取页面标题
    let title = if let Ok(title_selector) = Selector::parse("title") {
        document
            .select(&title_selector)
            .next()
            .map(|element| element.text().collect::<String>().trim().to_string())
    } else {
        None
    };

    // 使用 scraper 移除不需要的元素
    let mut cleaned_html = html.to_string();

    // 移除不需要的元素（改进版）
    let unwanted_tags = [
        ("script", "</script>"),
        ("style", "</style>"),
        ("nav", "</nav>"),
        ("header", "</header>"),
        ("footer", "</footer>"),
        ("aside", "</aside>"),
        ("noscript", "</noscript>"),
    ];

    for (start_tag, end_tag) in &unwanted_tags {
        while let Some(start) = cleaned_html.find(&format!("<{}", start_tag)) {
            if let Some(end) = cleaned_html[start..].find(end_tag) {
                let end_pos = start + end + end_tag.len();
                cleaned_html.replace_range(start..end_pos, "");
            } else {
                break;
            }
        }
    }

    // 转换为纯文本
    let text = from_read(cleaned_html.as_bytes(), max_length);

    // 清理多余的空白字符
    let cleaned_text = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // 限制长度
    let final_text = if cleaned_text.len() > max_length {
        format!(
            "{}...\n\n[内容被截断，原始长度: {} 字符]",
            &cleaned_text[..max_length],
            cleaned_text.len()
        )
    } else {
        cleaned_text
    };

    (final_text, title)
}

/// 智能内容摘要
fn create_content_summary(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if total_lines <= 50 {
        return content.to_string();
    }

    // 取前20行和后10行，中间显示省略信息
    let mut summary = Vec::new();

    // 前20行
    for line in lines.iter().take(20) {
        summary.push(line.to_string());
    }

    // 省略信息
    summary.push(format!("\n... [省略了 {} 行内容] ...\n", total_lines - 30));

    // 后10行
    for line in lines.iter().skip(total_lines - 10) {
        summary.push(line.to_string());
    }

    summary.join("\n")
}
