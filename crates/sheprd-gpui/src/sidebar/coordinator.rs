//! One workspace per lane, grouped by project within each host.
pub(crate) mod context;
pub(crate) mod freshness;
pub(crate) mod groups;
mod projects;

use context::Phase;

use super::{label_text, layout::SidebarLook, line_height, workspaces::label_of};
use crate::{NavigationTarget, SheprdWindow, direction::Logical, i18n::fl};
use gpui::{prelude::*, *};
use sheprd_client::protocol::{AgentStatus, ClientShellSnapshot};
use std::collections::HashSet;

pub(crate) fn host_listing(snapshot: &ClientShellSnapshot) -> groups::Listing<'_> {
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
            let coordinator = members
                .first()
                .filter(|member| member.lane.is_none())
                .map(|member| member.index);
            members
                .into_iter()
                .filter(move |member| {
                    Some(member.index) == coordinator || (!collapsed && member.lane.is_some())
                })
                .map(|member| member.index)
        })
        .chain(listing.unmatched)
        .collect()
}

fn status_color(theme: &crate::config::Theme, status: AgentStatus) -> u32 {
    theme.ink(match status {
        AgentStatus::Blocked => theme.palette[3],
        AgentStatus::Working => theme.palette[2],
        AgentStatus::Done => theme.palette[6],
        _ => theme.muted,
    })
}

