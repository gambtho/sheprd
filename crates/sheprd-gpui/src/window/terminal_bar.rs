//! One-line workspace context above terminals in the Coordinator layout.
mod model;

use super::SheprdWindow;
use crate::sidebar::{coordinator::freshness::Age, label_text};
use crate::{
    NavigationTarget,
    browser::{Shown, Slot},
    config::LayoutMode,
    direction::Logical,
    fonts::StyledFont,
    i18n::fl,
    notifications::safe_text,
    sidebar::{coordinator::host_listing, status_text},
};
use gpui::{prelude::*, *};
use model::{Bar, Phase};
use sheprd_client::protocol::AgentStatus;

impl SheprdWindow {
    pub(super) fn render_terminal_bar(
        &self,
        slot: Slot,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        if self.config.layout.mode != LayoutMode::Coordinator
            || !matches!(self.group_shown(slot.id, cx), Shown::Terminal)
        {
            return None;
        }
        let snapshot = self.live.snapshot.as_deref()?;
        let workspace = snapshot.focused_workspace_id.as_deref()?;
        let listing = host_listing(snapshot);
        let theme = &self.theme;
        let amber = theme.ink(theme.palette[3]);
        let (badge, title, status, label, coordinator) =
            match model::select(snapshot, &listing, workspace)? {
                Bar::Coordinator {
                    project,
                    lanes,
                    needs,
                } => (
                    fl!("terminal-bar-coordinator"),
                    Some(safe_text(project, 160)),
                    (
                        fl!("coordinator-summary", lanes = lanes, needs = needs),
                        if needs > 0 { amber } else { theme.muted },
                    ),
                    None,
                    None,
                ),
                Bar::Lane {
                    number,
                    task,
                    status,
                    label,
                    coordinator,
                } => {
                    let color = match status {
                        Phase::NeedsDecision | Phase::Live(AgentStatus::Blocked) => amber,
                        Phase::Ready | Phase::Live(AgentStatus::Working) => {
                            theme.ink(theme.palette[2])
                        }
                        Phase::Live(AgentStatus::Done) => theme.ink(theme.palette[6]),
                        _ => theme.muted,
                    };
                    let word = match status {
                        Phase::NeedsDecision => fl!("coordinator-needs-you"),
                        Phase::Ready => fl!("coordinator-ready"),
                        Phase::Live(status) => status_text(status),
                    };
                    (
                        format!("W{}", safe_text(number, 8)),
                        task.map(|task| safe_text(task, 240)),
                        (word, color),
                        Some(safe_text(label, 160)),
                        coordinator,
                    )
                }
            };
        let viewport = f32::from(window.viewport_size().width);
        let width = (viewport - self.sidebar_column_width(viewport).unwrap_or(0.))
            * self.group_share(slot.id);
        let age = label
            .as_ref()
            .and_then(|_| {
                Age::since(
                    self.live.workspace_updates.get(workspace).copied(),
                    std::time::Instant::now(),
                )
            })
            .map(|age| match age {
                Age::JustNow => fl!("coordinator-updated-now"),
                Age::Minutes(n) => fl!("coordinator-updated-minutes", minutes = n),
                Age::Hours(n) => fl!("coordinator-updated-hours", hours = n),
            });
        let font = &self.config.sidebar;
        let badge = div()
            .child(format!("[{badge}]"))
            .flex_none()
            .px(px(6.))
            .bg(rgb(theme.active));
        let title = div()
            .child(title.unwrap_or_default())
            .flex_1()
            .min_w_0()
            .truncate();
        let status = div().child(status.0).flex_none().text_color(rgb(status.1));
        let label = label.filter(|_| width >= 600.).map(|label| {
            div()
                .child(label)
                .max_w(px(180.))
                .min_w_0()
                .truncate()
                .text_color(rgb(theme.muted))
        });
        let coordinator = coordinator.map(|coordinator| {
            let host = self.endpoints[self.selected_endpoint].id.clone();
            let target = coordinator.to_owned();
            div()
                .child(fl!("terminal-bar-return"))
                .id(SharedString::from(slot.selector("terminal-coordinator")))
                .role(Role::Button)
                .aria_label(fl!("terminal-bar-return"))
                .flex_none()
                .px(px(8.))
                .py(px(2.))
                .border_1()
                .border_color(rgb(theme.active))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(theme.active)))
                .map(|control| self.chrome_stop(super::chrome_focus::Region::TitleBar, control))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    window.focus(&this.focus, cx);
                    this.navigate_endpoint(&host, NavigationTarget::Workspace(&target), cx);
                }))
        });
        Some(
            div()
                .debug_selector(move || slot.selector("terminal-bar"))
                .row()
                .items_center()
                .gap(px(10.))
                .px(px(12.))
                .flex_none()
                .h(px((font.size * 1.5 + 10.).max(32.)))
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_font(font)
                .text_size(px(font.size))
                .text_color(rgb(theme.foreground))
                .bg(self.fill().over(theme.surface, theme.surface))
                .border_b_1()
                .border_color(rgb(theme.active))
                .child(badge)
                .child(title)
                .child(status)
                .children(age.map(|text| {
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(rgb(theme.muted))
                        .child(label_text(&text))
                }))
                .children(label)
                .children(coordinator),
        )
    }
}

#[cfg(test)]
mod tests;
