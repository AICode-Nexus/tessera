use async_trait::async_trait;
use futures::{stream, Stream};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::pin::Pin;
use tessera_protocol::{
    CostEstimate, ErrorSource, ExtensionMap, ItemId, ModelProfileId, NormalizedError,
    ProviderCapability, ProviderId, RunEvent,
};

pub mod ollama;
pub mod openai_compatible;

pub mod mock {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct MockProvider {
        response_prefix: String,
    }

    impl Default for MockProvider {
        fn default() -> Self {
            Self {
                response_prefix: "mock response".to_string(),
            }
        }
    }

    #[async_trait]
    impl ChatProvider for MockProvider {
        async fn capability(&self) -> Result<ProviderCapability> {
            Ok(ProviderCapability {
                provider_id: ProviderId::from_static("mock"),
                supports_streaming: true,
                supports_reasoning_delta: true,
                supports_cache_telemetry: true,
                supports_cost_estimate: true,
                supports_tool_calling: false,
                max_context_tokens: Some(128_000),
                extension: None,
            })
        }

        async fn stream_chat(&self, request: ProviderRequest) -> Result<ProviderEventStream> {
            let message_count = request.chat_messages().len();
            let assistant_item_id = request.assistant_item_id;
            let response = if message_count > 1 {
                format!(
                    "{} to: {} (history messages: {message_count})",
                    self.response_prefix, request.prompt
                )
            } else {
                format!("{} to: {}", self.response_prefix, request.prompt)
            };
            let events = vec![
                Ok(RunEvent::AssistantMessageStarted {
                    item_id: assistant_item_id.clone(),
                }),
                Ok(RunEvent::AssistantReasoningDelta {
                    item_id: assistant_item_id.clone(),
                    text: "mock reasoning: select deterministic offline response".to_string(),
                }),
                Ok(RunEvent::AssistantDelta {
                    item_id: assistant_item_id.clone(),
                    text: response,
                }),
                Ok(RunEvent::AssistantMessageCompleted {
                    item_id: assistant_item_id,
                }),
                Ok(RunEvent::UsageReported {
                    input_tokens: Some(1),
                    output_tokens: Some(4),
                    total_tokens: Some(5),
                    cache_read_tokens: Some(0),
                    cache_write_tokens: Some(0),
                    cache_miss_tokens: Some(1),
                    estimated_cost: Some(CostEstimate {
                        amount: 0.0,
                        currency: "CNY".to_string(),
                        input_cost: Some(0.0),
                        output_cost: Some(0.0),
                        cache_read_cost: Some(0.0),
                        cache_write_cost: Some(0.0),
                    }),
                    latency_ms: Some(0),
                }),
            ];

            Ok(Box::pin(stream::iter(events)))
        }
    }
}

pub type ProviderEventStream =
    Pin<Box<dyn Stream<Item = std::result::Result<RunEvent, ProviderError>> + Send>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderMessageRole {
    System,
    User,
    Assistant,
}

