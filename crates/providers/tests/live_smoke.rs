use futures::TryStreamExt;
use tessera_protocol::{ItemId, ModelProfileId, ProviderId, RunEvent};
use tessera_providers::{
    ollama::OllamaProvider, openai_compatible::OpenAiCompatibleProvider, ChatProvider,
    ProviderRequest,
};

#[tokio::test]
#[ignore = "requires TESSERA_OPENAI_COMPATIBLE_BASE_URL and a reachable model endpoint"]
async fn openai_compatible_live_smoke_streams_standard_events() {
    let base_url = std::env::var("TESSERA_OPENAI_COMPATIBLE_BASE_URL")
        .expect("TESSERA_OPENAI_COMPATIBLE_BASE_URL is required");
    let model = std::env::var("TESSERA_OPENAI_COMPATIBLE_MODEL")
        .unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let api_key = std::env::var("TESSERA_OPENAI_COMPATIBLE_API_KEY").ok();
    let provider_id = ProviderId::from_static("openai-compatible-live");
    let assistant_item_id = ItemId::new();

    let provider = OpenAiCompatibleProvider::new(base_url, api_key, provider_id.clone());
    let events: Vec<RunEvent> = provider
        .stream_chat(ProviderRequest {
            provider_id,
            profile_id: ModelProfileId::from_static("openai-compatible-live"),
            model,
            prompt: "Reply with one short sentence.".to_string(),
            assistant_item_id,
        })
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();

    assert!(events
        .iter()
        .any(|event| matches!(event, RunEvent::AssistantDelta { .. })));
}

#[tokio::test]
#[ignore = "requires a reachable Ollama server and model"]
async fn ollama_live_smoke_streams_standard_events() {
    let base_url = std::env::var("TESSERA_OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let model = std::env::var("TESSERA_OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
    let provider_id = ProviderId::from_static("ollama-live");
    let assistant_item_id = ItemId::new();

    let provider = OllamaProvider::new(base_url, provider_id.clone());
    let events: Vec<RunEvent> = provider
        .stream_chat(ProviderRequest {
            provider_id,
            profile_id: ModelProfileId::from_static("ollama-live"),
            model,
            prompt: "Reply with one short sentence.".to_string(),
            assistant_item_id,
        })
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();

    assert!(events
        .iter()
        .any(|event| matches!(event, RunEvent::AssistantDelta { .. })));
}
