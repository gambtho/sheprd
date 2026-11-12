#![allow(clippy::unwrap_used)]
use super::*;
use crate::{
    config::LayoutMode,
    sidebar::layout_tests::{fixture_window, full_draw},
    window::chrome_focus::Region,
};
use core::prelude::v1::test;
use std::sync::Arc;

mod coordinator_projects;
mod freshness;

#[gpui::test]
fn only_coordinator_layout_adds_a_bar_for_a_lane(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
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

// Isolate the real bar so unrelated title-bar controls cannot satisfy focus_next.
struct KeyboardBar(Entity<SheprdWindow>);

impl Render for KeyboardBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |view, cx| {
            div().track_focus(&view.focus).child(
                view.render_terminal_bar(view.group_slots()[0], window, cx)
                    .unwrap(),
            )
        })
    }
}

#[gpui::test]
fn return_is_a_keyboard_stop_only_in_the_title_bar_region(cx: &mut TestAppContext) {
    let (harness, cx) = cx.add_window_view(|window, cx| {
        KeyboardBar(cx.new(|cx| {
            let mut view = fixture_window(window, cx);
            let mut snapshot: sheprd_client::protocol::ClientShellSnapshot =
                serde_json::from_str(include_str!(
                    "../../../../sheprd-protocol/tests/fixtures/endpoint-snapshot-v1.json"
                ))
                .unwrap();
            snapshot.workspaces[0].label = "demo-12".into();
            let mut coordinator = snapshot.workspaces[0].clone();
            coordinator.workspace_id = "coordinator".into();
            coordinator.label = "demo-coord".into();
            snapshot.workspaces.push(coordinator);
            view.live.snapshot = Some(Arc::new(snapshot));
            view.config.layout.mode = LayoutMode::Coordinator;
            view
        }))
    });
    let view = harness.read_with(cx, |harness, _| harness.0.clone());
    cx.update(|window, _| window.activate_window());

    for key in ["enter", "space"] {
        // Terminal mode must not add a chrome tab stop.
        cx.update(|window, cx| {
            full_draw(window, cx).clear(cx);
            window.blur(cx);
            window.focus_next(cx);
            assert!(window.focused(cx).is_none());
        });
        view.update(cx, |view, _| view.chrome_region = Some(Region::TitleBar));
        cx.update(|window, cx| {
            full_draw(window, cx).clear(cx);
            window.focus_next(cx);
            assert!(
                window.focused(cx).is_some(),
                "the return button must be reachable in the title-bar region"
            );
            assert!(!view.read(cx).focus.is_focused(window));
            full_draw(window, cx).clear(cx);
        });
        let keystroke = crate::keymap::parse_keystroke(key).unwrap();
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
        cx.run_until_parked();
        cx.update(|window, cx| {
            full_draw(window, cx).clear(cx);
            let view = view.read(cx);
            assert!(
                view.focus.is_focused(window),
                "{key} returns terminal focus"
            );
            assert_eq!(view.chrome_region, None);
            assert_eq!(
                view.pending_navigation,
                Some(NavigationTarget::Workspace("coordinator".into())),
                "{key} activates the return button"
            );
        });
        view.update(cx, |view, _| view.pending_navigation = None);
    }
}

#[gpui::test]
fn missing_snapshots_have_no_bar(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.config.layout.mode = LayoutMode::Coordinator;
        view.live.snapshot = None;
        view
    });
    assert!(!cx.update(|window, cx| view.update(cx, |view, cx| {
        view.render_terminal_bar(view.group_slots()[0], window, cx)
            .is_some()
    })));
}
