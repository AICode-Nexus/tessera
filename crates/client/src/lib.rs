//! UI-neutral client model for Tessera shells.

use serde::{Deserialize, Serialize};
use tessera_protocol::{
    ArtifactId, ArtifactKind, EventFrame, ItemId, RunEvent, TaskId, TaskKind, TaskStatus, ThreadId,
    Timestamp, TraceRecord, TurnId,
};

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

/// UI-neutral task projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientTask {
    pub task_id: TaskId,
    pub kind: Option<TaskKind>,
    pub status: TaskStatus,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub created_at: Option<Timestamp>,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    pub cancel_reason: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl ClientTask {
    fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            kind: None,
            status: TaskStatus::Pending,
            thread_id: None,
            turn_id: None,
            created_at: None,
            started_at: None,
            finished_at: None,
            cancel_reason: None,
            error_code: None,
            error_message: None,
        }
    }

    fn update_scope(&mut self, thread_id: Option<ThreadId>, turn_id: Option<TurnId>) {
        if thread_id.is_some() {
            self.thread_id = thread_id;
        }
        if turn_id.is_some() {
            self.turn_id = turn_id;
        }
    }
}

/// UI-neutral artifact handle projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientArtifact {
    pub artifact_id: ArtifactId,
    pub kind: Option<ArtifactKind>,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub task_id: Option<TaskId>,
    pub item_id: Option<ItemId>,
    pub created_at: Option<Timestamp>,
    pub referenced_by_event_kinds: Vec<String>,
}

impl ClientArtifact {
    fn new(artifact_id: ArtifactId) -> Self {
        Self {
            artifact_id,
            kind: None,
            thread_id: None,
            turn_id: None,
            task_id: None,
            item_id: None,
            created_at: None,
            referenced_by_event_kinds: Vec::new(),
        }
    }

    fn update_scope(
        &mut self,
        thread_id: Option<ThreadId>,
        turn_id: Option<TurnId>,
        task_id: Option<TaskId>,
        item_id: Option<ItemId>,
    ) {
        if thread_id.is_some() {
            self.thread_id = thread_id;
        }
        if turn_id.is_some() {
            self.turn_id = turn_id;
        }
        if task_id.is_some() {
            self.task_id = task_id;
        }
        if item_id.is_some() {
            self.item_id = item_id;
        }
    }

    fn record_reference(&mut self, event_kind: &str) {
        if !self
            .referenced_by_event_kinds
            .iter()
            .any(|existing| existing == event_kind)
        {
            self.referenced_by_event_kinds.push(event_kind.to_string());
        }
    }
}

/// Provider-neutral telemetry projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientTelemetrySummary {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cache_miss_tokens: u64,
    pub cache_total_tokens: u64,
    pub latest_context_tokens: Option<u64>,
    pub max_context_tokens: Option<u64>,
    pub estimated_cost: Option<f64>,
    pub cost_currency: Option<String>,
    pub cost_currency_mixed: bool,
}

#[derive(Clone, Copy, Debug)]
struct UsageTelemetryInput<'a> {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    total_tokens: Option<u64>,
    cache_read_tokens: Option<u64>,
    cache_write_tokens: Option<u64>,
    cache_miss_tokens: Option<u64>,
    cost_amount: Option<f64>,
    cost_currency: Option<&'a str>,
}

impl ClientTelemetrySummary {
    fn record_capability(&mut self, max_context_tokens: Option<u64>) {
        if max_context_tokens.is_some() {
            self.max_context_tokens = max_context_tokens;
        }
    }

