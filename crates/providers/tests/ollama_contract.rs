use tessera_protocol::{ErrorSource, ItemId, ProviderId, RunEvent};
use tessera_providers::normalize_provider_http_error;
use tessera_providers::ollama::events_from_json_line;

#[test]
fn ollama_jsonl_maps_content_and_final_usage() {
    let item_id = ItemId::new();

    let content = events_from_json_line(
        &item_id,
        r#"{"model":"llama","message":{"role":"assistant","content":"hi"},"done":false}"#,
    )
    .unwrap();
    let usage = events_from_json_line(
        &item_id,
        r#"{"model":"llama","done":true,"total_duration":12000000,"prompt_eval_count":7,"eval_count":5}"#,
    )
    .unwrap();

    assert!(matches!(
        content.as_slice(),
        [RunEvent::AssistantDelta { text, .. }] if text == "hi"
    ));
    assert!(matches!(
        usage.as_slice(),
        [RunEvent::UsageReported {
            input_tokens: Some(7),
            output_tokens: Some(5),
            total_tokens: Some(12),
            latency_ms: Some(12),
            ..
        }]
    ));
}

#[test]
fn ollama_error_body_maps_model_not_found() {
    let error = normalize_provider_http_error(
        ProviderId::from_static("ollama"),
        reqwest::StatusCode::NOT_FOUND,
        r#"{"error":"model 'missing-model' not found"}"#,
    );

    assert_eq!(error.code, "provider_model_not_found");
    assert_eq!(error.source, ErrorSource::Provider);
    assert!(!error.retryable);
    assert_eq!(error.message, "model 'missing-model' not found");
    let details = error.details.as_ref().unwrap();
    assert_eq!(details["provider_id"], "ollama");
    assert_eq!(details["http_status"], 404);
}

#[test]
fn retryable_http_statuses_are_normalized_as_unavailable() {
    let error = normalize_provider_http_error(
        ProviderId::from_static("ollama"),
        reqwest::StatusCode::SERVICE_UNAVAILABLE,
        "server is temporarily busy",
    );

    assert_eq!(error.code, "provider_unavailable");
    assert_eq!(error.source, ErrorSource::Provider);
    assert!(error.retryable);
}
