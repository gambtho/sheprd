#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn working_coordinator_mark_is_available_to_the_asset_loader() {
    let path = "icons/coordinator-diamond.svg";
    assert!(Icons.load(path).unwrap().is_some());
    assert!(
        Icons
            .list("icons/")
            .unwrap()
            .iter()
            .any(|listed| listed == path)
    );
}
