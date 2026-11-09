#![allow(clippy::unwrap_used)]
use super::*;
use crate::sidebar::coordinator::groups;

fn snapshot(names: &[&str]) -> ClientShellSnapshot {
    let mut snapshot: ClientShellSnapshot = serde_json::from_str(include_str!(
        "../../../../../sheprd-protocol/tests/fixtures/endpoint-snapshot-v1.json"
    ))
    .unwrap();
    let template = snapshot.workspaces[0].clone();
    snapshot.workspaces = names
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let mut workspace = template.clone();
            workspace.workspace_id = format!("w{i}");
            workspace.label = (*label).into();
            workspace.agent_status = AgentStatus::Idle;
            workspace.tokens.clear();
            workspace
        })
        .collect();
    snapshot
}

fn bar(snapshot: &ClientShellSnapshot, focused: usize) -> Option<Bar<'_>> {
    let listing = groups::listing(
        snapshot
            .workspaces
            .iter()
            .enumerate()
            .map(|(i, workspace)| (i, [workspace.label.as_str()])),
    );
    select(snapshot, &listing, &format!("w{focused}"))
}

#[test]
fn lane_keeps_number_task_and_same_host_coordinator() {
    let mut snapshot = snapshot(&["demo-api-003", "demo-coord", "demo-api-coord"]);
    snapshot.workspaces[0].tokens = vec![
        ("task".into(), "Check schema".into()),
        ("phase".into(), "needs-decision".into()),
    ];
    assert_eq!(
        bar(&snapshot, 0),
        Some(Bar::Lane {
            number: "003",
            task: Some("Check schema"),
            status: Phase::NeedsDecision,
            label: "demo-api-003",
            coordinator: Some("w2"),
        })
    );
}

#[test]
fn missing_coordinator_and_task_leave_only_lane_context() {
    let snapshot = snapshot(&["solo-8", "scratch"]);
    assert_eq!(
        bar(&snapshot, 0),
        Some(Bar::Lane {
            number: "8",
            task: None,
            status: Phase::Live(AgentStatus::Idle),
            label: "solo-8",
            coordinator: None,
        })
    );
    assert_eq!(bar(&snapshot, 1), None);
    let listing = groups::listing([(0, ["solo-8"])]);
    assert_eq!(select(&snapshot, &listing, "gone"), None);
}

#[test]
fn coordinator_counts_project_members_with_refined_attention() {
    let mut snapshot = snapshot(&["demo-12", "demo-coord", "demo-3", "other-2", "demo-coord"]);
    snapshot.workspaces[0].agent_status = AgentStatus::Blocked;
    snapshot.workspaces[1].tokens = vec![("phase".into(), "needs-decision".into())];
    snapshot.workspaces[2].tokens = vec![("phase".into(), "needs-decision".into())];
    snapshot.workspaces[3].agent_status = AgentStatus::Blocked;
    assert_eq!(
        bar(&snapshot, 1),
        Some(Bar::Coordinator {
            project: "demo",
            lanes: 2,
            needs: 3,
        })
    );
    let Some(Bar::Lane { coordinator, .. }) = bar(&snapshot, 0) else {
        panic!("no lane")
    };
    assert_eq!(
        coordinator,
        Some("w1"),
        "first coordinator wins stable ties"
    );
}

#[test]
fn live_working_and_blocked_always_override_phase_hints() {
    for status in [AgentStatus::Working, AgentStatus::Blocked] {
        for hint in [None, Some("needs-decision"), Some("ready"), Some("other")] {
            assert_eq!(phase(status, hint), Phase::Live(status));
        }
    }
}

#[test]
fn needs_decision_refines_inactive_status_but_ready_only_refines_idle() {
    for status in [AgentStatus::Idle, AgentStatus::Done, AgentStatus::Unknown] {
        assert_eq!(phase(status, Some("needs-decision")), Phase::NeedsDecision);
        assert_eq!(
            phase(status, Some("ready")),
            if status == AgentStatus::Idle {
                Phase::Ready
            } else {
                Phase::Live(status)
            }
        );
        for hint in [None, Some("other"), Some("Ready")] {
            assert_eq!(phase(status, hint), Phase::Live(status));
        }
    }
}

#[test]
fn last_token_wins_and_blank_task_is_omitted() {
    let mut snapshot = snapshot(&["demo-2"]);
    snapshot.workspaces[0].tokens = vec![
        ("task".into(), "Old".into()),
        ("task".into(), "  ".into()),
        ("phase".into(), "needs-decision".into()),
        ("phase".into(), "ready".into()),
    ];
    assert_eq!(
        bar(&snapshot, 0),
        Some(Bar::Lane {
            number: "2",
            task: None,
            status: Phase::Ready,
            label: "demo-2",
            coordinator: None,
        })
    );
}
