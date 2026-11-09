//! One workspace per lane, grouped by project within each host.
mod groups;

use super::{label_text, layout::SidebarLook, line_height, workspaces::workspace_label};
use crate::{NavigationTarget, SheprdWindow, direction::Logical, i18n::fl};
use gpui::{prelude::*, *};
use sheprd_client::protocol::{AgentStatus, ClientShellSnapshot};
use std::collections::HashSet;

fn host_listing(snapshot: &ClientShellSnapshot) -> groups::Listing<'_> {
    groups::listing(
        snapshot
            .workspaces
            .iter()
            .enumerate()
            .map(|(index, workspace)| {
                let agents = snapshot
                    .agents
                    .iter()
                    .filter(|agent| agent.workspace_id == workspace.workspace_id);
                let names = std::iter::once(workspace.label.as_str())
                    .chain(agents.filter_map(|agent| agent.name.as_deref()))
                    .chain(
                        snapshot
                            .panes
                            .iter()
                            .filter(|pane| pane.workspace_id == workspace.workspace_id)
                            .filter_map(|pane| pane.label.as_deref()),
                    );
                (index, names)
            }),
    )
}

fn fold_key(host: &str, project: &str) -> String {
    format!("{host}\0{project}")
}

pub(super) fn workspace_order(
    snapshot: &ClientShellSnapshot,
    host: &str,
    folded: &HashSet<String>,
) -> Vec<usize> {
    let listing = host_listing(snapshot);
    listing
        .projects
        .into_iter()
        .flat_map(|(project, members)| {
            let collapsed = folded.contains(&fold_key(host, project));
            members
                .into_iter()
                .filter(move |_| !collapsed)
                .map(|member| member.index)
        })
        .chain(listing.unmatched)
        .collect()
}

