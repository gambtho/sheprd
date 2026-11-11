use super::*;
use std::time::Duration;

#[test]
fn no_receive_time_means_unknown_including_after_restart() {
    assert_eq!(Age::since(None, Instant::now()), None);
}

#[test]
fn buckets_use_elapsed_receive_time_not_snapshot_activity() {
    let received = Instant::now();
    for (seconds, want) in [
        (0, Age::JustNow),
        (59, Age::JustNow),
        (60, Age::Minutes(1)),
        (1_200, Age::Minutes(20)),
        (3_599, Age::Minutes(59)),
        (3_600, Age::Hours(1)),
        (7_200, Age::Hours(2)),
    ] {
        assert_eq!(
            Age::since(Some(received), received + Duration::from_secs(seconds)),
            Some(want)
        );
    }
}
