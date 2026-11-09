//! Snapshot-only selection and display status for the terminal context row.
pub(super) use crate::sidebar::coordinator::context::Phase;
use crate::sidebar::coordinator::{context, groups::Listing};
use sheprd_client::protocol::{AgentStatus, ClientShellSnapshot};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Bar<'a> {
    Lane {
        number: &'a str,
        task: Option<&'a str>,
        status: Phase,
        label: &'a str,
        coordinator: Option<&'a str>,
    },
    Coordinator {
        project: &'a str,
        lanes: usize,
        needs: usize,
    },
}

pub(super) fn select<'a>(
    snapshot: &'a ClientShellSnapshot,
    listing: &Listing<'a>,
    focused: &str,
) -> Option<Bar<'a>> {
    let index = snapshot
        .workspaces
        .iter()
        .position(|workspace| workspace.workspace_id == focused)?;
    for (&project, members) in &listing.projects {
        let Some(member) = members.iter().find(|member| member.index == index) else {
            continue;
        };
        let workspace = &snapshot.workspaces[index];
        return Some(match member.lane {
            Some(number) => Bar::Lane {
                number,
                task: context::task(&workspace.tokens).filter(|task| !task.trim().is_empty()),
                status: context::phase(workspace.agent_status, &workspace.tokens),
                label: &workspace.label,
                coordinator: members
                    .iter()
                    .find(|member| member.lane.is_none())
                    .map(|member| snapshot.workspaces[member.index].workspace_id.as_str()),
            },
            None => {
                let (lanes, needs) = members.iter().fold((0, 0), |(lanes, needs), member| {
                    let workspace = &snapshot.workspaces[member.index];
                    let attention = matches!(
                        context::phase(workspace.agent_status, &workspace.tokens),
                        Phase::Live(AgentStatus::Blocked) | Phase::NeedsDecision
                    );
                    (
                        lanes + usize::from(member.lane.is_some()),
                        needs + usize::from(attention),
                    )
                });
                Bar::Coordinator {
                    project,
                    lanes,
                    needs,
                }
            }
        });
    }
    None
}

#[cfg(test)]
#[path = "model/tests.rs"]
mod tests;
