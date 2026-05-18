//! Typed bridge DTOs for the future Tauri GUI shell.
//!
//! This crate deliberately stays on the client side of the runtime boundary. It
//! can project mock/replay data into `tessera-client` snapshots, but it must not
//! call provider SDKs, read SQLite internals, execute tools, or own runtime work.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use tessera_client::{ClientIntent, ClientSnapshot};
use tessera_protocol::{EventFrame, ItemId, RunEvent, TaskId, TraceRecord};
use thiserror::Error;

pub const GUI_IPC_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GuiRuntimeMode {
    MockReplay,
    ReadOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct GuiProfile {
    pub id: String,
    pub label: String,
    pub mode: GuiRuntimeMode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct GuiShellState {
    pub ipc_version: u32,
    pub mode: GuiRuntimeMode,
    pub event_buffer_capacity: usize,
    pub profiles: Vec<GuiProfile>,
    pub snapshot: ClientSnapshot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct GuiCommandOutcome {
    pub accepted: bool,
    pub notice: Option<String>,
    pub snapshot: ClientSnapshot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum GuiEvent {
    SnapshotUpdated { snapshot: Box<ClientSnapshot> },
    Notice { message: String },
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum GuiBridgeError {
    #[error("GUI event buffer is full at capacity {capacity}")]
    Backpressure { capacity: usize },
    #[error("unknown GUI profile {profile_id}")]
    UnknownProfile { profile_id: String },
}

#[derive(Clone, Debug)]
pub struct BoundedGuiEventBuffer {
    capacity: usize,
    events: VecDeque<GuiEvent>,
}

impl BoundedGuiEventBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            events: VecDeque::with_capacity(capacity),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn push(&mut self, event: GuiEvent) -> Result<(), GuiBridgeError> {
        if self.events.len() >= self.capacity {
            return Err(GuiBridgeError::Backpressure {
                capacity: self.capacity,
            });
        }
        self.events.push_back(event);
        Ok(())
    }

    pub fn drain(&mut self) -> Vec<GuiEvent> {
        self.events.drain(..).collect()
    }
}

#[derive(Clone, Debug)]
pub struct GuiBridge {
    profiles: Vec<GuiProfile>,
    snapshot: ClientSnapshot,
    events: BoundedGuiEventBuffer,
    next_seq: u64,
    next_item_index: u64,
}

impl GuiBridge {
    pub fn new(event_buffer_capacity: usize) -> Self {
        let profiles = vec![
            GuiProfile {
                id: "mock-replay".to_string(),
                label: "Mock Replay".to_string(),
                mode: GuiRuntimeMode::MockReplay,
            },
            GuiProfile {
                id: "read-only".to_string(),
                label: "Read Only Runtime".to_string(),
                mode: GuiRuntimeMode::ReadOnly,
            },
        ];
        let mut bridge = Self {
            profiles,
            snapshot: ClientSnapshot::with_profiles("mock-replay", ["mock-replay", "read-only"]),
            events: BoundedGuiEventBuffer::new(event_buffer_capacity),
            next_seq: 1,
            next_item_index: 1,
        };
        bridge.apply_mock_turn(
            "Show me the GUI runtime boundary.",
            "This mock/replay snapshot is projected through tessera-client; no provider or storage path is active.",
        );
        bridge
    }

    pub fn shell_state(&self) -> GuiShellState {
        GuiShellState {
            ipc_version: GUI_IPC_VERSION,
            mode: self.active_mode(),
            event_buffer_capacity: self.events.capacity(),
            profiles: self.profiles.clone(),
            snapshot: self.snapshot.clone(),
        }
    }

    pub fn list_profiles(&self) -> Vec<GuiProfile> {
        self.profiles.clone()
    }

    pub fn load_client_snapshot(&self) -> ClientSnapshot {
        self.snapshot.clone()
    }

    pub fn submit_client_intent(
        &mut self,
        intent: ClientIntent,
    ) -> Result<GuiCommandOutcome, GuiBridgeError> {
        match intent {
            ClientIntent::SubmitPrompt { profile_id, prompt } => {
                self.ensure_profile(&profile_id)?;
                self.snapshot.status.active_profile = profile_id;
                self.apply_mock_turn(
                    &prompt,
                    "mock/replay response accepted by the GUI bridge. Live provider execution stays outside this spike.",
                );
                self.accept_with_notice("Prompt projected with mock/replay events.")
            }
            ClientIntent::SwitchProfile { profile_id } => {
                self.ensure_profile(&profile_id)?;
                self.snapshot.status.active_profile = profile_id;
                self.accept_with_notice("Profile switched in client projection only.")
            }
            ClientIntent::NewThread => {
                self.snapshot.start_new_thread();
                self.accept_with_notice("Started a new GUI projection thread.")
            }
            ClientIntent::SaveThread => {
                self.accept_with_notice("Save is a GUI intent only in this spike.")
            }
            ClientIntent::ExportThread => {
                self.accept_with_notice("Export is available through the typed export command.")
            }
            ClientIntent::CancelTask { task_id } => self.cancel_task(task_id),
            ClientIntent::PauseTask { task_id } => {
                let task_label = task_id
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "latest running task".to_string());
                self.snapshot.push_notice(format!(
                    "Pause requested for {task_label} as typed metadata; no runtime execution was invoked."
                ));
                self.accept_with_notice(
                    "Pause task intent accepted as typed metadata with no runtime execution.",
                )
            }
            ClientIntent::ResumeTask { task_id } => {
                self.snapshot.push_notice(format!(
                    "Resume requested for {task_id} as typed metadata; no runtime execution was invoked."
                ));
                self.accept_with_notice(
                    "Resume task intent accepted as typed metadata with no runtime execution.",
                )
            }
            ClientIntent::ApproveToolCall { .. } | ClientIntent::DenyToolCall { .. } => self
                .accept_with_notice(
                "Approval intents are typed but not connected to runtime execution in this spike.",
            ),
            ClientIntent::AcceptMemoryProposal { .. }
            | ClientIntent::RejectMemoryProposal { .. } => self.accept_with_notice(
                "Memory proposal intents are typed but not connected to memory runtime in this spike.",
            ),
        }
    }

    pub fn cancel_task(
        &mut self,
        _task_id: Option<TaskId>,
    ) -> Result<GuiCommandOutcome, GuiBridgeError> {
        self.snapshot.push_notice(
            "Cancel requested for mock/replay projection only; no runtime task was executed.",
        );
        self.accept_with_notice("Cancel recorded in mock/replay mode.")
    }

    pub fn load_trace_projection(
        &mut self,
        records: Vec<TraceRecord>,
    ) -> Result<GuiCommandOutcome, GuiBridgeError> {
        self.snapshot.start_new_thread();
        self.snapshot.status.active_profile = "read-only".to_string();
        for record in &records {
            self.snapshot.apply_trace_record(record);
        }
        self.accept_with_notice("Trace records projected through read-only client model.")
    }

    pub fn export_thread(&self) -> String {
        self.snapshot.export_markdown()
    }

    pub fn drain_events(&mut self) -> Vec<GuiEvent> {
        self.events.drain()
    }

    fn ensure_profile(&self, profile_id: &str) -> Result<(), GuiBridgeError> {
        if self.profiles.iter().any(|profile| profile.id == profile_id) {
            return Ok(());
        }
        Err(GuiBridgeError::UnknownProfile {
            profile_id: profile_id.to_string(),
        })
    }

    fn active_mode(&self) -> GuiRuntimeMode {
        self.profiles
            .iter()
            .find(|profile| profile.id == self.snapshot.status.active_profile)
            .map(|profile| profile.mode)
            .unwrap_or(GuiRuntimeMode::MockReplay)
    }

    fn accept_with_notice(
        &mut self,
        notice: impl Into<String>,
    ) -> Result<GuiCommandOutcome, GuiBridgeError> {
        let notice = notice.into();
        self.events.push(GuiEvent::SnapshotUpdated {
            snapshot: Box::new(self.snapshot.clone()),
        })?;
        Ok(GuiCommandOutcome {
            accepted: true,
            notice: Some(notice),
            snapshot: self.snapshot.clone(),
        })
    }

    fn apply_mock_turn(&mut self, prompt: &str, response: &str) {
        let user_item_id = self.next_item_id("user");
        let assistant_item_id = self.next_item_id("assistant");
        for event in [
            RunEvent::UserMessageRecorded {
                item_id: user_item_id.clone(),
                text: prompt.to_string(),
            },
            RunEvent::AssistantMessageStarted {
                item_id: assistant_item_id.clone(),
            },
            RunEvent::AssistantDelta {
                item_id: assistant_item_id.clone(),
                text: response.to_string(),
            },
            RunEvent::AssistantMessageCompleted {
                item_id: assistant_item_id,
            },
        ] {
            let frame = EventFrame::new("trace_gui_mock", self.next_seq, event);
            self.next_seq = self.next_seq.saturating_add(1);
            self.snapshot.apply_event(&frame);
        }
    }

    fn next_item_id(&mut self, role: &str) -> ItemId {
        let item_id = ItemId::from(format!("item_gui_{role}_{}", self.next_item_index));
        self.next_item_index = self.next_item_index.saturating_add(1);
        item_id
    }
}

impl Default for GuiBridge {
    fn default() -> Self {
        Self::new(64)
    }
}
