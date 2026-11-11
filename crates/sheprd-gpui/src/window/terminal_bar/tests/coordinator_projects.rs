use super::*;
use crate::sidebar::layout_tests::{coordinator_projects::two_devices, painted_labels};

#[gpui::test]
fn coordinator_projects_bar_sums_both_devices(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(|window, cx| {
        KeyboardBar(cx.new(|cx| {
            let mut view = two_devices(window, cx);
            let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
            snapshot.focused_workspace_id = Some("w0".into());
            snapshot.workspaces[0].focused = true;
            snapshot.workspaces[1].focused = false;
            view.chrome_region = Some(Region::TitleBar);
            view
        }))
    });
    cx.update(|window, cx| {
        window.activate_window();
        painted_labels(cx);
        full_draw(window, cx).clear(cx);
        let labels = painted_labels(cx);
        assert!(
            labels.iter().any(|label| {
                label.replace(['\u{2068}', '\u{2069}'], "") == "2 lanes · 1 needs you"
            }),
            "{labels:?}"
        );
        window.blur(cx);
        window.focus_next(cx);
        assert!(
            window.focused(cx).is_none(),
            "coordinator bar has no return stop"
        );
    });
}

#[gpui::test]
fn coordinator_projects_return_switches_device_despite_duplicate_workspace_ids(
    cx: &mut TestAppContext,
) {
    let (harness, cx) = cx.add_window_view(|window, cx| {
        KeyboardBar(cx.new(|cx| {
            let mut view = two_devices(window, cx);
            // Keep the destination renderable after the real endpoint switch.
            Arc::make_mut(view.endpoints[1].live.snapshot.as_mut().unwrap()).focused_workspace_id =
                Some("w0".into());
            view.chrome_region = Some(Region::TitleBar);
            view
        }))
    });
    let view = harness.read_with(cx, |harness, _| harness.0.clone());
    cx.update(|window, cx| {
        window.activate_window();
        full_draw(window, cx).clear(cx);
        window.blur(cx);
        window.focus_next(cx);
        assert!(window.focused(cx).is_some());
    });
    let keystroke = crate::keymap::parse_keystroke("enter").unwrap();
    cx.simulate_event(KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(KeyUpEvent { keystroke });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.selected_endpoint, 1,
            "return must select the newest coordinator's device"
        );
        assert_eq!(
            view.pending_navigation,
            Some(NavigationTarget::Workspace("w0".into()))
        );
    });
}
