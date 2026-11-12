//! A shared row index for project rendering, navigation and coordinator selection.
use super::{context, fold_key, groups, host_listing};
use crate::{SheprdWindow, state::LiveState};
use sheprd_client::protocol::{AgentStatus, ClientShellWorkspace};

#[derive(Default)]
pub(crate) struct Projects<'a> {
    pub(crate) rows: Vec<(usize, &'a LiveState, &'a ClientShellWorkspace)>,
    pub(crate) listing: groups::Listing<'a>,
}

impl Projects<'_> {
    pub(crate) fn coordinator(
        &self,
        members: &[groups::Member<'_>],
        selected: usize,
    ) -> Option<usize> {
        members
            .iter()
            .filter(|m| m.lane.is_none())
            .max_by_key(|m| {
                let (host, live, workspace) = self.rows[m.index];
                (
                    host == selected && workspace.focused,
                    live.workspace_updates.get(&workspace.workspace_id),
                    std::cmp::Reverse(m.index),
                )
            })
            .map(|m| m.index)
    }

    pub(crate) fn counts(&self, members: &[groups::Member<'_>]) -> (usize, usize) {
        members.iter().fold((0, 0), |(lanes, needs), member| {
            let workspace = self.rows[member.index].2;
            let attention = matches!(
                context::phase(workspace.agent_status, &workspace.tokens),
                context::Phase::Live(AgentStatus::Blocked) | context::Phase::NeedsDecision
            );
            (
                lanes + usize::from(member.lane.is_some()),
                needs + usize::from(attention),
            )
        })
    }

    pub(crate) fn order(
        &self,
        selected: usize,
        folded: &std::collections::HashSet<String>,
    ) -> Vec<(usize, String)> {
        self.listing
            .projects
            .iter()
            .flat_map(|(name, members)| {
                let coordinator = self.coordinator(members, selected);
                coordinator.into_iter().chain(
                    members
                        .iter()
                        .filter(move |m| m.lane.is_some() && !folded.contains(&fold_key("", name)))
                        .map(|m| m.index),
                )
            })
            .chain(self.listing.unmatched.iter().copied())
            .map(|index| {
                let (host, _, workspace) = self.rows[index];
                (host, workspace.workspace_id.clone())
            })
            .collect()
    }
}

impl SheprdWindow {
    pub(crate) fn project_grouping(&self) -> bool {
        self.config.layout.mode == crate::config::LayoutMode::Coordinator
            && self.config.coordinator_group_by == crate::config::CoordinatorGroupBy::Project
    }

    pub(crate) fn coordinator_projects(&self, host: Option<usize>) -> Projects<'_> {
        let mut result = Projects::default();
        for (index, live) in self
            .listed_lives()
            .filter(|(index, _)| host.is_none_or(|h| h == *index))
        {
            let Some(snapshot) = live.snapshot.as_deref() else {
                continue;
            };
            let offset = result.rows.len();
            result
                .rows
                .extend(snapshot.workspaces.iter().map(|w| (index, live, w)));
            let listing = host_listing(snapshot);
            for (project, mut members) in listing.projects {
                for member in &mut members {
                    member.index += offset;
                }
                result
                    .listing
                    .projects
                    .entry(project)
                    .or_default()
                    .extend(members);
            }
            result
                .listing
                .unmatched
                .extend(listing.unmatched.into_iter().map(|i| i + offset));
        }
        result.listing.sort();
        result
    }
}
