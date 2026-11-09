#![allow(clippy::unwrap_used)]
use super::*;
use crate::config::LayoutMode;
use core::prelude::v1::test;
use std::sync::Arc;

#[gpui::test]
fn only_coordinator_layout_adds_a_bar_for_a_lane(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        let mut snapshot: sheprd_client::protocol::ClientShellSnapshot = serde_json::from_str(
            include_str!("../../../../sheprd-protocol/tests/fixtures/endpoint-snapshot-v1.json"),
        )
        .unwrap();
        snapshot.workspaces[0].label = "demo-12".into();
        view.live.snapshot = Some(Arc::new(snapshot));
        view
    });
    for mode in LayoutMode::ALL {
        let rendered = cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.config.layout.mode = mode;
                let slot = view.group_slots()[0];
                view.render_terminal_bar(slot, window, cx).is_some()
            })
        });
        assert_eq!(rendered, mode == LayoutMode::Coordinator, "{mode:?}");
    }
}

#[gpui::test]
fn missing_snapshots_have_no_bar(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        view.config.layout.mode = LayoutMode::Coordinator;
        view.live.snapshot = None;
        view
    });
    assert!(!cx.update(|window, cx| view.update(cx, |view, cx| {
        view.render_terminal_bar(view.group_slots()[0], window, cx)
            .is_some()
    })));
}
