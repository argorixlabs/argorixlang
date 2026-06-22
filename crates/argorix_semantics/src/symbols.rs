use argorix_parser::ast::CapabilityLevel;
use std::collections::{HashMap, HashSet};

pub const EXTERNAL_ENTITIES: [&str; 5] = ["User", "System", "Runtime", "Memory", "Tool"];
pub const COMMUNICATIVE_ACTS: [&str; 9] = [
    "tell", "ask", "propose", "commit", "reject", "verify", "delegate", "observe", "handoff",
];
pub const PRIMITIVE_TYPES: [&str; 4] = ["string", "bool", "int", "float"];

#[derive(Debug, Default)]
pub struct Symbols {
    pub providers: HashSet<String>,
    pub types: HashSet<String>,
    pub enums: HashSet<String>,
    pub agents: HashSet<String>,
    pub capabilities: HashMap<String, CapabilityLevel>,
    pub tools: HashSet<String>,
    pub models: HashSet<String>,
}

impl Symbols {
    pub fn is_participant(&self, name: &str) -> bool {
        self.agents.contains(name) || EXTERNAL_ENTITIES.contains(&name)
    }

    pub fn is_field_type(&self, name: &str) -> bool {
        self.types.contains(name) || self.enums.contains(name) || PRIMITIVE_TYPES.contains(&name)
    }
}
