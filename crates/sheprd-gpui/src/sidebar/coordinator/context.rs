//! Display-only phase hints never conceal live work or a live block.
use sheprd_client::protocol::AgentStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Live(AgentStatus),
    NeedsDecision,
    Ready,
}

pub(super) fn phase(status: AgentStatus, tokens: &[(String, String)]) -> Phase {
    if matches!(status, AgentStatus::Working | AgentStatus::Blocked) {
        return Phase::Live(status);
    }
    match token(tokens, "phase") {
        Some("needs-decision") => Phase::NeedsDecision,
        Some("ready") if status == AgentStatus::Idle => Phase::Ready,
        _ => Phase::Live(status),
    }
}

pub(super) fn task(tokens: &[(String, String)]) -> Option<&str> {
    token(tokens, "task").filter(|value| !value.is_empty())
}

fn token<'a>(tokens: &'a [(String, String)], key: &str) -> Option<&'a str> {
    tokens
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

#[cfg(test)]
#[path = "context/tests.rs"]
mod tests;
