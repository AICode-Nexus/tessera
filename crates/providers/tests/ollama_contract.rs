use tessera_protocol::{ItemId, RunEvent};
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