    fn record_usage(&mut self, usage: UsageTelemetryInput<'_>) {
        if let Some(input_tokens) = usage.input_tokens {
            self.input_tokens = self.input_tokens.saturating_add(input_tokens);
            self.latest_context_tokens = Some(input_tokens);
        }
        if let Some(output_tokens) = usage.output_tokens {
            self.output_tokens = self.output_tokens.saturating_add(output_tokens);
        }
        let reported_total =
            usage
                .total_tokens
                .or_else(|| match (usage.input_tokens, usage.output_tokens) {
                    (Some(input), Some(output)) => Some(input.saturating_add(output)),
                    (Some(input), None) => Some(input),
                    (None, Some(output)) => Some(output),
                    (None, None) => None,
                });
        if let Some(total_tokens) = reported_total {
            self.total_tokens = self.total_tokens.saturating_add(total_tokens);
        }
        if let Some(cache_read_tokens) = usage.cache_read_tokens {
            self.cache_read_tokens = self.cache_read_tokens.saturating_add(cache_read_tokens);
        }
        if let Some(cache_write_tokens) = usage.cache_write_tokens {
            self.cache_write_tokens = self.cache_write_tokens.saturating_add(cache_write_tokens);
        }
        if let Some(cache_miss_tokens) = usage.cache_miss_tokens {
            self.cache_miss_tokens = self.cache_miss_tokens.saturating_add(cache_miss_tokens);
        }
        if usage.cache_read_tokens.is_some() || usage.cache_miss_tokens.is_some() {
            let cache_read_tokens = usage.cache_read_tokens.unwrap_or_default();
            let cache_miss_tokens = usage.cache_miss_tokens.unwrap_or_default();
            let reported_cache_total = cache_read_tokens.saturating_add(cache_miss_tokens);
            let cache_total_tokens =
                if usage.cache_read_tokens.is_some() && usage.cache_miss_tokens.is_none() {
                    usage.input_tokens.unwrap_or(reported_cache_total)
                } else if reported_cache_total > 0 {
                    reported_cache_total
                } else {
                    usage.input_tokens.unwrap_or(reported_cache_total)
                };
            self.cache_total_tokens = self.cache_total_tokens.saturating_add(cache_total_tokens);
        }
        if let (Some(amount), Some(currency)) = (usage.cost_amount, usage.cost_currency) {
            self.record_cost(amount, currency);
        }
    }

    fn record_cost(&mut self, amount: f64, currency: &str) {
        if let Some(existing_currency) = &self.cost_currency {
            if existing_currency == currency && !self.cost_currency_mixed {
                let total = self.estimated_cost.unwrap_or_default() + amount;
                self.estimated_cost = Some(total);
                return;
            }

            self.cost_currency_mixed = true;
            self.estimated_cost = None;
            return;
        }

        self.cost_currency = Some(currency.to_string());
        self.estimated_cost = Some(amount);
    }

    fn usage_summary(&self) -> String {
        format!(
            "usage in {} / out {} / total {}",
            self.input_tokens, self.output_tokens, self.total_tokens
        )
    }

    fn cache_summary(&self) -> String {
        if self.cache_total_tokens == 0 {
            return "cache 0/0".to_string();
        }

        let percentage = self.cache_read_tokens.saturating_mul(100) / self.cache_total_tokens;
        format!(
            "cache {}/{} ({percentage}%)",
            self.cache_read_tokens, self.cache_total_tokens
        )
    }

    fn cost_summary(&self) -> String {
        if self.cost_currency_mixed {
            return "cost mixed".to_string();
        }

        match (self.estimated_cost, &self.cost_currency) {
            (Some(amount), Some(currency)) => format!("{currency} {amount:.4}"),
            _ => "CNY 0.0000".to_string(),
        }
    }

    fn context_summary(&self) -> String {
        let Some(context_tokens) = self.latest_context_tokens else {
            return "ctx 0 tokens".to_string();
        };

        match self.max_context_tokens {
            Some(max_context_tokens) if max_context_tokens > 0 => {
                let percentage = context_tokens.saturating_mul(100) / max_context_tokens;
                format!("ctx {context_tokens}/{max_context_tokens} ({percentage}%)")
            }
            _ => format!("ctx {context_tokens} tokens"),
        }
    }
}

