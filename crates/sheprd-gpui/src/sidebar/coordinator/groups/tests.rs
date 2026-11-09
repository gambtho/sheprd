use super::*;

#[test]
fn workspace_identity_wins_then_agent_then_pane() {
    let rows = [
        (0, vec![" demo-12 ", "other-coord", "pane-1"]),
        (1, vec!["scratch", "demo-coord", "pane-2"]),
        (2, vec!["terminal", "pi", "demo-3"]),
    ];
    let got = listing(rows);
    assert_eq!(
        got.projects["demo"],
        vec![
            Member {
                index: 1,
                lane: None
            },
            Member {
                index: 2,
                lane: Some("3")
            },
            Member {
                index: 0,
                lane: Some("12")
            },
        ]
    );
    assert!(got.unmatched.is_empty());
}

#[test]
fn projects_without_coordinators_still_group_and_longest_prefix_wins() {
    let got = listing(
        ["demo-3", "demo-api-12", "demo-api-2", "demo-coord"]
            .into_iter()
            .enumerate()
            .map(|(i, name)| (i, [name])),
    );
    assert_eq!(
        got.projects.keys().copied().collect::<Vec<_>>(),
        ["demo", "demo-api"]
    );
    assert_eq!(
        got.projects["demo"]
            .iter()
            .map(|m| m.index)
            .collect::<Vec<_>>(),
        [3, 0]
    );
    assert_eq!(
        got.projects["demo-api"]
            .iter()
            .map(|m| m.index)
            .collect::<Vec<_>>(),
        [2, 1]
    );
}

#[test]
fn unmatched_names_keep_original_order_and_are_not_lanes() {
    let names = [
        ".config",
        "demo-local",
        "-3",
        "-coord",
        "demo-",
        "demo-3x",
        "demo-٣",
        "demo-2",
    ];
    let got = listing(names.into_iter().enumerate().map(|(i, name)| (i, [name])));
    assert_eq!(got.unmatched, [0, 1, 2, 3, 4, 5, 6]);
    assert_eq!(
        got.projects["demo"],
        [Member {
            index: 7,
            lane: Some("2")
        }]
    );
}

#[test]
fn numeric_order_handles_leading_zeroes_large_numbers_and_duplicate_coordinators() {
    let names = [
        "demo-12",
        "demo-003",
        "demo-coord",
        "demo-3",
        "demo-99999999999999999999999999999",
        "demo-coord",
        "demo-0",
    ];
    let got = listing(names.into_iter().enumerate().map(|(i, name)| (i, [name])));
    assert_eq!(
        got.projects["demo"]
            .iter()
            .map(|m| m.index)
            .collect::<Vec<_>>(),
        [2, 5, 6, 1, 3, 0, 4]
    );
}

#[test]
fn hosts_are_grouped_independently_and_each_workspace_occurs_once() {
    let left = listing([(0, ["demo-coord", "other-1"]), (1, ["demo-2", "other-2"])]);
    let right = listing([(0, ["demo-1", "pi"]), (1, ["scratch", "terminal"])]);
    assert_eq!(left.projects["demo"].len(), 2);
    assert_eq!(
        right.projects["demo"],
        [Member {
            index: 0,
            lane: Some("1")
        }]
    );
    assert_eq!(right.unmatched, [1]);
    assert!(!left.projects.contains_key("other"));
}
