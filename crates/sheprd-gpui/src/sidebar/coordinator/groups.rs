//! Workspace identities and deterministic project ordering, independent of UI state.
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Member<'a> {
    pub(crate) index: usize,
    pub(crate) lane: Option<&'a str>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Listing<'a> {
    pub(crate) projects: BTreeMap<&'a str, Vec<Member<'a>>>,
    pub(super) unmatched: Vec<usize>,
}

pub(crate) fn listing<'a, N: IntoIterator<Item = &'a str>>(
    rows: impl IntoIterator<Item = (usize, N)>,
) -> Listing<'a> {
    let mut result = Listing::default();
    for (index, names) in rows {
        let identity = names.into_iter().find_map(|name| {
            let (project, suffix) = name.trim().rsplit_once('-')?;
            if project.is_empty() {
                return None;
            }
            match suffix {
                "coord" => Some((project, None)),
                digits if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {
                    Some((project, Some(digits)))
                }
                _ => None,
            }
        });
        match identity {
            Some((project, lane)) => result
                .projects
                .entry(project)
                .or_default()
                .push(Member { index, lane }),
            None => result.unmatched.push(index),
        }
    }
    result.sort();
    result
}

impl Listing<'_> {
    pub(super) fn sort(&mut self) {
        for members in self.projects.values_mut() {
            // Decimal strings avoid overflow, and stable sorting preserves device-order ties.
            members.sort_by_key(|member| {
                member.lane.map(|lane| {
                    let digits = lane.trim_start_matches('0');
                    (digits.len(), digits)
                })
            });
        }
    }
}

#[cfg(test)]
#[path = "groups/tests.rs"]
mod tests;
