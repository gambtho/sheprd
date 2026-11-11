use super::*;
use std::time::Instant;

#[test]
fn snapshots_do_not_invent_or_refresh_metadata_receive_times() {
    let mut live = LiveState::default();
    let first = snapshot();
    let id = first.workspaces[0].workspace_id.clone();
    live.apply(ClientEvent::Snapshot(first.clone()));
    assert!(
        live.workspace_updates.is_empty(),
        "initial tokens do not provide an age"
    );
    let received = Instant::now();
    live.workspace_updates.insert(id.clone(), received);
    let mut next = first.clone();
    Arc::make_mut(&mut next).revision += 1;
    Arc::make_mut(&mut next).workspaces[0]
        .tokens
        .push(("task".into(), "Build the demo".into()));
    live.apply(ClientEvent::Snapshot(next));
    assert_eq!(live.workspace_updates.get(&id), Some(&received));
    let mut changed = live.clone();
    changed
        .workspace_updates
        .insert(id, received + std::time::Duration::from_secs(1));
    assert!(
        !live.only_surface_changed(&changed),
        "metadata invalidates cached chrome"
    );
}

#[test]
fn closed_workspaces_new_boots_and_disconnects_retire_old_receive_times() {
    let mut live = LiveState::default();
    let first = snapshot();
    let id = first.workspaces[0].workspace_id.clone();
    live.apply(ClientEvent::Snapshot(first.clone()));
    live.workspace_updates.insert(id.clone(), Instant::now());
    let mut closed = first.clone();
    Arc::make_mut(&mut closed).workspaces.clear();
    live.apply(ClientEvent::Snapshot(closed));
    assert!(!live.workspace_updates.contains_key(&id));
    live.workspace_updates.insert(id.clone(), Instant::now());
    let mut reboot = first;
    Arc::make_mut(&mut reboot).boot_id = "new-demo-boot".into();
    live.apply(ClientEvent::Snapshot(reboot));
    assert!(live.workspace_updates.is_empty());
    live.workspace_updates.insert(id, Instant::now());
    live.apply(ClientEvent::Disconnected {
        reason: "demo closed".into(),
        ssh: None,
    });
    assert!(live.workspace_updates.is_empty());
    assert!(
        LiveState::default().workspace_updates.is_empty(),
        "restart starts unknown"
    );
}
