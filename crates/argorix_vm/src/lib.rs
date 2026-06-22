pub mod errors;
pub mod evidence;
pub mod injection;
pub mod mailbox;
pub mod policy;
pub mod reactive;
pub mod runtime;
pub mod scheduler;
pub mod security_report;
pub mod trace;
pub mod vm;

pub use errors::VmError;
pub use injection::parse_injection;
pub use mailbox::{AgentMailbox, MessageEnvelope};
pub use reactive::ReactiveScheduler;
pub use runtime::{
    AgentState, ModelCallEnvelope, RuntimeState, RuntimeStatus, StateCheckpoint, ToolCallEnvelope,
};
pub use scheduler::Scheduler;
pub use security_report::{
    AgentPassportSummary, CallSummary, ExecutionSummary, FeatureFlagsSummary,
    InjectedMessageSummary, IntrinsicSummary, LedgerSummary, MessageContractSummary, PolicySummary,
    ProviderBoundarySummary, ProviderHarnessSummary, SecretBoundariesSummary, SecurityReport,
    SecurityVerdict,
};
pub use trace::{
    AgentStateSummary, AssertionResult, EmittedMessage, EventFields, EventType, ExecutionEvent,
    ExecutionTrace, FailureActivation, InjectedMessage, IntrinsicExecution, InvokedIntrinsic,
    MailboxSummary, ModelCallSummary, PolicyActionResult, PolicyBlockResult, PolicyReport,
    PolicyRuleResult, PolicyViolation, ProviderCallSummary, ProviderContractSummary,
    ProviderSummary, ReactiveExecutionTrace, ReactiveStep, ToolCallSummary, TraceLedger, TraceStep,
};
pub use vm::{ExecutionOutcome, Vm};
