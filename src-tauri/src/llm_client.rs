use crate::settings::{PostProcessProvider, CLAUDE_CODE_LOCAL_PROVIDER_ID};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use log::debug;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct JsonSchema {
    name: String,
    strict: bool,
    schema: Value,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    json_schema: JsonSchema,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct ReasoningConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningConfig>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

/// Is this the synthetic Claude Code (local subscription) provider?
///
/// Identified by the reserved `claude-code-local://` base URL. Checking the
/// URL (not just the id) means a user who renames the custom provider won't
/// accidentally route through the subprocess.
pub(crate) fn is_claude_code_local(provider: &PostProcessProvider) -> bool {
    provider.id == CLAUDE_CODE_LOCAL_PROVIDER_ID
        && provider.base_url.starts_with("claude-code-local://")
}

/// Build headers for API requests based on provider type
fn build_headers(provider: &PostProcessProvider, api_key: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();

    // Common headers
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://github.com/cjpais/Handy"),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Handy/1.0 (+https://github.com/cjpais/Handy)"),
    );
    headers.insert("X-Title", HeaderValue::from_static("Handy"));

    // Provider-specific auth headers
    if !api_key.is_empty() {
        if provider.id == "anthropic" {
            headers.insert(
                "x-api-key",
                HeaderValue::from_str(api_key)
                    .map_err(|e| format!("Invalid API key header value: {}", e))?,
            );
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        } else {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .map_err(|e| format!("Invalid authorization header value: {}", e))?,
            );
        }
    }

    Ok(headers)
}

/// Create an HTTP client with provider-specific headers
fn create_client(provider: &PostProcessProvider, api_key: &str) -> Result<reqwest::Client, String> {
    let headers = build_headers(provider, api_key)?;
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

/// Send a chat completion request to an OpenAI-compatible API
/// Returns Ok(Some(content)) on success, Ok(None) if response has no content,
/// or Err on actual errors (HTTP, parsing, etc.)
pub async fn send_chat_completion(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    prompt: String,
    reasoning_effort: Option<String>,
    reasoning: Option<ReasoningConfig>,
) -> Result<Option<String>, String> {
    send_chat_completion_with_schema(
        provider,
        api_key,
        model,
        prompt,
        None,
        None,
        reasoning_effort,
        reasoning,
    )
    .await
}

/// Send a chat completion request with structured output support
/// When json_schema is provided, uses structured outputs mode
/// system_prompt is used as the system message when provided
/// reasoning_effort sets the OpenAI-style top-level field (e.g., "none", "low", "medium", "high")
/// reasoning sets the OpenRouter-style nested object (effort + exclude)
pub async fn send_chat_completion_with_schema(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    user_content: String,
    system_prompt: Option<String>,
    json_schema: Option<Value>,
    reasoning_effort: Option<String>,
    reasoning: Option<ReasoningConfig>,
) -> Result<Option<String>, String> {
    // Claude Code (local subscription) bypasses HTTP entirely. Handle it here so
    // both post-processing (send_chat_completion and this fn) and AI-mode
    // (send_chat_completion_multimodal) all route through the local CLI.
    if is_claude_code_local(provider) {
        let _ = (api_key, model, json_schema, reasoning_effort, reasoning);
        debug!("Routing chat completion through local Claude Code CLI");
        let reply =
            crate::claude_code::run_claude_code(&user_content, None, system_prompt.as_deref())
                .await?;
        return Ok(Some(reply));
    }

    let base_url = provider.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base_url);

    debug!("Sending chat completion request to: {}", url);

    let client = create_client(provider, &api_key)?;

    // Build messages vector
    let mut messages = Vec::new();

    // Add system prompt if provided
    if let Some(system) = system_prompt {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system,
        });
    }

    // Add user message
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: user_content,
    });

    // Build response_format if schema is provided
    let response_format = json_schema.map(|schema| ResponseFormat {
        format_type: "json_schema".to_string(),
        json_schema: JsonSchema {
            name: "transcription_output".to_string(),
            strict: true,
            schema,
        },
    });

    let request_body = ChatCompletionRequest {
        model: model.to_string(),
        messages,
        response_format,
        reasoning_effort,
        reasoning,
    };

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        return Err(format!(
            "API request failed with status {}: {}",
            status, error_text
        ));
    }

    let completion: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(completion
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone()))
}