/// UI-neutral status projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientStatus {
    pub active_profile: String,
    pub available_profiles: Vec<String>,
    pub reasoning_visible: bool,
    pub task_summary: String,
    pub artifact_summary: String,
    pub usage_summary: String,
    pub cache_summary: String,
    pub cost_summary: String,
    pub context_summary: String,
    #[serde(default)]
    pub telemetry: ClientTelemetrySummary,
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
            task_summary: "task idle".to_string(),
            artifact_summary: "artifacts 0".to_string(),
            usage_summary: "usage in 0 / out 0 / total 0".to_string(),
            cache_summary: "cache 0/0".to_string(),
            cost_summary: "CNY 0.0000".to_string(),
            context_summary: "ctx 0 tokens".to_string(),
            telemetry: ClientTelemetrySummary::default(),
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

    fn update_provider_capability(&mut self, max_context_tokens: Option<u64>) {
        self.telemetry.record_capability(max_context_tokens);
        self.refresh_telemetry_summaries();
    }

    fn update_usage(&mut self, usage: UsageTelemetryInput<'_>) {
        self.telemetry.record_usage(usage);
        self.refresh_telemetry_summaries();
    }

    fn reset_telemetry(&mut self) {
        self.telemetry = ClientTelemetrySummary::default();
        self.refresh_telemetry_summaries();
    }

    fn refresh_telemetry_summaries(&mut self) {
        self.usage_summary = self.telemetry.usage_summary();
        self.cache_summary = self.telemetry.cache_summary();
        self.cost_summary = self.telemetry.cost_summary();
        self.context_summary = self.telemetry.context_summary();
    }

    fn update_task_summary(&mut self, tasks: &[ClientTask]) {
        self.task_summary = latest_task_summary(tasks);
    }

    fn update_artifact_summary(&mut self, artifacts: &[ClientArtifact]) {
        self.artifact_summary = format!("artifacts {}", artifacts.len());
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientSnapshot {
    pub status: ClientStatus,
    pub projection: ClientProjection,
    pub tasks: Vec<ClientTask>,
    pub artifacts: Vec<ClientArtifact>,
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
            tasks: Vec::new(),
            artifacts: Vec::new(),
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
        match &frame.event {
            RunEvent::TaskCreated { task_id, kind } => {
                let thread_id = frame.thread_id.clone();
                let turn_id = frame.turn_id.clone();
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.kind = Some(kind.clone());
                task.status = TaskStatus::Pending;
                task.created_at = Some(timestamp);
                task.finished_at = None;
                task.cancel_reason = None;
                task.error_code = None;
                task.error_message = None;
                task.update_scope(thread_id, turn_id);
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskStarted { task_id } => {
                let thread_id = frame.thread_id.clone();
                let turn_id = frame.turn_id.clone();
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Running;
                task.started_at = Some(timestamp);
                task.update_scope(thread_id, turn_id);
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskCompleted { task_id } => {
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Completed;
                task.finished_at = Some(timestamp);
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskFailed { task_id, error } => {
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Failed;
                task.finished_at = Some(timestamp);
                task.error_code = Some(error.code.clone());
                task.error_message = Some(error.message.clone());
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskCancelled { task_id, reason } => {
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Cancelled;
                task.finished_at = Some(timestamp);
                task.cancel_reason = reason.clone();
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::ArtifactCreated { artifact_id, kind } => {
                let thread_id = frame.thread_id.clone();
                let turn_id = frame.turn_id.clone();
                let task_id = frame.task_id.clone();
                let item_id = frame.item_id.clone();
                let timestamp = frame.timestamp.clone();
                let artifact = self.artifact_mut_or_insert(artifact_id);
                artifact.kind = Some(kind.clone());
                artifact.created_at = Some(timestamp);
                artifact.update_scope(thread_id, turn_id, task_id, item_id);
                self.status.update_artifact_summary(&self.artifacts);
            }
            RunEvent::ProviderCapabilityReported { capability, .. } => self
                .status
                .update_provider_capability(capability.max_context_tokens),
            RunEvent::UsageReported {
                input_tokens,
                output_tokens,
                total_tokens,
                cache_read_tokens,
                cache_write_tokens,
                cache_miss_tokens,
                estimated_cost,
                ..
            } => {
                self.status.update_usage(UsageTelemetryInput {
                    input_tokens: *input_tokens,
                    output_tokens: *output_tokens,
                    total_tokens: *total_tokens,
                    cache_read_tokens: *cache_read_tokens,
                    cache_write_tokens: *cache_write_tokens,
                    cache_miss_tokens: *cache_miss_tokens,
                    cost_amount: estimated_cost.as_ref().map(|cost| cost.amount),
                    cost_currency: estimated_cost.as_ref().map(|cost| cost.currency.as_str()),
                });
            }
            _ => {}
        }
        self.apply_artifact_refs_from_frame(frame);
        self.projection.reasoning_visible = self.status.reasoning_visible;
        self.projection.apply_event(frame);
    }

    pub fn apply_trace_record(&mut self, record: &TraceRecord) {
        match record.event_kind.as_str() {
            "task_created" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let kind = record
                    .payload
                    .get("kind")
                    .and_then(|value| value.as_str())
                    .and_then(TaskKind::from_snake_case);
                let task = self.task_mut_or_insert(&task_id);
                task.kind = kind;
                task.status = TaskStatus::Pending;
                task.created_at = Some(record.timestamp.clone());
                task.finished_at = None;
                task.cancel_reason = None;
                task.error_code = None;
                task.error_message = None;
                task.update_scope(record.thread_id.clone(), record.turn_id.clone());
                self.status.update_task_summary(&self.tasks);
            }
            "task_started" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Running;
                task.started_at = Some(record.timestamp.clone());
                task.update_scope(record.thread_id.clone(), record.turn_id.clone());
                self.status.update_task_summary(&self.tasks);
            }
            "task_completed" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Completed;
                task.finished_at = Some(record.timestamp.clone());
                self.status.update_task_summary(&self.tasks);
            }
            "task_failed" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Failed;
                task.finished_at = Some(record.timestamp.clone());
                task.error_code = record
                    .payload
                    .get("error")
                    .and_then(|error| error.get("code"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                task.error_message = record
                    .payload
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                self.status.update_task_summary(&self.tasks);
            }
            "task_cancelled" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Cancelled;
                task.finished_at = Some(record.timestamp.clone());
                task.cancel_reason = record
                    .payload
                    .get("reason")
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                self.status.update_task_summary(&self.tasks);
            }
            "artifact_created" => {
                let Some(artifact_id) = trace_record_artifact_id(record) else {
                    return;
                };
                let kind = record
                    .payload
                    .get("kind")
                    .and_then(|value| value.as_str())
                    .and_then(ArtifactKind::from_snake_case);
                let artifact = self.artifact_mut_or_insert(&artifact_id);
                artifact.kind = kind;
                artifact.created_at = Some(record.timestamp.clone());
                artifact.update_scope(
                    record.thread_id.clone(),
                    record.turn_id.clone(),
                    record.task_id.clone(),
                    record.item_id.clone(),
                );
                self.status.update_artifact_summary(&self.artifacts);
            }
            "provider_capability_reported" => self.status.update_provider_capability(
                record
                    .payload
                    .get("capability")
                    .and_then(|value| value.get("max_context_tokens"))
                    .and_then(|value| value.as_u64()),
            ),
            "usage_reported" => {
                let estimated_cost = record.payload.get("estimated_cost");
                self.status.update_usage(UsageTelemetryInput {
                    input_tokens: record
                        .payload
                        .get("input_tokens")
                        .and_then(|value| value.as_u64()),
                    output_tokens: record
                        .payload
                        .get("output_tokens")
                        .and_then(|value| value.as_u64()),
                    total_tokens: record
                        .payload
                        .get("total_tokens")
                        .and_then(|value| value.as_u64()),
                    cache_read_tokens: record
                        .payload
                        .get("cache_read_tokens")
                        .and_then(|value| value.as_u64()),
                    cache_write_tokens: record
                        .payload
                        .get("cache_write_tokens")
                        .and_then(|value| value.as_u64()),
                    cache_miss_tokens: record
                        .payload
                        .get("cache_miss_tokens")
                        .and_then(|value| value.as_u64()),
                    cost_amount: estimated_cost
                        .and_then(|value| value.get("amount"))
                        .and_then(|value| value.as_f64()),
                    cost_currency: estimated_cost
                        .and_then(|value| value.get("currency"))
                        .and_then(|value| value.as_str()),
                });
            }
            _ => {}
        }
        self.apply_artifact_refs_from_record(record);
        self.projection.reasoning_visible = self.status.reasoning_visible;
        self.projection.apply_trace_record(record);
    }

    pub fn start_new_thread(&mut self) {
        self.projection = ClientProjection::new(self.status.active_profile.clone());
        self.tasks.clear();
        self.artifacts.clear();
        self.draft_input.clear();
        self.status.reset_telemetry();
        self.status.update_task_summary(&self.tasks);
        self.status.update_artifact_summary(&self.artifacts);
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

    fn task_mut_or_insert(&mut self, task_id: &TaskId) -> &mut ClientTask {
        if let Some(index) = self.tasks.iter().position(|task| &task.task_id == task_id) {
            return &mut self.tasks[index];
        }

        self.tasks.push(ClientTask::new(task_id.clone()));
        self.tasks
            .last_mut()
            .expect("task was just inserted into non-empty registry")
    }

    fn artifact_mut_or_insert(&mut self, artifact_id: &ArtifactId) -> &mut ClientArtifact {
        if let Some(index) = self
            .artifacts
            .iter()
            .position(|artifact| &artifact.artifact_id == artifact_id)
        {
            return &mut self.artifacts[index];
        }

        self.artifacts
            .push(ClientArtifact::new(artifact_id.clone()));
        self.artifacts
            .last_mut()
            .expect("artifact was just inserted into non-empty registry")
    }

    fn apply_artifact_refs_from_frame(&mut self, frame: &EventFrame) {
        if frame.artifact_refs.is_empty() {
            return;
        }

        let event_kind = frame.event.kind().to_string();
        let thread_id = frame.thread_id.clone();
        let turn_id = frame.turn_id.clone();
        let task_id = frame.task_id.clone();
        let item_id = frame.item_id.clone();
        for artifact_id in &frame.artifact_refs {
            let artifact = self.artifact_mut_or_insert(artifact_id);
            artifact.update_scope(
                thread_id.clone(),
                turn_id.clone(),
                task_id.clone(),
                item_id.clone(),
            );
            artifact.record_reference(&event_kind);
        }
        self.status.update_artifact_summary(&self.artifacts);
    }

    fn apply_artifact_refs_from_record(&mut self, record: &TraceRecord) {
        if record.artifact_refs.is_empty() {
            return;
        }

        let event_kind = record.event_kind.clone();
        let thread_id = record.thread_id.clone();
        let turn_id = record.turn_id.clone();
        let task_id = record.task_id.clone();
        let item_id = record.item_id.clone();
        for artifact_id in &record.artifact_refs {
            let artifact = self.artifact_mut_or_insert(artifact_id);
            artifact.update_scope(
                thread_id.clone(),
                turn_id.clone(),
                task_id.clone(),
                item_id.clone(),
            );
            artifact.record_reference(&event_kind);
        }
        self.status.update_artifact_summary(&self.artifacts);
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

fn trace_record_task_id(record: &TraceRecord) -> Option<TaskId> {
    record.task_id.clone().or_else(|| {
        record
            .payload
            .get("task_id")
            .and_then(|value| value.as_str())
            .map(TaskId::from)
    })
}

fn trace_record_artifact_id(record: &TraceRecord) -> Option<ArtifactId> {
    record
        .payload
        .get("artifact_id")
        .and_then(|value| value.as_str())
        .map(ArtifactId::from)
}

fn latest_task_summary(tasks: &[ClientTask]) -> String {
    let Some(task) = tasks.last() else {
        return "task idle".to_string();
    };

    format!("task {}", task_status_label(&task.status))
}

fn task_status_label(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::WaitingForApproval => "waiting",
        TaskStatus::Paused => "paused",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}
