use futures::TryStreamExt;
use tessera_protocol::{ItemId, ModelProfileId, ProviderId, RunEvent};
use tessera_providers::{mock::MockProvider, ChatProvider, ProviderMessage, ProviderRequest};

#[tokio::test]
async fn mock_provider_reports_capability_and_streams_standard_events() {
    let provider = MockProvider::default();
    let capability = provider.capability().await.unwrap();
    assert!(capability.supports_streaming);
    assert!(capability.supports_reasoning_delta);

    let assistant_item_id = ItemId::new();
    let events: Vec<RunEvent> = provider
        .stream_chat(ProviderRequest {
            provider_id: ProviderId::from_static("mock"),
            profile_id: ModelProfileId::from_static("mock-default"),
            model: "mock-chat".to_string(),
            prompt: "hello".to_string(),
            messages: vec![
                ProviderMessage::user("prior question"),
                ProviderMessage::assistant("prior answer"),
                ProviderMessage::user("hello"),
            ],
            assistant_item_id: assistant_item_id.clone(),
        })
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();

    assert!(matches!(
        events.first(),
        Some(RunEvent::AssistantMessageStarted { item_id }) if item_id == &assistant_item_id
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        RunEvent::AssistantReasoningDelta { text, .. } if text.contains("mock reasoning")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        RunEvent::AssistantDelta { text, .. } if text.contains("history messages: 3")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        RunEvent::UsageReported {
            cache_read_tokens: Some(0),
            latency_ms: Some(_),
            ..
        }
    )));
}
