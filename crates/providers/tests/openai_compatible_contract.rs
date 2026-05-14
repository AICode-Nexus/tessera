use tessera_protocol::{ItemId, ProviderId, RunEvent};
use tessera_providers::openai_compatible::{events_from_sse_data, OpenAiCompatibleProvider};

#[test]
fn openai_compatible_sse_maps_reasoning_content_and_usage() {
    let item_id = ItemId::new();

    let reasoning = events_from_sse_data(
        &item_id,
        r#"{"choices":[{"delta":{"reasoning_content":"think first"}}]}"#,
    )
    .unwrap();
    let content =
        events_from_sse_data(&item_id, r#"{"choices":[{"delta":{"content":"hello"}}]}"#).unwrap();
    let usage = events_from_sse_data(
        &item_id,
        r#"{"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":3,"total_tokens":13,"prompt_tokens_details":{"cached_tokens":4},"prompt_cache_miss_tokens":6}}"#,
    )
    .unwrap();
    let done = events_from_sse_data(&item_id, "[DONE]").unwrap();

    assert!(matches!(
        reasoning.as_slice(),
        [RunEvent::AssistantReasoningDelta { text, .. }] if text == "think first"
    ));
    assert!(matches!(
        content.as_slice(),
        [RunEvent::AssistantDelta { text, .. }] if text == "hello"
    ));
    assert!(matches!(
        usage.as_slice(),
        [RunEvent::UsageReported {
            input_tokens: Some(10),
            output_tokens: Some(3),
            total_tokens: Some(13),
            cache_read_tokens: Some(4),
            cache_miss_tokens: Some(6),
            ..
        }]
    ));
    assert!(done.is_empty());
}

#[test]
fn openai_compatible_debug_redacts_api_key() {
    let provider = OpenAiCompatibleProvider::new(
        "https://api.example.test/v1",
        Some("sk-secret-test-value".to_string()),
        ProviderId::from_static("openai-compatible"),
    );

    let debug = format!("{provider:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("sk-secret-test-value"));
}
