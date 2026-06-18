use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub id: String,
    pub from: String,
    pub to: String,
    pub act: String,
    pub message_type: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentMailbox {
    messages: VecDeque<MessageEnvelope>,
    delivered: usize,
    processed: usize,
}

impl AgentMailbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, message: MessageEnvelope) {
        self.messages.push_back(message);
        self.delivered += 1;
    }

    pub fn pop(&mut self) -> Option<MessageEnvelope> {
        let message = self.messages.pop_front();
        if message.is_some() {
            self.processed += 1;
        }
        message
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn delivered(&self) -> usize {
        self.delivered
    }

    pub fn processed(&self) -> usize {
        self.processed
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentMailbox, MessageEnvelope};
    use serde_json::json;

    fn message(id: &str) -> MessageEnvelope {
        MessageEnvelope {
            id: id.into(),
            from: "User".into(),
            to: "Worker".into(),
            act: "tell".into(),
            message_type: "Ping".into(),
            payload: json!({}),
        }
    }

    #[test]
    fn creates_empty_mailbox() {
        let mailbox = AgentMailbox::new();
        assert!(mailbox.is_empty());
        assert_eq!(mailbox.len(), 0);
    }

    #[test]
    fn push_and_pop_are_fifo() {
        let mut mailbox = AgentMailbox::new();
        mailbox.push(message("msg_001"));
        mailbox.push(message("msg_002"));

        assert_eq!(mailbox.pop().unwrap().id, "msg_001");
        assert_eq!(mailbox.pop().unwrap().id, "msg_002");
        assert!(mailbox.is_empty());
    }
}
