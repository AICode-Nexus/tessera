//! UI-neutral client model for Tessera shells.

use serde::{Deserialize, Serialize};
use tessera_protocol::{EventFrame, ItemId, RunEvent, TaskId, TraceRecord};

/// User intent shared by CLI/TUI/GUI surfaces before it reaches runtime code.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientIntent {
    SubmitPrompt { profile_id: String, prompt: String },
    SwitchProfile { profile_id: String },
    NewThread,
    SaveThread,
    ExportThread,
    CancelTask { task_id: Option<TaskId> },
}

/// UI-neutral message role for client projections.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientMessageRole {
    System,
    User,
    Assistant,
    Reasoning,
}

/// UI-neutral chat message projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientMessage {
    pub role: ClientMessageRole,
    pub content: String,
    pub item_id: Option<ItemId>,
    pub streaming: bool,
}

/// UI-neutral status projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientStatus {
    pub active_profile: String,
    pub available_profiles: Vec<String>,
    pub reasoning_visible: bool,
    pub cache_summary: String,
    pub cost_summary: String,
}

impl ClientStatus {
    pub fn new(active_profile: impl Into<String>) -> Self {
        let active_profile = active_profile.into();
        Self::with_profiles(active_profile.clone(), [active_profile])
    }

    pub fn with_profiles<I, S>(active_profile: impl Into<String>, profiles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let active_profile = active_profile.into();
        let mut available_profiles = Vec::new();
        for profile in profiles {
            let profile = profile.into();
            if !profile.trim().is_empty() && !available_profiles.contains(&profile) {
                available_profiles.push(profile);
            }
        }
        if !available_profiles.contains(&active_profile) {
            available_profiles.insert(0, active_profile.clone());
        }

        Self {
            active_profile,
            available_profiles,
            reasoning_visible: false,
            cache_summary: "cache 0/0".to_string(),
            cost_summary: "CNY 0.0000".to_string(),
        }
    }

    pub fn active_profile_position(&self) -> (usize, usize) {
        let total = self.available_profiles.len().max(1);
        let index = self
            .available_profiles
            .iter()
            .position(|profile| profile == &self.active_profile)
            .map(|index| index + 1)
            .unwrap_or(1);
        (index, total)
    }

    pub fn cycle_profile(&mut self, offset: isize) -> Option<ClientIntent> {
        let total = self.available_profiles.len();
        if total <= 1 {
            return None;
        }
        let current = self
            .available_profiles
            .iter()
            .position(|profile| profile == &self.active_profile)
            .unwrap_or(0);
        let next = (current as isize + offset).rem_euclid(total as isize) as usize;
        self.active_profile = self.available_profiles[next].clone();
        Some(ClientIntent::SwitchProfile {
            profile_id: self.active_profile.clone(),
        })
    }

    fn update_usage(
        &mut self,
        input_tokens: Option<u64>,
        cache_read_tokens: Option<u64>,
        cache_miss_tokens: Option<u64>,
        cost_amount: Option<f64>,
        cost_currency: Option<&str>,
    ) {
        if let Some(cache_summary) =
            format_cache_summary(input_tokens, cache_read_tokens, cache_miss_tokens)
        {
            self.cache_summary = cache_summary;
        }

        if let (Some(amount), Some(currency)) = (cost_amount, cost_currency) {
            self.cost_summary = format!("{currency} {amount:.4}");
        }
    }
}

/// UI-neutral message projection built from runtime events or trace records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientProjection {
    pub messages: Vec<ClientMessage>,
    pub reasoning_visible: bool,
}

impl ClientProjection {
    pub fn new(_active_profile: impl Into<String>) -> Self {
        Self {
            messages: Vec::new(),
            reasoning_visible: false,
        }
    }

