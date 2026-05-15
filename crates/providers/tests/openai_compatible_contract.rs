use tessera_protocol::{ErrorSource, ItemId, NormalizedError, ProviderId, RunEvent};
use tessera_providers::openai_compatible::{events_from_sse_data, OpenAiCompatibleProvider};
use tessera_providers::{normalize_provider_http_error, ProviderError};

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

#[test]
fn openai_compatible_error_body_normalizes_and_redacts_provider_secret() {
    let error = normalize_provider_http_error(
        ProviderId::from_static("openai-compatible"),
        reqwest::StatusCode::UNAUTHORIZED,
        r#"{"error":{"message":"Invalid API key: sk-secret-test-value-1234567890","type":"invalid_request_error","code":"invalid_api_key"}}"#,
    );

    assert_eq!(error.code, "provider_authentication_failed");
    assert_eq!(error.source, ErrorSource::Provider);
    assert!(!error.retryable);
    assert!(error.message.contains("<redacted>"));
    assert!(!error.message.contains("sk-secret-test-value"));
    let details = error.details.as_ref().unwrap();
    assert_eq!(details["provider_id"], "openai-compatible");
    assert_eq!(details["http_status"], 401);
    assert_eq!(details["provider_error_code"], "invalid_api_key");
    assert_eq!(details["provider_error_type"], "invalid_request_error");
}

#[test]
fn provider_error_redacts_authorization_and_cookie_material() {
    let error = normalize_provider_http_error(
        ProviderId::from_static("openai-compatible"),
        reqwest::StatusCode::BAD_REQUEST,
        r#"{"error":{"message":"Authorization: Bearer bearer-secret Cookie: session=secret-cookie request failed","code":"bad_request"}}"#,
    );

    assert!(!error.message.contains("bearer-secret"));
    assert!(!error.message.contains("secret-cookie"));
    assert!(error.message.contains("Authorization: <redacted>"));
    assert!(error.message.contains("Cookie: <redacted>"));
}

#[test]
fn provider_error_exposes_normalized_parse_errors() {
    let json_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let normalized = ProviderError::Json(json_error).normalized();

    assert_eq!(normalized.code, "provider_parse_error");
    assert_eq!(normalized.source, ErrorSource::Provider);
    assert!(!normalized.retryable);
}

#[test]
fn provider_message_display_does_not_expose_raw_secret_material() {
    let error = ProviderError::Message("provider returned sk-secret-display-token".to_string());

    assert!(!error.to_string().contains("sk-secret-display-token"));

    let error = ProviderError::Normalized(NormalizedError {
        code: "provider_error".to_string(),
        message: "provider returned sk-secret-display-token".to_string(),
        retryable: false,
        source: ErrorSource::Provider,
        details: None,
    });

    assert!(!error.to_string().contains("sk-secret-display-token"));
}