impl SheprdWindow {
    pub(super) fn append_coordinator_workspaces(
        &self,
        mut list: Stateful<Div>,
        host_index: Option<usize>,
        look: SidebarLook,
        width: f32,
        cx: &mut Context<Self>,
    ) -> (Stateful<Div>, usize, Option<usize>) {
        if host_index
            .is_some_and(|index| self.endpoints[index].collapsed && self.endpoints.len() > 1)
        {
            return (list, 0, None);
        }
        let projects = self.coordinator_projects(host_index);
        let font = &self.config.sidebar;
        let mark_size = font.size;
        let theme = &self.theme;
        let amber = theme.ink(theme.palette[3]);
        let mut count = 0;
        let mut focused = None;
        for (&project, members) in &projects.listing.projects {
            let coordinator = if host_index.is_some() {
                members
                    .iter()
                    .find_map(|member| member.lane.is_none().then_some(member.index))
            } else {
                projects.coordinator(members, self.selected_endpoint)
            };
            let coordinator_workspace = coordinator.map(|index| projects.rows[index].2);
            let coordinator_focused = coordinator.is_some_and(|index| {
                let (host, _, workspace) = projects.rows[index];
                host == self.selected_endpoint && workspace.focused
            });
            let mark_color = coordinator_workspace.map_or(theme.muted, |workspace| {
                status_color(theme, workspace.agent_status)
            });
            let group_host = host_index.map_or("", |index| self.endpoints[index].id.as_str());
            let key = fold_key(group_host, project);
            let folded = self.collapsed_projects.contains(&key);
            let (lanes, needs) = projects.counts(members);
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
            let group_id = host_index.map_or_else(|| "all".to_owned(), |index| index.to_string());
            let mut heading = div()
                .id(SharedString::from(format!(
                    "project-{group_host}-{project}"
                )))
                .debug_selector(move || format!("coordinator-project-{group_id}-{project}"))
                .min_w_0()
                .flex_none()
                .mt(px(6.))
                .px(px(look.content_x()))
                .py(px(4.))
                .rounded(px(5.))
                .when(coordinator_focused, |row| row.bg(rgb(theme.active)))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .min_w_0()
                        .row()
                        .items_center()
                        .gap(px(8.))
                        .child(fold)
                        .child(div().flex_none().text_color(rgb(mark_color)).child(
                            if coordinator_workspace.is_some_and(|workspace| {
                                workspace.agent_status == AgentStatus::Working
                            }) {
                                crate::mark_layer::layered(mark_size, 1., move || {
                                    super::agents::spin(
                                        svg()
                                            .path("icons/coordinator-diamond.svg")
                                            .size(px(mark_size))
                                            .text_color(rgb(mark_color)),
                                        "coordinator-diamond",
                                        900,
                                    )
                                    .into_any_element()
                                })
                                .into_any_element()
                            } else {
                                label_text(if coordinator.is_some() { "◆" } else { "◇" })
                                    .into_any_element()
                            },
                        ))
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
            let received = coordinator.and_then(|index| {
                let (_, live, workspace) = projects.rows[index];
                live.workspace_updates.get(&workspace.workspace_id).copied()
            });
            if let Some(freshness::Age::Hours(hours)) =
                freshness::Age::since(received, std::time::Instant::now())
            {
                heading = heading.child(
                    div()
                        .ps(px(40.))
                        .truncate()
                        .text_color(rgb(theme.muted))
                        .child(label_text(&fl!("coordinator-stale", hours = hours))),
                );
            }
            if let Some(coordinator) = coordinator {
                let (index, _, workspace) = projects.rows[coordinator];
                let host = self.endpoints[index].id.clone();
                let workspace = workspace.workspace_id.clone();
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
            if coordinator_focused
                || (folded
                    && members.iter().any(|member| {
                        let (host, _, workspace) = projects.rows[member.index];
                        host == self.selected_endpoint && workspace.focused
                    }))
            {
                focused = Some(count);
            }
            list = list.child(heading);
            count += 1;
            if folded {
                continue;
            }
            for member in members {
                let Some(lane) = member.lane else { continue };
                let label = format!("W{lane}");
                let (endpoint_index, _, workspace) = projects.rows[member.index];
                if endpoint_index == self.selected_endpoint && workspace.focused {
                    focused = Some(count);
                }
                list = list.child(self.coordinator_lane(
                    endpoint_index,
                    (workspace, &label, true),
                    look,
                    width,
                    cx,
                ));
                count += 1;
            }
        }
        if host_index.is_none() && !projects.listing.unmatched.is_empty() {
            list = list.child(
                div()
                    .px(px(look.content_x()))
                    .child(label_text(&fl!("coordinator-other"))),
            );
            count += 1;
        }
        for &index in &projects.listing.unmatched {
            let (endpoint_index, _, workspace) = projects.rows[index];
            if endpoint_index == self.selected_endpoint && workspace.focused {
                focused = Some(count);
            }
            let label = label_of(
                &workspace.label,
                workspace.branch.as_deref(),
                workspace.custom_label,
                false,
            );
            list = list.child(self.coordinator_lane(
                endpoint_index,
                (workspace, label, false),
                look,
                width,
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
        width: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = &self.theme;
        let font = &self.config.sidebar;
        let status = workspace.agent_status;
        let phase = context::phase(status, &workspace.tokens);
        let blocked = matches!(
            phase,
            Phase::Live(AgentStatus::Blocked) | Phase::NeedsDecision
        );
        let color = status_color(
            theme,
            if blocked {
                AgentStatus::Blocked
            } else {
                status
            },
        );
        let text = match phase {
            Phase::NeedsDecision | Phase::Live(AgentStatus::Blocked) => {
                fl!("coordinator-needs-you")
            }
            Phase::Ready => fl!("coordinator-ready"),
            Phase::Live(status) => super::agents::status_text(status),
        };
        let task = context::task(&workspace.tokens)
            .map(|value| crate::notifications::safe_text(value, 160))
            .filter(|value| !value.is_empty());
        let host = self.endpoints[endpoint_index].id.clone();
        let id = workspace.workspace_id.clone();
        let target = id.clone();
        let selected = endpoint_index == self.selected_endpoint && workspace.focused;
        let name = crate::notifications::safe_text(label, 160);
        let device = crate::notifications::safe_text(&self.endpoints[endpoint_index].label, 80);
        let mut spoken_name = name.clone();
        if self.project_grouping() {
            spoken_name.push_str(&format!(", {device}"));
        }
        self.sidebar_button(
            div().id(SharedString::from(format!("coordinator-lane-{host}-{id}"))),
            task.as_ref().map_or_else(
                || format!("{spoken_name}: {text}"),
                |task| format!("{spoken_name}: {text}, {task}"),
            ),
        )
        .debug_selector(move || format!("coordinator-lane-{endpoint_index}-{id}"))
        .aria_selected(selected)
        .min_w_0()
        .flex_none()
        .h(px(
            line_height(font) * if task.is_some() { 2. } else { 1. } + 10.
        ))
        .mx(px(8.))
        .rounded(px(5.))
        .ps(px(if nested { 32. } else { look.content_x() }))
        .pe(px(10.))
        .row()
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
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .font_weight(FontWeight::MEDIUM)
                        .child(label_text(&name)),
                )
                .when_some(task, |column, task| {
                    column.child(
                        div()
                            .truncate()
                            .text_size(px(font.size * 0.85))
                            .text_color(rgb(theme.muted))
                            .child(label_text(&task)),
                    )
                }),
        )
        .when(
            self.project_grouping() && width >= super::scaled(font, 300.).max(300.),
            |row| {
                row.child(
                    div()
                        .max_w(px(90.))
                        .min_w_0()
                        .truncate()
                        .text_size(px(font.size * 0.85))
                        .text_color(rgb(theme.muted))
                        .child(label_text(&device)),
                )
            },
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
