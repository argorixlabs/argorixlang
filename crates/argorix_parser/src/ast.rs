use crate::span::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub module: Spanned<String>,
    pub capabilities: Vec<CapabilityDecl>,
    pub enums: Vec<EnumDecl>,
    pub types: Vec<TypeDecl>,
    pub agents: Vec<AgentDecl>,
    pub protocols: Vec<ProtocolDecl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityLevel {
    Safe,
    Restricted,
    Dangerous,
}

impl CapabilityLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Restricted => "restricted",
            Self::Dangerous => "dangerous",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDecl {
    pub name: Spanned<String>,
    pub level: Spanned<CapabilityLevel>,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approval {
    Granted,
    Denied,
}

impl Approval {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDecl {
    pub name: Spanned<String>,
    pub variants: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDecl {
    pub name: Spanned<String>,
    pub fields: Vec<FieldDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: Spanned<String>,
    pub field_type: Spanned<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDecl {
    pub name: Spanned<String>,
    pub approval: Option<Spanned<Approval>>,
    pub receives: Vec<ReceiveDecl>,
    pub sends: Vec<SendDecl>,
    pub capabilities: Vec<Spanned<String>>,
    pub handlers: Vec<HandlerDecl>,
}

impl AgentDecl {
    pub fn effective_approval(&self) -> Approval {
        self.approval
            .as_ref()
            .map(|approval| approval.value)
            .unwrap_or(Approval::Denied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerDecl {
    pub message_type: Spanned<String>,
    pub binding: Spanned<String>,
    pub instructions: Vec<HandlerInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerInstruction {
    Emit {
        message_type: Spanned<String>,
        to: Spanned<String>,
    },
    Trace {
        binding: Spanned<String>,
    },
    Halt {
        span: crate::span::Span,
    },
    IntrinsicCall {
        name: Spanned<String>,
        argument: Spanned<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiveDecl {
    pub message_type: Spanned<String>,
    pub from: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendDecl {
    pub message_type: Spanned<String>,
    pub to: Spanned<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolDecl {
    pub name: Spanned<String>,
    pub steps: Vec<ProtocolStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolStep {
    pub from: Spanned<String>,
    pub to: Spanned<String>,
    pub act: Spanned<String>,
    pub message_type: Spanned<String>,
}
