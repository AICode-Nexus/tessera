use crate::{
    drain_utf8_lines, normalize_provider_http_error, ChatProvider, ProviderError,
    ProviderEventStream, ProviderRequest, Result,
};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{json, Value};
use tessera_protocol::{ItemId, ProviderCapability, ProviderId, RunEvent};

#[derive(Clone)]
pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    provider_id: ProviderId,
}

impl std::fmt::Debug for OpenAiCompatibleProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OpenAiCompatibleProvider")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("provider_id", &self.provider_id)
            .finish_non_exhaustive()
    }
}

impl OpenAiCompatibleProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
        provider_id: ProviderId,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key,
            provider_id,
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl ChatProvider for OpenAiCompatibleProvider {
    async fn capability(&self) -> Result<ProviderCapability> {
        Ok(ProviderCapability {
            provider_id: self.provider_id.clone(),
            supports_streaming: true,
            supports_reasoning_delta: true,
            supports_cache_telemetry: true,
            supports_cost_estimate: false,
            supports_tool_calling: false,
            max_context_tokens: None,
            extension: None,
        })
    }

    async fn stream_chat(&self, request: ProviderRequest) -> Result<ProviderEventStream> {
        let body = json!({
            "model": request.model,
            "stream": true,
            "stream_options": { "include_usage": true },
            "messages": [
                { "role": "user", "content": request.prompt }
            ]
        });

        let mut builder = self.client.post(self.endpoint()).json(&body);
        if let Some(api_key) = &self.api_key {
            builder = builder.bearer_auth(api_key);
        }

        let response = builder.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Normalized(normalize_provider_http_error(
                self.provider_id.clone(),
                status,
                &body,
            )));
        }

        let assistant_item_id = request.assistant_item_id;
        let byte_stream = response.bytes_stream();
        let stream = async_stream::try_stream! {
            yield RunEvent::AssistantMessageStarted {
                item_id: assistant_item_id.clone(),
            };

            futures::pin_mut!(byte_stream);
            let mut buffer = Vec::new();
            while let Some(chunk) = byte_stream.next().await {
                let chunk = chunk?;
                buffer.extend_from_slice(&chunk);

                for line in drain_utf8_lines(&mut buffer)? {
                    if let Some(data) = line.strip_prefix("data:") {
                        for event in events_from_sse_data(&assistant_item_id, data.trim())? {
                            yield event;
                        }
                    }
                }
            }

            let trailing = std::str::from_utf8(&buffer)?.trim();
            if let Some(data) = trailing.strip_prefix("data:") {
                for event in events_from_sse_data(&assistant_item_id, data.trim())? {
                    yield event;
                }
            }

            yield RunEvent::AssistantMessageCompleted {
                item_id: assistant_item_id,
            };
        };

        Ok(Box::pin(stream))
    }
}

pub fn events_from_sse_data(item_id: &ItemId, data: &str) -> Result<Vec<RunEvent>> {
    if data.is_empty() || data == "[DONE]" {
        return Ok(Vec::new());
    }

    let value: Value = serde_json::from_str(data)?;
    let mut events = Vec::new();

    if let Some(delta) = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("delta"))
    {
        if let Some(reasoning) =
            first_string(delta, &["reasoning_content", "reasoning", "reasoning_text"])
        {
            events.push(RunEvent::AssistantReasoningDelta {
                item_id: item_id.clone(),
                text: reasoning.to_string(),
            });
        }
        if let Some(content) = first_string(delta, &["content"]) {
            events.push(RunEvent::AssistantDelta {
                item_id: item_id.clone(),
                text: content.to_string(),
            });
        }
    }

    if let Some(usage) = value.get("usage").filter(|usage| !usage.is_null()) {
        let input_tokens = usage.get("prompt_tokens").and_then(Value::as_u64);
        let output_tokens = usage.get("completion_tokens").and_then(Value::as_u64);
        let total_tokens = usage.get("total_tokens").and_then(Value::as_u64);
        let cache_read_tokens = usage
            .get("prompt_tokens_details")
            .and_then(|details| details.get("cached_tokens"))
            .and_then(Value::as_u64)
            .or_else(|| usage.get("prompt_cache_hit_tokens").and_then(Value::as_u64));
        let cache_miss_tokens = usage
            .get("prompt_cache_miss_tokens")
            .and_then(Value::as_u64);

        events.push(RunEvent::UsageReported {
            input_tokens,
            output_tokens,
            total_tokens,
            cache_read_tokens,
            cache_write_tokens: None,
            cache_miss_tokens,
            estimated_cost: None,
            latency_ms: None,
        });
    }

    Ok(events)
}

fn first_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .filter(|text| !text.is_empty())
}
