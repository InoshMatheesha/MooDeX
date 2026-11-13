// stats.rs - Dashboard aggregation as a pure fold.

use crate::model::{DashboardStats, Game, PlayStats, Status};
use crate::playtime::format_playtime;
use std::cmp::Ordering;

// Aggregate the whole library for the dashboard.
pub fn dashboard_stats(games: &[(Game, PlayStats)]) -> DashboardStats {
    let mut s = DashboardStats { total: games.len(), ..DashboardStats::default() };
    let mut rating_sum = 0u64;
    let mut rated = 0u64;
    for (g, p) in games {
        match g.status {
            Status::Playing => s.playing += 1,
            Status::Completed => s.completed += 1,
            Status::Backlog => s.backlog += 1,
            Status::Dropped => s.dropped += 1,
        }
        if g.install.is_some() {
            s.installed += 1;
        }
        if p.minutes.is_finite() && p.minutes > 0.0 {
            s.total_minutes += p.minutes;
        }
        s.total_sessions += u64::from(p.play_count);
        if let Some(r) = g.rating {
            rating_sum += u64::from(r.get());
            rated += 1;
        }
        if g.is_goat && s.goat.is_none() {
            s.goat = Some(g.id);
        }
    }
    s.total_playtime = format_playtime(s.total_minutes);
    s.average_rating = (rated > 0).then(|| rating_sum as f64 / rated as f64);

    let mut by_time: Vec<&(Game, PlayStats)> = games.iter().filter(|(_, p)| p.minutes > 0.0).collect();
    by_time.sort_by(|a, b| b.1.minutes.partial_cmp(&a.1.minutes).unwrap_or(Ordering::Equal).then(a.0.id.cmp(&b.0.id)));
    s.most_played = by_time.iter().take(3).map(|(g, _)| g.id).collect();

    let mut recent: Vec<&(Game, PlayStats)> = games.iter().filter(|(_, p)| p.last_played.is_some()).collect();
    recent.sort_by_key(|r| std::cmp::Reverse(r.1.last_played));
    s.recently_played = recent.iter().take(6).map(|(g, _)| g.id).collect();
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GameDraft, Rating};
    use time::macros::datetime;
    use time::Duration;

    fn game(title: &str, status: Status, rating: Option<i64>) -> Game {
        let d = GameDraft {
            title: title.into(),
            status,
            rating: rating.map(|r| Rating::new(r).unwrap()),
            ..GameDraft::default()
        };
        d.into_game(datetime!(2026-01-01 00:00 UTC)).unwrap()
    }

    fn stats(min: f64, count: u32, last: Option<i64>) -> PlayStats {
        PlayStats {
            minutes: min,
            play_count: count,
            last_played: last.map(|h| datetime!(2026-01-01 00:00 UTC) + Duration::hours(h)),
        }
    }

    #[test]
    fn empty_library() {
        let s = dashboard_stats(&[]);
        assert_eq!(s.total, 0);
        assert_eq!(s.total_playtime, "0m");
        assert!(s.average_rating.is_none());
        assert!(s.most_played.is_empty());
    }

    #[test]
    fn counts_and_rankings() {
        let mut goat = game("Goat", Status::Completed, Some(5));
        goat.is_goat = true;
        let data = vec![
            (game("A", Status::Playing, Some(4)), stats(100.0, 2, Some(5))),
            (game("B", Status::Backlog, None), stats(0.0, 0, None)),
            (goat.clone(), stats(500.0, 10, Some(1))),
            (game("C", Status::Dropped, Some(1)), stats(30.0, 1, Some(9))),
            (game("D", Status::Playing, None), stats(60.0, 1, Some(2))),
        ];
        let s = dashboard_stats(&data);
        assert_eq!(s.total, 5);
        assert_eq!((s.playing, s.completed, s.backlog, s.dropped), (2, 1, 1, 1));
        assert!((s.total_minutes - 690.0).abs() < 1e-9);
        assert_eq!(s.total_playtime, "11h 30m");
        assert_eq!(s.total_sessions, 14);
        assert_eq!(s.goat, Some(goat.id));
        assert_eq!(s.most_played.len(), 3);
        assert_eq!(s.most_played[0], goat.id);
        assert_eq!(s.recently_played[0], data[3].0.id);
        let avg = s.average_rating.unwrap();
        assert!((avg - 10.0 / 3.0).abs() < 1e-9);
    }
}
