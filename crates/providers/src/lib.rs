use async_trait::async_trait;
use futures::{stream, Stream};
use std::pin::Pin;
use tessera_protocol::{
    CostEstimate, ItemId, ModelProfileId, ProviderCapability, ProviderId, RunEvent,
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
            let assistant_item_id = request.assistant_item_id;
            let response = format!("{} to: {}", self.response_prefix, request.prompt);
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderRequest {
    pub provider_id: ProviderId,
    pub profile_id: ModelProfileId,
    pub model: String,
    pub prompt: String,
    pub assistant_item_id: ItemId,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("provider error: {0}")]
    Message(String),
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
