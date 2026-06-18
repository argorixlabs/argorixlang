use argorix_bytecode::BytecodeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("bytecode verification failed: {0}")]
    InvalidBytecode(String),
    #[error("approval denied for agent `{agent}` using capability `{capability}`")]
    ApprovalDenied { agent: String, capability: String },
    #[error("capability `{capability}` required by agent `{agent}` is not declared")]
    UnknownCapability { agent: String, capability: String },
    #[error("dry-run execution halted: {0}")]
    Halted(String),
    #[error("message type must not be empty")]
    EmptyMessageType,
    #[error("communicative act must not be empty")]
    EmptyAct,
    #[error("mailbox for internal agent `{0}` does not exist")]
    MissingMailbox(String),
    #[error("mailbox for agent `{0}` was empty during processing")]
    MailboxEmpty(String),
    #[error("scheduler reached the end of the instruction stream without End")]
    MissingEnd,
    #[error("no handler for message `{message_type}` on agent `{agent}`")]
    MissingHandler { agent: String, message_type: String },
    #[error("invalid injection `{0}`; expected from:to:act:message_type")]
    InvalidInjection(String),
    #[error(
        "intrinsic `{intrinsic}` binding `{argument}` does not match handler binding `{binding}`"
    )]
    IntrinsicBindingMismatch {
        intrinsic: String,
        argument: String,
        binding: String,
    },
    #[error("marron causal guard failed: {0}")]
    CausalGuardFailed(String),
}

impl VmError {
    pub fn from_verification(errors: Vec<BytecodeError>) -> Self {
        Self::InvalidBytecode(
            errors
                .into_iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}