/// Send a chat completion with a multimodal user message (text + optional PNG
/// image). Uses the OpenAI-compatible multipart-content format — supported by
/// OpenAI, Anthropic's OpenAI compatibility layer, OpenRouter, and most others.
pub async fn send_chat_completion_multimodal(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    system_prompt: Option<String>,
    user_text: String,
    image_png_bytes: Option<&[u8]>,
) -> Result<Option<String>, String> {
    // Claude Code (local subscription) is a synthetic provider — it delegates
    // to the locally-installed `claude` CLI instead of making HTTP requests.
    if is_claude_code_local(provider) {
        let _ = (api_key, model); // Unused — the CLI uses the user's `claude login` credentials.
        debug!("Routing multimodal request through local Claude Code CLI");
        let reply = crate::claude_code::run_claude_code(
            &user_text,
            image_png_bytes,
            system_prompt.as_deref(),
        )
        .await?;
        return Ok(Some(reply));
    }

    let base_url = provider.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base_url);

    debug!(
        "Sending multimodal chat completion to: {} (image_bytes={})",
        url,
        image_png_bytes.map(<[u8]>::len).unwrap_or(0)
    );

    let client = create_client(provider, &api_key)?;

    let mut messages: Vec<Value> = Vec::new();
    if let Some(system) = system_prompt {
        messages.push(json!({ "role": "system", "content": system }));
    }

    let user_content: Value = if let Some(bytes) = image_png_bytes {
        let b64 = BASE64.encode(bytes);
        json!([
            { "type": "text", "text": user_text },
            {
                "type": "image_url",
                "image_url": { "url": format!("data:image/png;base64,{}", b64) }
            }
        ])
    } else {
        Value::String(user_text)
    };

    messages.push(json!({ "role": "user", "content": user_content }));

    let body = json!({
        "model": model,
        "messages": messages,
    });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        return Err(format!(
            "API request failed with status {}: {}",
            status, text
        ));
    }

    let completion: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(completion
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone()))
}

/// Fetch available models from an OpenAI-compatible API
/// Returns a list of model IDs
pub async fn fetch_models(
    provider: &PostProcessProvider,
    api_key: String,
) -> Result<Vec<String>, String> {
    let base_url = provider.base_url.trim_end_matches('/');
    let url = format!("{}/models", base_url);

    debug!("Fetching models from: {}", url);

    let client = create_client(provider, &api_key)?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!(
            "Model list request failed ({}): {}",
            status, error_text
        ));
    }

    let parsed: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let mut models = Vec::new();

    // Handle OpenAI format: { data: [ { id: "..." }, ... ] }
    if let Some(data) = parsed.get("data").and_then(|d| d.as_array()) {
        for entry in data {
            if let Some(id) = entry.get("id").and_then(|i| i.as_str()) {
                models.push(id.to_string());
            } else if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                models.push(name.to_string());
            }
        }
    }
    // Handle array format: [ "model1", "model2", ... ]
    else if let Some(array) = parsed.as_array() {
        for entry in array {
            if let Some(model) = entry.as_str() {
                models.push(model.to_string());
            }
        }
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_provider() -> PostProcessProvider {
        PostProcessProvider {
            id: CLAUDE_CODE_LOCAL_PROVIDER_ID.to_string(),
            label: "Claude Code (local subscription)".to_string(),
            base_url: "claude-code-local://".to_string(),
            allow_base_url_edit: false,
            models_endpoint: None,
            supports_structured_output: false,
        }
    }

    #[test]
    fn claude_code_local_routing_matches_synthetic_provider() {
        assert!(is_claude_code_local(&local_provider()));
    }

    #[test]
    fn claude_code_local_routing_rejects_mismatched_id() {
        let mut p = local_provider();
        p.id = "custom".to_string();
        assert!(
            !is_claude_code_local(&p),
            "a custom provider with the sentinel URL should not be routed to the CLI"
        );
    }

    #[test]
    fn claude_code_local_routing_rejects_mismatched_url() {
        let mut p = local_provider();
        p.base_url = "http://localhost:11434/v1".to_string();
        assert!(
            !is_claude_code_local(&p),
            "the provider id alone should not be enough — URL must match too"
        );
    }

    #[test]
    fn regular_http_providers_are_not_treated_as_local() {
        let anthropic = PostProcessProvider {
            id: "anthropic".to_string(),
            label: "Anthropic".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: false,
        };
        assert!(!is_claude_code_local(&anthropic));
    }
}
