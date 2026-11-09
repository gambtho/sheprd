use super::*;

#[test]
fn task_line_is_opt_in_and_independent_of_phase() {
    assert_eq!(task(&[]), None);
    assert_eq!(task(&hint("ready")), None);
    assert_eq!(task(&[("task".into(), "".into())]), None);
    assert_eq!(
        task(&[("task".into(), "Demo task".into())]),
        Some("Demo task")
    );
}

fn hint(value: &str) -> Vec<(String, String)> {
    vec![("phase".into(), value.into())]
}

#[test]
fn decision_hint_marks_settled_lanes_as_needing_you() {
    for status in [AgentStatus::Idle, AgentStatus::Done, AgentStatus::Unknown] {
        assert_eq!(phase(status, &hint("needs-decision")), Phase::NeedsDecision);
    }
}

#[test]
fn live_work_and_blocks_win_over_phase_hints() {
    for status in [AgentStatus::Working, AgentStatus::Blocked] {
        for value in ["needs-decision", "ready"] {
            assert_eq!(phase(status, &hint(value)), Phase::Live(status));
        }
    }
}

#[test]
fn ready_only_refines_idle() {
    assert_eq!(phase(AgentStatus::Idle, &hint("ready")), Phase::Ready);
    for status in [AgentStatus::Done, AgentStatus::Unknown] {
        assert_eq!(phase(status, &hint("ready")), Phase::Live(status));
    }
}

#[test]
fn absent_and_unknown_hints_preserve_every_live_status() {
    for status in [
        AgentStatus::Idle,
        AgentStatus::Working,
        AgentStatus::Blocked,
        AgentStatus::Done,
        AgentStatus::Unknown,
    ] {
        assert_eq!(phase(status, &[]), Phase::Live(status));
        assert_eq!(phase(status, &hint("parked")), Phase::Live(status));
        assert_eq!(
            phase(status, &[("task".into(), "ready".into())]),
            Phase::Live(status)
        );
    }
}
