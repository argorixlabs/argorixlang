use crate::{InjectedMessage, VmError};

pub fn parse_injection(value: &str) -> Result<InjectedMessage, VmError> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 4 || parts.iter().any(|part| part.trim().is_empty()) {
        return Err(VmError::InvalidInjection(value.to_owned()));
    }
    Ok(InjectedMessage {
        from: parts[0].to_owned(),
        to: parts[1].to_owned(),
        act: parts[2].to_owned(),
        message_type: parts[3].to_owned(),
    })
}