    pub fn apply_event(&mut self, frame: &EventFrame) {
        match &frame.event {
            RunEvent::UserMessageRecorded { item_id, text } => {
                self.messages.push(ClientMessage {
                    role: ClientMessageRole::User,
                    content: text.clone(),
                    item_id: Some(item_id.clone()),
                    streaming: false,
                });
            }
            RunEvent::AssistantMessageStarted { item_id } => {
                self.push_empty_streaming_message(ClientMessageRole::Assistant, item_id.clone());
            }
            RunEvent::AssistantDelta { item_id, text } => {
                self.append_to_streaming_message(ClientMessageRole::Assistant, item_id, text);
            }
            RunEvent::AssistantReasoningDelta { item_id, text } if self.reasoning_visible => {
                self.append_to_streaming_message(ClientMessageRole::Reasoning, item_id, text);
            }
            RunEvent::AssistantMessageCompleted { item_id } => {
                self.complete_assistant_item(item_id);
            }
            _ => {}
        }
    }

    pub fn apply_trace_record(&mut self, record: &TraceRecord) {
        let item_id = trace_record_item_id(record);
        match record.event_kind.as_str() {
            "user_message_recorded" => {
                let Some(text) = record.payload.get("text").and_then(|value| value.as_str()) else {
                    return;
                };
                self.messages.push(ClientMessage {
                    role: ClientMessageRole::User,
                    content: text.to_string(),
                    item_id,
                    streaming: false,
                });
            }
            "assistant_message_started" => {
                let Some(item_id) = item_id else {
                    return;
                };
                self.push_empty_streaming_message(ClientMessageRole::Assistant, item_id);
            }
            "assistant_delta" => {
                let (Some(item_id), Some(text)) = (
                    item_id.as_ref(),
                    record.payload.get("text").and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                self.append_to_streaming_message(ClientMessageRole::Assistant, item_id, text);
            }
            "assistant_reasoning_delta" if self.reasoning_visible => {
                let (Some(item_id), Some(text)) = (
                    item_id.as_ref(),
                    record.payload.get("text").and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                self.append_to_streaming_message(ClientMessageRole::Reasoning, item_id, text);
            }
            "assistant_message_completed" => {
                let Some(item_id) = item_id else {
                    return;
                };
                self.complete_assistant_item(&item_id);
            }
            _ => {}
        }
    }

    fn push_empty_streaming_message(&mut self, role: ClientMessageRole, item_id: ItemId) {
        self.messages.push(ClientMessage {
            role,
            content: String::new(),
            item_id: Some(item_id),
            streaming: true,
        });
    }

    fn append_to_streaming_message(
        &mut self,
        role: ClientMessageRole,
        item_id: &ItemId,
        text: &str,
    ) {
        if let Some(message) = self.message_by_item_id_and_role_mut(item_id, &role) {
            message.content.push_str(text);
            message.streaming = true;
            return;
        }

        self.messages.push(ClientMessage {
            role,
            content: text.to_string(),
            item_id: Some(item_id.clone()),
            streaming: true,
        });
    }

    fn message_by_item_id_and_role_mut(
        &mut self,
        item_id: &ItemId,
        role: &ClientMessageRole,
    ) -> Option<&mut ClientMessage> {
        self.messages
            .iter_mut()
            .rev()
            .find(|message| message.item_id.as_ref() == Some(item_id) && message.role == *role)
    }

    fn complete_assistant_item(&mut self, item_id: &ItemId) {
        for message in self.messages.iter_mut().filter(|message| {
            message.item_id.as_ref() == Some(item_id)
                && matches!(
                    message.role,
                    ClientMessageRole::Assistant | ClientMessageRole::Reasoning
                )
        }) {
            message.streaming = false;
        }
    }
}

/// Complete client-side snapshot for a shell render pass.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientSnapshot {
    pub status: ClientStatus,
    pub projection: ClientProjection,
    pub draft_input: String,
}

impl ClientSnapshot {
    pub fn new(active_profile: impl Into<String>) -> Self {
        let active_profile = active_profile.into();
        Self::with_profiles(active_profile.clone(), [active_profile])
    }

    pub fn with_profiles<I, S>(active_profile: impl Into<String>, profiles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let active_profile = active_profile.into();
        Self {
            status: ClientStatus::with_profiles(active_profile.clone(), profiles),
            projection: ClientProjection::new(active_profile),
            draft_input: String::new(),
        }
    }

    pub fn set_input(&mut self, input: impl Into<String>) {
        self.draft_input = input.into();
    }

