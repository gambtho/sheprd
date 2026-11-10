//! Receive-time buckets shared by Coordinator's project row and terminal bar.
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Age {
    JustNow,
    Minutes(u64),
    Hours(u64),
}

impl Age {
    pub(crate) fn since(received: Option<Instant>, now: Instant) -> Option<Self> {
        Some(match now.saturating_duration_since(received?).as_secs() {
            0..60 => Self::JustNow,
            seconds @ 60..3600 => Self::Minutes(seconds / 60),
            seconds => Self::Hours(seconds / 3600),
        })
    }
}

#[cfg(test)]
#[path = "freshness/tests.rs"]
mod tests;
