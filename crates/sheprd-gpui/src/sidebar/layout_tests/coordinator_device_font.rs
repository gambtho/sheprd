use super::*;

#[gpui::test]
fn device_labels_yield_to_lane_numbers_at_boundary_widths(cx: &mut gpui::TestAppContext) {
    let (fixture, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| coordinator_projects::two_devices(window, cx));
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        SidebarFixture(view)
    });
    cx.simulate_resize(size(px(1000.), px(700.)));
    for font_size in [12., 20.] {
        fixture.update(cx, |fixture, cx| {
            fixture.0.update(cx, |view, cx| {
                view.sidebar_width = Some(260.);
                view.config.sidebar.size = font_size;
                view.endpoints[0].label = "Long meadow device label".into();
                view.endpoints[1].label = "Long pebble device label".into();
                cx.notify();
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            painted_labels(cx);
            full_draw(window, cx).clear(cx);
            let labels = painted_labels(cx);
            assert!(
                labels.iter().any(|label| label == "W12"),
                "{font_size}: {labels:?}"
            );
            assert!(
                !labels.iter().any(|label| label.starts_with("Long ")),
                "{font_size}: {labels:?}"
            );
        });
    }
}
