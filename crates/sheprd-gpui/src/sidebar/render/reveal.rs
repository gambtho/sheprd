//! Keep a changed selection visible without overriding manual sidebar scrolling.
use crate::SheprdWindow;
use gpui::{Pixels, Window, px};

impl SheprdWindow {
    pub(super) fn reveal_sidebar_rows(
        &self,
        highlighted: [Option<usize>; 2],
        cover: Pixels,
        headed: bool,
        window: &mut Window,
    ) {
        // Follow the selection, but only once a frame has measured the viewport:
        // the handle resolves the request against the previous frame's bounds, so
        // an unmeasured list would scroll to a meaningless offset. Recording what
        // was revealed keeps later frames from undoing the user's own scrolling.
        for (list, row) in highlighted.iter().enumerate() {
            let Some(row) = *row else { continue };
            if self.sidebar_revealed[list].get() != Some(row)
                && self.sidebar_scroll[list].bounds().size.height > px(0.)
            {
                let scroll = &self.sidebar_scroll[list];
                let cover = if list == 0 { cover } else { px(0.) };
                let visible = scroll.bounds_for_item(row).is_some_and(|bounds| {
                    let offset = scroll.offset().y;
                    bounds.bottom() + offset > scroll.bounds().top() + cover
                        && bounds.top() + offset < scroll.bounds().bottom()
                });
                // Even a partially visible worktree is already seen. GPUI's
                // reveal also moves clipped rows, so only request it off-screen.
                if !visible {
                    scroll.scroll_to_item(row);
                    // The scroll lands in prepaint, after this render pinned a
                    // header for the old offset, and puts a row from above at
                    // the top edge, under the pinned header. Another frame pins
                    // for the new offset and moves the row out from under it.
                    if list == 0 && headed {
                        window.request_animation_frame();
                        self.sidebar_pin_reveal.set(Some(row));
                    }
                }
                self.sidebar_revealed[list].set(Some(row));
            }
        }
    }
}
