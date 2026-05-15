use crate::{
    drain_utf8_lines, normalize_provider_http_error, ChatProvider, ProviderError,
    ProviderEventStream, ProviderRequest, Result,
};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{json, Value};
use tessera_protocol::{ItemId, ProviderCapability, ProviderId, RunEvent};

#[derive(Clone, Debug)]
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    provider_id: ProviderId,
}

impl OllamaProvider {
    pub fn new(base_url: impl Into<String>, provider_id: ProviderId) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            provider_id,
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/api/chat", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl ChatProvider for OllamaProvider {
    async fn capability(&self) -> Result<ProviderCapability> {
        Ok(ProviderCapability {
            provider_id: self.provider_id.clone(),
            supports_streaming: true,
            supports_reasoning_delta: false,
            supports_cache_telemetry: false,
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
            "messages": [
                { "role": "user", "content": request.prompt }
            ]
        });

        let response = self.client.post(self.endpoint()).json(&body).send().await?;
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
                    for event in events_from_json_line(&assistant_item_id, &line)? {
                        yield event;
                    }
                }
            }

            let trailing = std::str::from_utf8(&buffer)?.trim();
            if !trailing.is_empty() {
                for event in events_from_json_line(&assistant_item_id, trailing)? {
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

pub fn events_from_json_line(item_id: &ItemId, line: &str) -> Result<Vec<RunEvent>> {
    if line.trim().is_empty() {
        return Ok(Vec::new());
    }

    let value: Value = serde_json::from_str(line)?;
    let mut events = Vec::new();

    if let Some(content) = value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .filter(|content| !content.is_empty())
    {
        events.push(RunEvent::AssistantDelta {
            item_id: item_id.clone(),
            text: content.to_string(),
        });
    }

    if value.get("done").and_then(Value::as_bool) == Some(true) {
        let input_tokens = value.get("prompt_eval_count").and_then(Value::as_u64);
        let output_tokens = value.get("eval_count").and_then(Value::as_u64);
        let total_tokens = match (input_tokens, output_tokens) {
            (Some(input), Some(output)) => Some(input + output),
            _ => None,
        };
        let latency_ms = value
            .get("total_duration")
            .and_then(Value::as_u64)
            .map(|nanos| nanos / 1_000_000);

        if input_tokens.is_some() || output_tokens.is_some() || latency_ms.is_some() {
            events.push(RunEvent::UsageReported {
                input_tokens,
                output_tokens,
                total_tokens,
                cache_read_tokens: None,
                cache_write_tokens: None,
                cache_miss_tokens: None,
                estimated_cost: None,
                latency_ms,
            });
        }
    }

    Ok(events)
}