impl ProviderMessageRole {
    pub fn as_chat_role(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderMessage {
    pub role: ProviderMessageRole,
    pub content: String,
}

impl ProviderMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ProviderMessageRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ProviderMessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ProviderMessageRole::Assistant,
            content: content.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderRequest {
    pub provider_id: ProviderId,
    pub profile_id: ModelProfileId,
    pub model: String,
    pub prompt: String,
    pub messages: Vec<ProviderMessage>,
    pub assistant_item_id: ItemId,
}

impl ProviderRequest {
    pub fn chat_messages(&self) -> Vec<ProviderMessage> {
        if self.messages.is_empty() {
            return vec![ProviderMessage::user(self.prompt.clone())];
        }

        self.messages.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("http request failed")]
    Http(#[from] reqwest::Error),
    #[error("provider stream parse failed")]
    Json(#[from] serde_json::Error),
    #[error("provider stream utf8 decode failed")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("provider error")]
    Normalized(NormalizedError),
    #[error("provider error")]
    Message(String),
}

impl ProviderError {
    pub fn normalized(&self) -> NormalizedError {
        match self {
            Self::Http(error) => {
                let retryable = error.is_timeout() || error.is_connect();
                normalized_provider_error(
                    if error.is_timeout() {
                        "provider_timeout"
                    } else {
                        "provider_http_error"
                    },
                    "provider HTTP request failed",
                    retryable,
                    None,
                )
            }
            Self::Json(_) | Self::Utf8(_) => normalized_provider_error(
                "provider_parse_error",
                "provider stream could not be parsed",
                false,
                None,
            ),
            Self::Normalized(error) => error.clone(),
            Self::Message(message) => {
                normalized_provider_error("provider_error", redact_sensitive(message), false, None)
            }
        }
    }
}

pub fn normalize_provider_http_error(
    provider_id: ProviderId,
    status: StatusCode,
    body: &str,
) -> NormalizedError {
    let parsed = ProviderErrorBody::parse(body);
    let message = parsed
        .message
        .as_deref()
        .map(redact_sensitive)
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("provider HTTP error")
                .to_string()
        });
    let (code, retryable) = classify_http_error(status, &message, parsed.code.as_deref());
    let mut details = ExtensionMap::new();
    details.insert("provider_id".to_string(), json!(provider_id.as_str()));
    details.insert("http_status".to_string(), json!(status.as_u16()));

    if let Some(provider_code) = parsed.code {
        details.insert(
            "provider_error_code".to_string(),
            json!(redact_sensitive(&provider_code)),
        );
    }
    if let Some(provider_error_type) = parsed.error_type {
        details.insert(
            "provider_error_type".to_string(),
            json!(redact_sensitive(&provider_error_type)),
        );
    }

    normalized_provider_error(code, message, retryable, Some(details))
}

fn normalized_provider_error(
    code: impl Into<String>,
    message: impl Into<String>,
    retryable: bool,
    details: Option<ExtensionMap>,
) -> NormalizedError {
    NormalizedError {
        code: code.into(),
        message: message.into(),
        retryable,
        source: ErrorSource::Provider,
        details,
    }
}

fn classify_http_error(
    status: StatusCode,
    message: &str,
    provider_code: Option<&str>,
) -> (&'static str, bool) {
    let status_code = status.as_u16();
    let lowercase_message = message.to_ascii_lowercase();
    let lowercase_provider_code = provider_code.unwrap_or_default().to_ascii_lowercase();

    match status_code {
        400 | 422 => ("provider_bad_request", false),
        401 => ("provider_authentication_failed", false),
        403 => ("provider_permission_denied", false),
        404 if lowercase_message.contains("model") || lowercase_provider_code.contains("model") => {
            ("provider_model_not_found", false)
        }
        404 => ("provider_not_found", false),
        408 => ("provider_timeout", true),
        409 | 425 | 429 => ("provider_rate_limited", true),
        500 | 502 | 503 | 504 => ("provider_unavailable", true),
        _ if status.is_server_error() => ("provider_unavailable", true),
        _ => ("provider_http_error", false),
    }
}

#[derive(Debug, Default)]
struct ProviderErrorBody {
    message: Option<String>,
    code: Option<String>,
    error_type: Option<String>,
}

impl ProviderErrorBody {
    fn parse(body: &str) -> Self {
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Self::default();
        }

        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            return Self {
                message: Some(trimmed.to_string()),
                ..Self::default()
            };
        };

        if let Some(error) = value.get("error") {
            return match error {
                Value::String(message) => Self {
                    message: Some(message.clone()),
                    ..Self::default()
                },
                Value::Object(object) => Self {
                    message: object
                        .get("message")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    code: object
                        .get("code")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    error_type: object
                        .get("type")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                },
                _ => Self::default(),
            };
        }

        Self {
            message: value
                .get("message")
                .or_else(|| value.get("detail"))
                .and_then(Value::as_str)
                .map(str::to_string),
            code: value
                .get("code")
                .and_then(Value::as_str)
                .map(str::to_string),
            error_type: value
                .get("type")
                .and_then(Value::as_str)
                .map(str::to_string),
        }
    }
}