    pub fn submit_input(&mut self) -> Option<ClientIntent> {
        let prompt = self.draft_input.trim().to_string();
        if prompt.is_empty() {
            return None;
        }
        self.draft_input.clear();
        match prompt.as_str() {
            "/new" => Some(ClientIntent::NewThread),
            "/save" => Some(ClientIntent::SaveThread),
            "/export" => Some(ClientIntent::ExportThread),
            _ => Some(ClientIntent::SubmitPrompt {
                profile_id: self.status.active_profile.clone(),
                prompt,
            }),
        }
    }

    pub fn active_profile_position(&self) -> (usize, usize) {
        self.status.active_profile_position()
    }

    pub fn cycle_profile(&mut self, offset: isize) -> Option<ClientIntent> {
        self.status.cycle_profile(offset)
    }

    pub fn apply_event(&mut self, frame: &EventFrame) {
        if let RunEvent::UsageReported {
            input_tokens,
            cache_read_tokens,
            cache_miss_tokens,
            estimated_cost,
            ..
        } = &frame.event
        {
            self.status.update_usage(
                *input_tokens,
                *cache_read_tokens,
                *cache_miss_tokens,
                estimated_cost.as_ref().map(|cost| cost.amount),
                estimated_cost.as_ref().map(|cost| cost.currency.as_str()),
            );
        }
        self.projection.reasoning_visible = self.status.reasoning_visible;
        self.projection.apply_event(frame);
    }

    pub fn apply_trace_record(&mut self, record: &TraceRecord) {
        if record.event_kind == "usage_reported" {
            let estimated_cost = record.payload.get("estimated_cost");
            self.status.update_usage(
                record
                    .payload
                    .get("input_tokens")
                    .and_then(|value| value.as_u64()),
                record
                    .payload
                    .get("cache_read_tokens")
                    .and_then(|value| value.as_u64()),
                record
                    .payload
                    .get("cache_miss_tokens")
                    .and_then(|value| value.as_u64()),
                estimated_cost
                    .and_then(|value| value.get("amount"))
                    .and_then(|value| value.as_f64()),
                estimated_cost
                    .and_then(|value| value.get("currency"))
                    .and_then(|value| value.as_str()),
            );
        }
        self.projection.reasoning_visible = self.status.reasoning_visible;
        self.projection.apply_trace_record(record);
    }

    pub fn start_new_thread(&mut self) {
        self.projection = ClientProjection::new(self.status.active_profile.clone());
        self.draft_input.clear();
    }

    pub fn push_notice(&mut self, content: impl Into<String>) {
        self.projection.messages.push(ClientMessage {
            role: ClientMessageRole::System,
            content: content.into(),
            item_id: None,
            streaming: false,
        });
    }

    pub fn export_markdown(&self) -> String {
        let mut output = String::from("# Tessera Export\n\n");
        if self.projection.messages.is_empty() {
            output.push_str("_No messages._\n");
            return output;
        }

        for message in &self.projection.messages {
            let role = match message.role {
                ClientMessageRole::System => "System",
                ClientMessageRole::User => "User",
                ClientMessageRole::Assistant => "Assistant",
                ClientMessageRole::Reasoning => "Reasoning",
            };
            output.push_str("## ");
            output.push_str(role);
            output.push_str("\n\n");
            output.push_str(&message.content);
            output.push_str("\n\n");
        }

        output
    }
}

fn trace_record_item_id(record: &TraceRecord) -> Option<ItemId> {
    record.item_id.clone().or_else(|| {
        record
            .payload
            .get("item_id")
            .and_then(|value| value.as_str())
            .map(ItemId::from)
    })
}

fn format_cache_summary(
    input_tokens: Option<u64>,
    cache_read_tokens: Option<u64>,
    cache_miss_tokens: Option<u64>,
) -> Option<String> {
    let cache_read_tokens = cache_read_tokens?;
    let total_tokens = match (cache_read_tokens, cache_miss_tokens, input_tokens) {
        (read, Some(miss), _) if read + miss > 0 => read + miss,
        (_, _, Some(input)) if input > 0 => input,
        _ => return None,
    };
    let percentage = cache_read_tokens.saturating_mul(100) / total_tokens;
    Some(format!(
        "cache {cache_read_tokens}/{total_tokens} ({percentage}%)"
    ))
}
