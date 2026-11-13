// session.rs - Session arithmetic; playtime is a SUM of immutable rows (spec 8.2).

use crate::model::{OpenSession, SessionClose};
use time::{Duration, OffsetDateTime};

// Minutes between two instants, never negative.
pub fn span_minutes(start: OffsetDateTime, end: OffsetDateTime) -> f64 {
    let secs = (end - start).as_seconds_f64();
    if secs.is_finite() && secs > 0.0 {
        secs / 60.0
    } else {
        0.0
    }
}

// Close an orphaned session at its last heartbeat, never inventing time.
pub fn recover_orphan_session(open: &OpenSession, now: OffsetDateTime) -> SessionClose {
    let mut end = open.last_heartbeat;
    if end < open.started_at {
        end = open.started_at;
    }
    if end > now {
        end = now.max(open.started_at);
    }
    SessionClose { id: open.id, ended_at: end, minutes: span_minutes(open.started_at, end) }
}

// What a heartbeat tick should do with an open session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatAction {
    // Normal tick - just advance last_heartbeat.
    Continue,
    // The machine slept: close at the last heartbeat and open a fresh session now.
    Split,
}

// A gap far longer than the interval means we were suspended.
pub fn heartbeat_action(last: OffsetDateTime, now: OffsetDateTime, interval: Duration) -> HeartbeatAction {
    let limit = interval * 3;
    if now - last > limit {
        HeartbeatAction::Split
    } else {
        HeartbeatAction::Continue
    }
}

// Sum of closed session spans.
pub fn total_minutes(sessions: &[(OffsetDateTime, OffsetDateTime)]) -> f64 {
    sessions.iter().map(|(s, e)| span_minutes(*s, *e)).sum()
}

// A session below the noise floor is deleted, not kept.
pub fn is_noise(minutes: f64, min_session_seconds: u64) -> bool {
    minutes * 60.0 < min_session_seconds as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GameId, SessionSource};
    use proptest::prelude::*;
    use time::macros::datetime;

    fn open(start: OffsetDateTime, hb: OffsetDateTime) -> OpenSession {
        OpenSession { id: 1, game_id: GameId::new(), started_at: start, last_heartbeat: hb, source: SessionSource::Wmi }
    }

    #[test]
    fn orphan_closes_at_heartbeat() {
        let s = datetime!(2026-01-01 10:00 UTC);
        let hb = datetime!(2026-01-01 11:30 UTC);
        let now = datetime!(2026-01-02 09:00 UTC);
        let c = recover_orphan_session(&open(s, hb), now);
        assert_eq!(c.ended_at, hb);
        assert!((c.minutes - 90.0).abs() < 1e-9);
    }

    #[test]
    fn orphan_with_bad_heartbeat_never_negative() {
        let s = datetime!(2026-01-01 10:00 UTC);
        let hb = datetime!(2026-01-01 09:00 UTC);
        let c = recover_orphan_session(&open(s, hb), datetime!(2026-01-01 12:00 UTC));
        assert_eq!(c.ended_at, s);
        assert!(c.minutes.abs() < 1e-9);
    }

    #[test]
    fn orphan_heartbeat_in_future_clamped_to_now() {
        let s = datetime!(2026-01-01 10:00 UTC);
        let hb = datetime!(2026-01-03 10:00 UTC);
        let now = datetime!(2026-01-01 11:00 UTC);
        let c = recover_orphan_session(&open(s, hb), now);
        assert_eq!(c.ended_at, now);
    }

    #[test]
    fn sleep_gap_splits() {
        let last = datetime!(2026-01-01 10:00 UTC);
        let iv = Duration::seconds(30);
        assert_eq!(heartbeat_action(last, last + Duration::seconds(30), iv), HeartbeatAction::Continue);
        assert_eq!(heartbeat_action(last, last + Duration::hours(3), iv), HeartbeatAction::Split);
    }

    #[test]
    fn noise_floor() {
        assert!(is_noise(0.5, 60));
        assert!(!is_noise(1.0, 60));
    }

    fn arb_sessions() -> impl Strategy<Value = Vec<(OffsetDateTime, OffsetDateTime)>> {
        prop::collection::vec((0i64..1_000_000, 0i64..100_000), 0..40).prop_map(|v| {
            let base = datetime!(2026-01-01 00:00 UTC);
            v.into_iter()
                .map(|(off, len)| (base + Duration::seconds(off), base + Duration::seconds(off + len)))
                .collect()
        })
    }

    proptest! {
        // Playtime is never negative.
        #[test]
        fn playtime_never_negative(sessions in arb_sessions()) {
            prop_assert!(total_minutes(&sessions) >= 0.0);
        }

        // Each session never exceeds its own wall-clock span.
        #[test]
        fn session_within_wall_clock(sessions in arb_sessions()) {
            for (s, e) in &sessions {
                let wall = (*e - *s).as_seconds_f64() / 60.0;
                prop_assert!(span_minutes(*s, *e) <= wall + 1e-9);
            }
        }

        // Playtime is invariant under session reordering.
        #[test]
        fn playtime_order_invariant(mut sessions in arb_sessions()) {
            let a = total_minutes(&sessions);
            sessions.reverse();
            let b = total_minutes(&sessions);
            prop_assert!((a - b).abs() < 1e-6);
        }

        // Recovery never yields negative time or time past now.
        #[test]
        fn recovery_bounded(start in 0i64..1_000_000, hb in -10_000i64..1_000_000, now in 0i64..2_000_000) {
            let base = datetime!(2026-01-01 00:00 UTC);
            let s = base + Duration::seconds(start);
            let o = open(s, s + Duration::seconds(hb));
            let n = base + Duration::seconds(now);
            let c = recover_orphan_session(&o, n);
            prop_assert!(c.minutes >= 0.0);
            prop_assert!(c.ended_at >= s);
            prop_assert!(c.ended_at <= n.max(s));
        }
    }
}