fn redact_sensitive(input: impl AsRef<str>) -> String {
    let input = redact_labeled_value(input.as_ref(), "Authorization:");
    let input = redact_labeled_value(&input, "Cookie:");
    let input = redact_bearer_values(&input);
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while index < input.len() {
        let rest = &input[index..];
        if rest.starts_with("sk-") {
            output.push_str("<redacted>");
            index += redacted_token_len(rest);
        } else {
            let character = rest.chars().next().expect("non-empty string slice");
            output.push(character);
            index += character.len_utf8();
        }
    }

    output
}

fn redact_labeled_value(input: &str, label: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while let Some(relative_start) = input[index..].find(label) {
        let start = index + relative_start;
        output.push_str(&input[index..start]);
        output.push_str(label);
        output.push(' ');
        output.push_str("<redacted>");

        let mut value_start = start + label.len();
        value_start += leading_ascii_whitespace_len(&input[value_start..]);

        if let Some(after_scheme) = input[value_start..].strip_prefix("Bearer") {
            value_start += "Bearer".len();
            value_start += leading_ascii_whitespace_len(after_scheme);
        }

        index = value_start + redacted_value_len(&input[value_start..]);
    }

    output.push_str(&input[index..]);
    output
}

fn redact_bearer_values(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while let Some(relative_start) = input[index..].find("Bearer ") {
        let start = index + relative_start;
        output.push_str(&input[index..start]);
        output.push_str("Bearer <redacted>");

        let value_start = start + "Bearer ".len();
        index = value_start + redacted_value_len(&input[value_start..]);
    }

    output.push_str(&input[index..]);
    output
}

fn leading_ascii_whitespace_len(input: &str) -> usize {
    input
        .char_indices()
        .find_map(|(index, character)| (!character.is_ascii_whitespace()).then_some(index))
        .unwrap_or(input.len())
}

fn redacted_token_len(token_start: &str) -> usize {
    token_start
        .char_indices()
        .find_map(|(index, character)| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                None
            } else {
                Some(index)
            }
        })
        .unwrap_or(token_start.len())
}

fn redacted_value_len(value_start: &str) -> usize {
    value_start
        .char_indices()
        .find_map(|(index, character)| {
            if character.is_ascii_whitespace() || matches!(character, '"' | '\'' | ',' | ';') {
                Some(index)
            } else {
                None
            }
        })
        .unwrap_or(value_start.len())
}

pub type Result<T> = std::result::Result<T, ProviderError>;

#[async_trait]
pub trait ChatProvider: Send + Sync {
    async fn capability(&self) -> Result<ProviderCapability>;
    async fn stream_chat(&self, request: ProviderRequest) -> Result<ProviderEventStream>;
}

pub(crate) fn drain_utf8_lines(
    buffer: &mut Vec<u8>,
) -> std::result::Result<Vec<String>, std::str::Utf8Error> {
    let mut lines = Vec::new();
    while let Some(newline) = buffer.iter().position(|byte| *byte == b'\n') {
        let line_bytes: Vec<_> = buffer.drain(..=newline).collect();
        lines.push(std::str::from_utf8(&line_bytes)?.trim().to_string());
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::drain_utf8_lines;

    #[test]
    fn drain_utf8_lines_waits_for_newline_before_decoding_multibyte_text() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice("data: 你".as_bytes());

        assert!(drain_utf8_lines(&mut buffer).unwrap().is_empty());

        buffer.extend_from_slice("好\n".as_bytes());
        let lines = drain_utf8_lines(&mut buffer).unwrap();

        assert_eq!(lines, vec!["data: 你好"]);
        assert!(buffer.is_empty());
    }
}
