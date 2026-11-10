use super::*;
use crate::sidebar::layout_tests::painted_labels;
use std::time::{Duration, Instant};

#[gpui::test]
fn a_lanes_bar_shows_only_known_receive_time_ages(cx: &mut TestAppContext) {
    let (harness, cx) = cx.add_window_view(|window, cx| {
        KeyboardBar(cx.new(|cx| {
            let mut view = fixture_window(window, cx);
            let mut snapshot: sheprd_client::protocol::ClientShellSnapshot =
                serde_json::from_str(include_str!(
                    "../../../../../sheprd-protocol/tests/fixtures/endpoint-snapshot-v1.json"
                ))
                .unwrap();
            snapshot.workspaces[0].label = "demo-7".into();
            view.live.snapshot = Some(Arc::new(snapshot));
            view.config.layout.mode = LayoutMode::Coordinator;
            view
        }))
    });
    let view = harness.read_with(cx, |h, _| h.0.clone());
    for (seconds, want) in [
        (None, None),
        (Some(30), Some("updated just now")),
        (Some(1200), Some("updated 20m ago")),
        (Some(7200), Some("updated 2h ago")),
    ] {
        view.update(cx, |v, cx| {
            v.live.workspace_updates.clear();
            if let Some(seconds) = seconds {
                let id = v
                    .live
                    .snapshot
                    .as_ref()
                    .unwrap()
                    .focused_workspace_id
                    .clone()
                    .unwrap();
                v.live
                    .workspace_updates
                    .insert(id, Instant::now() - Duration::from_secs(seconds));
            }
            cx.notify();
        });
        cx.update(|window, cx| {
            painted_labels(cx);
            window.refresh();
            full_draw(window, cx).clear(cx);
            let labels = painted_labels(cx);
            let ages: Vec<_> = labels
                .into_iter()
                .filter(|s| s.starts_with("updated "))
                .map(|s| s.replace(['\u{2068}', '\u{2069}'], ""))
                .collect();
            assert_eq!(
                ages,
                want.into_iter().map(str::to_owned).collect::<Vec<_>>()
            );
        });
    }
}
