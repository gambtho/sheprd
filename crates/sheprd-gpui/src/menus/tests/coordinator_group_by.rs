use super::*;
use crate::{actions::SetCoordinatorGroupBy, config::CoordinatorGroupBy};

#[test]
fn coordinator_group_by_menu_checks_one_choice_and_requires_coordinator_layout() {
    for mode in LayoutMode::ALL {
        for current in [CoordinatorGroupBy::Device, CoordinatorGroupBy::Project] {
            let item = layout_menu(mode, current);
            let MenuItem::Submenu(layout) = item else {
                panic!("Layout submenu")
            };
            let group_by = layout
                .items
                .iter()
                .find_map(|item| match item {
                    MenuItem::Submenu(menu) if menu.name.as_ref() == "Group by" => Some(menu),
                    _ => None,
                })
                .expect("Group by in the existing Layout menu");
            assert_eq!(group_by.disabled, mode != LayoutMode::Coordinator);
            let choices: Vec<_> = group_by
                .items
                .iter()
                .filter_map(|item| match item {
                    MenuItem::Action {
                        name,
                        action,
                        checked,
                        ..
                    } => Some((
                        name.as_ref(),
                        action
                            .as_any()
                            .downcast_ref::<SetCoordinatorGroupBy>()
                            .unwrap()
                            .group_by,
                        *checked,
                    )),
                    _ => None,
                })
                .collect();
            assert_eq!(
                choices,
                [
                    (
                        "Device",
                        CoordinatorGroupBy::Device,
                        current == CoordinatorGroupBy::Device
                    ),
                    (
                        "Project",
                        CoordinatorGroupBy::Project,
                        current == CoordinatorGroupBy::Project
                    ),
                ]
            );
        }
    }
}

#[test]
fn coordinator_group_by_reloads_refresh_menu_checks() {
    let old = Config::default();
    let mut new = old.clone();
    new.coordinator_group_by = CoordinatorGroupBy::Project;
    assert!(changed(&old, &new));
}
