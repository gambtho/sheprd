use super::*;
use crate::config::CoordinatorGroupBy;

#[test]
fn coordinator_group_by_saves_only_the_last_choice_even_when_it_matches_the_loaded_value() {
    let saves = LayoutSaves::default();
    saves.coordinator.want(CoordinatorGroupBy::Project);
    saves.coordinator.want(CoordinatorGroupBy::Device);
    let mut written = Vec::new();
    for _ in 0..2 {
        saves
            .coordinator
            .write_latest(|choice| {
                written.push(choice);
                Ok(())
            })
            .unwrap();
    }
    assert_eq!(written, [CoordinatorGroupBy::Device]);
}

#[test]
fn coordinator_group_by_fences_stale_loads_but_accepts_files_after_the_save_finishes() {
    let saves = LayoutSaves::default();
    let stale = saves.stamp();
    saves.coordinator.want(CoordinatorGroupBy::Project);
    saves.coordinator.write_latest(|_| Ok(())).unwrap();
    let mut loaded = Config::default();
    saves.keep_newer(stale, &mut loaded);
    assert_eq!(loaded.coordinator_group_by, CoordinatorGroupBy::Project);

    let current = saves.stamp();
    let mut loaded = Config::default();
    saves.keep_newer(current, &mut loaded);
    assert_eq!(loaded.coordinator_group_by, CoordinatorGroupBy::Device);
}
