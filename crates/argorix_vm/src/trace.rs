use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub vm_version: String,
    pub status: String,
    pub mode: String,
    pub scheduler: String,
    pub steps: Vec<TraceStep>,
    pub mailboxes: Vec<MailboxSummary>,
    pub events: Vec<ExecutionEvent>,
    pub security_checks: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceStep {
    pub index: usize,
    pub from: String,
    pub to: String,
    pub act: String,
    pub message_type: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxSummary {
    pub agent: String,
    pub delivered: usize,
    pub processed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    VmStarted,
    AgentDeclared,
    CapabilityDeclared,
    MessageScheduled,
    MessageDelivered,
    MessageProcessed,
    HandlerExecuted,
    MessageEmitted,
    ValueTraced,
    VmHalted,
    FacuStateCheckpoint,
    MarronCausalGuard,
    ToolDeclared,
    ToolAuthorized,
    ToolCallRequested,
    ToolCallAllowed,
    ToolCallDenied,
    ToolCallDryRunResult,
    ModelDeclared,
    ModelAuthorized,
    ModelCallRequested,
    ModelCallAllowed,
    ModelCallDenied,
    ModelCallDryRunResult,
    AssertionDeclared,
    FailureDeclared,
    AssertionVerified,
    AssertionFailed,
    PolicyReportGenerated,
    FailureModeActivated,
    VmCompleted,
    VmFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectedMessage {
    pub from: String,
    pub to: String,
    pub act: String,
    pub message_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmittedMessage {
    pub message_type: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactiveStep {
    pub index: usize,
    pub agent: String,
    pub handled: String,
    pub emitted: Vec<EmittedMessage>,
    pub traced: Vec<String>,
    pub halted: bool,
    pub intrinsics: Vec<InvokedIntrinsic>,
    pub tool_calls: Vec<String>,
    pub model_calls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvokedIntrinsic {
    pub name: String,
    pub argument: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentStateSummary {
    pub agent: String,
    pub handled_count: usize,
    pub checkpoints: usize,
    pub last_message_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicExecution {
    pub agent: String,
    pub name: String,
    pub argument: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub agent: String,
    pub tool: String,
    pub capability: String,
    pub status: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCallSummary {
    pub agent: String,
    pub model: String,
    pub provider: String,
    pub capability: String,
    pub status: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactiveExecutionTrace {
    pub vm_version: String,
    pub status: String,
    pub mode: String,
    pub scheduler: String,
    pub injected: InjectedMessage,
    pub steps: Vec<ReactiveStep>,
    pub mailboxes: Vec<MailboxSummary>,
    pub agent_state: Vec<AgentStateSummary>,
    pub intrinsics: Vec<IntrinsicExecution>,
    pub tool_calls: Vec<ToolCallSummary>,
    pub model_calls: Vec<ModelCallSummary>,
    pub policy_report: PolicyReport,
    pub events: Vec<ExecutionEvent>,
    pub security_checks: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyReport {
    pub status: String,
    pub assertions: Vec<AssertionResult>,
    pub failures: Vec<FailureActivation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionResult {
    pub name: String,
    pub argument: Option<String>,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureActivation {
    pub name: String,
    pub action: String,
    pub trace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub index: usize,
    pub event_type: EventType,
    pub from: Option<String>,
    pub to: Option<String>,
    pub act: Option<String>,
    pub message_type: Option<String>,
    pub status: String,
    pub details: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventFields {
    pub from: Option<String>,
    pub to: Option<String>,
    pub act: Option<String>,
    pub message_type: Option<String>,
}

impl EventFields {
    pub fn message(from: &str, to: &str, act: &str, message_type: &str) -> Self {
        Self {
            from: Some(from.to_owned()),
            to: Some(to.to_owned()),
            act: Some(act.to_owned()),
            message_type: Some(message_type.to_owned()),
        }
    }

    pub fn target(to: &str) -> Self {
        Self {
            to: Some(to.to_owned()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceLedger {
    pub events: Vec<ExecutionEvent>,
}

impl TraceLedger {
    pub fn record(
        &mut self,
        event_type: EventType,
        status: impl Into<String>,
        details: impl Into<String>,
        fields: EventFields,
    ) {
        self.events.push(ExecutionEvent {
            index: self.events.len() + 1,
            event_type,
            from: fields.from,
            to: fields.to,
            act: fields.act,
            message_type: fields.message_type,
            status: status.into(),
            details: details.into(),
        });
    }
}
