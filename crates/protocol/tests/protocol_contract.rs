use tessera_protocol::{
    CostEstimate, EventFrame, ItemId, ModelProfileId, ProviderCapability, ProviderId,
    RouteDecision, RouteDecisionId, RouteStrategy, RunEvent,
};

#[test]
fn event_frame_serializes_reasoning_delta_with_artifact_refs() {
    let item_id = ItemId::new();
    let frame = EventFrame::new(
        "trace_test",
        1,
        RunEvent::AssistantReasoningDelta {
            item_id: item_id.clone(),
            text: "planning".to_string(),
        },
    )
    .with_item_id(item_id)
    .with_artifact_ref(tessera_protocol::ArtifactId::new());

    let record = frame.to_trace_record();
    assert_eq!(record.schema_version, 1);
    assert_eq!(record.event_kind, "assistant_reasoning_delta");
    assert_eq!(record.payload["text"], "planning");
    assert_eq!(record.artifact_refs.len(), 1);
}

#[test]
fn usage_and_route_decision_keep_ai_ready_telemetry() {
    let usage = RunEvent::UsageReported {
        input_tokens: Some(10),
        output_tokens: Some(4),
        total_tokens: Some(14),
        cache_read_tokens: Some(8),
        cache_write_tokens: Some(2),
        cache_miss_tokens: Some(0),
        estimated_cost: Some(CostEstimate {
            amount: 0.001,
            currency: "CNY".to_string(),
            input_cost: Some(0.0004),
            output_cost: Some(0.0006),
            cache_read_cost: Some(0.0),
            cache_write_cost: Some(0.0),
        }),
        latency_ms: Some(32),
    };

    let usage_record = EventFrame::new("trace_usage", 1, usage).to_trace_record();
    assert_eq!(usage_record.event_kind, "usage_reported");
    assert_eq!(usage_record.payload["cache_read_tokens"], 8);
    assert_eq!(usage_record.payload["estimated_cost"]["currency"], "CNY");

    let capability = ProviderCapability {
        provider_id: ProviderId::from_static("mock"),
        supports_streaming: true,
        supports_reasoning_delta: true,
        supports_cache_telemetry: true,
        supports_cost_estimate: true,
        supports_tool_calling: false,
        max_context_tokens: Some(1_000_000),
        extension: None,
    };
    let route = RunEvent::RouteDecisionRecorded {
        decision_id: RouteDecisionId::new(),
        decision: RouteDecision {
            requested_profile: Some(ModelProfileId::from_static("mock-default")),
            selected_profile: ModelProfileId::from_static("mock-default"),
            selected_model: "mock-chat".to_string(),
            reasoning_level: Some("standard".to_string()),
            strategy: RouteStrategy::Manual,
            fallback_reason: None,
        },
    };

    let capability_record = EventFrame::new(
        "trace_capability",
        1,
        RunEvent::ProviderCapabilityReported {
            provider_id: ProviderId::from_static("mock"),
            capability,
        },
    )
    .to_trace_record();
    let route_record = EventFrame::new("trace_route", 1, route).to_trace_record();

    assert_eq!(capability_record.event_kind, "provider_capability_reported");
    assert_eq!(
        capability_record.payload["capability"]["supports_reasoning_delta"],
        true
    );
    assert_eq!(route_record.event_kind, "route_decision_recorded");
    assert_eq!(route_record.payload["decision"]["strategy"], "manual");
}
