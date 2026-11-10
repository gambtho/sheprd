use super::*;
use crate::config::LayoutMode;
use std::time::{Duration, Instant};

#[gpui::test]
fn coordinator_hides_only_hosts_without_workspace_rows_not_folded_nonempty_hosts(
    cx: &mut gpui::TestAppContext,
) {
    let (fixture, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| {
            let mut view = fixture_window(window, cx);
            let mut data = snapshot(1);
            data.workspaces[0].label = "demo-coord".into();
            view.live.snapshot = Some(Arc::new(data.clone()));
            for (id, name, empty) in [
                ("one", "Unavailable", None),
                ("two", "Vacant", Some(true)),
                ("three", "Folded", Some(false)),
            ] {
                let mut host = crate::endpoint::Endpoint::new(
                    format!("ssh:{id}"),
                    name.into(),
                    ConnectTarget::Ssh {
                        target: "unused".into(),
                        session: "default".into(),
                    },
                    true,
                );
                host.live.snapshot = empty.map(|empty| {
                    let mut s = data.clone();
                    if empty {
                        s.workspaces.clear();
                    }
                    Arc::new(s)
                });
                host.collapsed = true;
                view.endpoints.push(host);
            }
            view
        });
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        SidebarFixture(view)
    });
    cx.simulate_resize(size(px(900.), px(600.)));
    for mode in [
        LayoutMode::Coordinator,
        LayoutMode::from(crate::config::Density::Comfortable),
    ] {
        fixture.update(cx, |fixture, cx| {
            fixture.0.update(cx, |view, cx| {
                view.config.layout.mode = mode;
                cx.notify();
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            cx.default_global::<TextProbes>().0.clear();
            full_draw(window, cx).clear(cx);
            let text = &cx.global::<TextProbes>().0;
            assert!(
                text.contains_key("Folded"),
                "a folded host still owns workspace rows"
            );
            for name in ["Vacant", "Unavailable"] {
                assert_eq!(
                    text.contains_key(name),
                    mode != LayoutMode::Coordinator,
                    "{mode:?}: {name}"
                );
            }
        });
    }
}

#[gpui::test]
fn project_info_is_unknown_until_a_receive_time_and_old_info_adds_a_muted_line(
    cx: &mut gpui::TestAppContext,
) {
    let (fixture, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| {
            let mut view = fixture_window(window, cx);
            let mut data = snapshot(1);
            data.workspaces[0].label = "demo-coord".into();
            view.live.snapshot = Some(Arc::new(data));
            view.config.layout.mode = LayoutMode::Coordinator;
            view
        });
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        SidebarFixture(view)
    });
    for received in [
        None,
        Some(Instant::now() - Duration::from_secs(1200)),
        Some(Instant::now() - Duration::from_secs(7200)),
    ] {
        fixture.update(cx, |fixture, cx| {
            fixture.0.update(cx, |view, cx| {
                view.live.workspace_updates.clear();
                if let Some(at) = received {
                    let id = view.live.snapshot.as_ref().unwrap().workspaces[0]
                        .workspace_id
                        .clone();
                    view.live.workspace_updates.insert(id, at);
                }
                cx.notify();
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            cx.default_global::<TextProbes>().0.clear();
            full_draw(window, cx).clear(cx);
            let labels = painted_labels(cx);
            let stale: Vec<_> = labels
                .iter()
                .filter(|s| s.contains("project info from"))
                .collect();
            assert_eq!(
                stale.len(),
                usize::from(received.is_some_and(|at| at.elapsed().as_secs() >= 3600))
            );
            if let Some(text) = stale.first() {
                assert!(text.contains('2'));
            }
            assert!(!labels.iter().any(|s| s.contains("0 need you")));
        });
    }
}

#[test]
fn summary_keeps_attention_only_when_nonzero_and_age_messages_use_coarse_buckets() {
    let clean = |text: String| text.replace(['\u{2068}', '\u{2069}'], "");
    for (lanes, needs, want) in [
        (1, 0, "1 lane"),
        (2, 0, "2 lanes"),
        (2, 1, "2 lanes · 1 needs you"),
        (2, 3, "2 lanes · 3 need you"),
    ] {
        assert_eq!(
            clean(crate::i18n::english(
                "coordinator-summary",
                &[("lanes", lanes.into()), ("needs", needs.into())]
            )),
            want
        );
    }
    for (key, args, want) in [
        ("coordinator-updated-now", vec![], "updated just now"),
        (
            "coordinator-updated-minutes",
            vec![("minutes", 20.into())],
            "updated 20m ago",
        ),
        (
            "coordinator-updated-hours",
            vec![("hours", 2.into())],
            "updated 2h ago",
        ),
        (
            "coordinator-stale",
            vec![("hours", 2.into())],
            "◷ project info from 2 h ago",
        ),
    ] {
        assert_eq!(clean(crate::i18n::english(key, &args)), want);
    }
}