impl SheprdWindow {
    pub(super) fn append_coordinator_workspaces(
        &self,
        mut list: Stateful<Div>,
        endpoint_index: usize,
        look: SidebarLook,
        cx: &mut Context<Self>,
    ) -> (Stateful<Div>, usize, Option<usize>) {
        let endpoint = &self.endpoints[endpoint_index];
        if endpoint.collapsed && self.endpoints.len() > 1 {
            return (list, 0, None);
        }
        let selected = endpoint_index == self.selected_endpoint;
        let live = if selected { &self.live } else { &endpoint.live };
        let Some(snapshot) = live.snapshot.as_deref() else {
            return (list, 0, None);
        };
        let listing = host_listing(snapshot);
        let font = &self.config.sidebar;
        let theme = &self.theme;
        let amber = theme.ink(theme.palette[3]);
        let mut count = 0;
        let mut focused = None;
        for (project, members) in listing.projects {
            let coordinator = members.iter().find(|member| member.lane.is_none());
            let key = fold_key(&endpoint.id, project);
            let folded = self.collapsed_projects.contains(&key);
            let needs = members
                .iter()
                .filter(|member| {
                    snapshot.workspaces[member.index].agent_status == AgentStatus::Blocked
                })
                .count();
            let lanes = members
                .iter()
                .filter(|member| member.lane.is_some())
                .count();
            let summary = fl!("coordinator-summary", lanes = lanes, needs = needs);
            let name = crate::notifications::safe_text(project, 160);
            let fold = self
                .sidebar_button(
                    div().id(SharedString::from(format!("project-fold-{key}"))),
                    name.clone(),
                )
                .aria_expanded(!folded)
                .flex_none()
                .cursor_pointer()
                .text_color(rgb(theme.muted))
                .child(label_text(if folded {
                    crate::direction::glyph("▸")
                } else {
                    "▾"
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    if !this.collapsed_projects.remove(&key) {
                        this.collapsed_projects.insert(key.clone());
                    }
                    cx.notify();
                }));
            let mut heading = div()
                .id(SharedString::from(format!(
                    "project-{}-{project}",
                    endpoint.id
                )))
                .debug_selector(move || format!("coordinator-project-{endpoint_index}-{project}"))
                .min_w_0()
                .flex_none()
                .mt(px(6.))
                .px(px(look.content_x()))
                .py(px(4.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(fold)
                        .child(
                            div()
                                .flex_none()
                                .text_color(rgb(theme.accent.unwrap_or(theme.foreground)))
                                .child(label_text("◆")),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .font_weight(FontWeight::BOLD)
                                .child(label_text(&name)),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(px(font.size * 0.85))
                                .text_color(rgb(if needs > 0 { amber } else { theme.muted }))
                                .child(label_text(&summary)),
                        ),
                );
            if let Some(coordinator) = coordinator {
                let host = endpoint.id.clone();
                let workspace = snapshot.workspaces[coordinator.index].workspace_id.clone();
                heading = self
                    .sidebar_button(heading, name)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.navigate_endpoint(&host, NavigationTarget::Workspace(&workspace), cx);
                    }));
            } else {
                heading = heading.child(
                    div()
                        .ps(px(40.))
                        .text_size(px(font.size * 0.85))
                        .text_color(rgb(theme.muted))
                        .child(fl!("coordinator-missing")),
                );
            }
            if selected
                && folded
                && members
                    .iter()
                    .any(|member| snapshot.workspaces[member.index].focused)
            {
                focused = Some(count);
            }
            list = list.child(heading);
            count += 1;
            if folded {
                continue;
            }
            for member in members {
                let label = member
                    .lane
                    .map_or_else(|| fl!("coordinator-row"), |lane| format!("W{lane}"));
                let workspace = &snapshot.workspaces[member.index];
                if selected && workspace.focused {
                    focused = Some(count);
                }
                list = list.child(self.coordinator_lane(
                    endpoint_index,
                    (workspace, &label, true),
                    look,
                    cx,
                ));
                count += 1;
            }
        }
        for index in listing.unmatched {
            let workspace = &snapshot.workspaces[index];
            if selected && workspace.focused {
                focused = Some(count);
            }
            list = list.child(self.coordinator_lane(
                endpoint_index,
                (workspace, workspace_label(workspace, false), false),
                look,
                cx,
            ));
            count += 1;
        }
        (list, count, focused)
    }

    fn coordinator_lane(
        &self,
        endpoint_index: usize,
        (workspace, label, nested): (&sheprd_client::protocol::ClientShellWorkspace, &str, bool),
        look: SidebarLook,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = &self.theme;
        let font = &self.config.sidebar;
        let status = workspace.agent_status;
        let blocked = status == AgentStatus::Blocked;
        let color = theme.ink(match status {
            AgentStatus::Blocked => theme.palette[3],
            AgentStatus::Working => theme.palette[2],
            AgentStatus::Done => theme.palette[6],
            _ => theme.muted,
        });
        let text = if blocked {
            fl!("coordinator-needs-you")
        } else {
            super::agents::status_text(status)
        };
        let host = self.endpoints[endpoint_index].id.clone();
        let id = workspace.workspace_id.clone();
        let target = id.clone();
        let selected = endpoint_index == self.selected_endpoint && workspace.focused;
        let name = crate::notifications::safe_text(label, 160);
        self.sidebar_button(
            div().id(SharedString::from(format!("coordinator-lane-{host}-{id}"))),
            format!("{name}: {text}"),
        )
        .debug_selector(move || format!("coordinator-lane-{endpoint_index}-{id}"))
        .aria_selected(selected)
        .min_w_0()
        .flex_none()
        .h(px(line_height(font) + 10.))
        .mx(px(8.))
        .rounded(px(5.))
        .ps(px(if nested { 32. } else { look.content_x() }))
        .pe(px(10.))
        .flex()
        .items_center()
        .gap(px(9.))
        .cursor_pointer()
        .when(selected, |row| row.bg(rgb(theme.active)))
        .hover(|style| style.bg(rgb(theme.active)))
        .child(div().flex_none().text_color(rgb(color)).child(label_text(
            if status == AgentStatus::Working || blocked {
                "●"
            } else {
                "○"
            },
        )))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::MEDIUM)
                .child(label_text(&name)),
        )
        .child(
            div()
                .flex_none()
                .text_size(px(font.size * 0.9))
                .text_color(rgb(if blocked { color } else { theme.muted }))
                .child(label_text(&text)),
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            this.navigate_endpoint(&host, NavigationTarget::Workspace(&target), cx);
        }))
    }
}
