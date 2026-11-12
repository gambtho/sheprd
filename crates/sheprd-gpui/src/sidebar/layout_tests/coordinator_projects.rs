use super::*;
use crate::config::{CoordinatorGroupBy, LayoutMode};
use std::time::{Duration, Instant};

pub(crate) fn two_devices(window: &mut Window, cx: &mut Context<SheprdWindow>) -> SheprdWindow {
    let mut view = fixture_window(window, cx);
    view.config.layout.mode = LayoutMode::Coordinator;
    view.config.coordinator_group_by = CoordinatorGroupBy::Project;
    view.sidebar_width = Some(380.);
    let mut local = snapshot(3);
    for (workspace, label) in
        local
            .workspaces
            .iter_mut()
            .zip(["orchard-coord", "orchard-12", "unassigned"])
    {
        workspace.label = label.into();
        workspace.focused = false;
        workspace.agent_status = AgentStatus::Idle;
    }
    local.workspaces[1].focused = true;
    local.workspaces[1].agent_status = AgentStatus::Blocked;
    local.focused_workspace_id = Some("w1".into());
    let mut remote = local.clone();
    remote.workspaces.truncate(2);
    remote.workspaces[1].label = "orchard-7".into();
    remote.workspaces[1].focused = false;
    remote.workspaces[1].agent_status = AgentStatus::Working;
    remote.focused_workspace_id = None;
    let now = Instant::now();
    view.live
        .workspace_updates
        .insert("w0".into(), now - Duration::from_secs(60));
    view.live.snapshot = Some(Arc::new(local));
    view.endpoints[0].label = "Meadow".into();
    let mut endpoint = crate::endpoint::Endpoint::new(
        "ssh:pebble".into(),
        "Pebble".into(),
        ConnectTarget::Ssh {
            target: "unused".into(),
            session: "default".into(),
        },
        true,
    );
    endpoint.live.snapshot = Some(Arc::new(remote));
    endpoint.live.workspace_updates.insert("w0".into(), now);
    view.endpoints.push(endpoint);
    view
}

#[gpui::test]
fn project_order_merges_hosts_chooses_newest_coordinator_and_folds_once(
    cx: &mut gpui::TestAppContext,
) {
    let (_, cx) = cx.add_window_view(|window, cx| {
        SidebarFixture(cx.new(|cx| {
            let mut view = two_devices(window, cx);
            let rows = |view: &SheprdWindow| view.sidebar_workspaces();
            assert_eq!(
                rows(&view),
                [(1, "w0"), (1, "w1"), (0, "w1"), (0, "w2")].map(|(h, w)| (h, w.into()))
            );
            view.collapsed_projects.insert("\0orchard".into());
            assert_eq!(
                rows(&view),
                [(1, "w0"), (0, "w2")].map(|(h, w)| (h, w.into()))
            );
            view.collapsed_projects.clear();
            Arc::make_mut(view.live.snapshot.as_mut().unwrap()).workspaces[0].focused = true;
            assert_eq!(
                rows(&view)[0],
                (0, "w0".into()),
                "focused coordinator beats a newer report"
            );
            view.live.workspace_updates.clear();
            view.endpoints[1].live.workspace_updates.clear();
            Arc::make_mut(view.live.snapshot.as_mut().unwrap()).workspaces[0].focused = false;
            assert_eq!(
                rows(&view)[0],
                (0, "w0".into()),
                "device order breaks unknown-age ties"
            );
            view.config.coordinator_group_by = CoordinatorGroupBy::Device;
            assert_eq!(
                rows(&view),
                [(0, "w0"), (0, "w1"), (0, "w2"), (1, "w0"), (1, "w1")].map(|(h, w)| (h, w.into())),
                "Device remains the default"
            );
            view
        }))
    });
    cx.run_until_parked();
}

#[gpui::test]
fn project_rows_sum_attention_show_lane_devices_and_one_other_section(
    cx: &mut gpui::TestAppContext,
) {
    let (fixture, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| two_devices(window, cx));
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        SidebarFixture(view)
    });
    cx.simulate_resize(size(px(1000.), px(700.)));
    cx.run_until_parked();
    cx.update(|window, cx| {
        cx.set_global(TextPaintCounts::default());
        cx.default_global::<TextProbes>().0.clear();
        full_draw(window, cx).clear(cx);
        let labels = painted_labels(cx);
        for expected in [
            "orchard",
            "W7",
            "W12",
            "Meadow",
            "Pebble",
            "Other",
            "2 lanes · 1 needs you",
        ] {
            assert!(
                labels
                    .iter()
                    .any(|label| label.replace(['\u{2068}', '\u{2069}'], "") == expected),
                "missing {expected}: {labels:?}"
            );
        }
        // Count actual paints: optimized GPUI doesn't retain debug selectors.
        let counts = &cx.global::<TextPaintCounts>().0;
        assert_eq!(counts.get("orchard"), Some(&1));
        assert_eq!(counts.get("Other"), Some(&1));
    });
    fixture.update(cx, |fixture, cx| {
        fixture.0.update(cx, |view, cx| {
            view.sidebar_width = Some(180.);
            cx.notify();
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        cx.default_global::<TextProbes>().0.clear();
        full_draw(window, cx).clear(cx);
        let labels = painted_labels(cx);
        assert!(
            !labels
                .iter()
                .any(|label| label == "Meadow" || label == "Pebble")
        );
        assert!(labels.iter().any(|label| label == "W12"));
    });
}
