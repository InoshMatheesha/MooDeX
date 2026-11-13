// query.rs - Filtering and ordering of the in-memory library.

use crate::model::{Game, GameFilter, GameId, PlayStats, SortOrder};
use crate::text::sort_title;
use std::cmp::Ordering;

// Does a game pass the filter?
pub fn matches_filter(g: &Game, f: &GameFilter) -> bool {
    if let Some(s) = f.status {
        if g.status != s {
            return false;
        }
    }
    if f.installed_only && g.install.is_none() {
        return false;
    }
    if let Some(q) = f.query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let q = q.to_lowercase();
        let hit = g.title.to_lowercase().contains(&q) || g.genres.iter().any(|x| x.to_lowercase().contains(&q));
        if !hit {
            return false;
        }
    }
    true
}

// Filter then order; returns ids so the UI can patch cards by key.
pub fn filter_and_sort(games: &[(Game, PlayStats)], f: &GameFilter, sort: SortOrder) -> Vec<GameId> {
    let mut rows: Vec<(&Game, &PlayStats, String)> =
        games.iter().filter(|(g, _)| matches_filter(g, f)).map(|(g, p)| (g, p, sort_title(&g.title))).collect();
    let by_title =
        |a: &(&Game, &PlayStats, String), b: &(&Game, &PlayStats, String)| a.2.cmp(&b.2).then(a.0.id.cmp(&b.0.id));
    rows.sort_by(|a, b| {
        let primary = match sort {
            SortOrder::Title => Ordering::Equal,
            SortOrder::RecentlyAdded => b.0.added_at.cmp(&a.0.added_at),
            SortOrder::Playtime => b.1.minutes.partial_cmp(&a.1.minutes).unwrap_or(Ordering::Equal),
            SortOrder::Rating => b.0.rating.cmp(&a.0.rating),
            SortOrder::LastPlayed => b.1.last_played.cmp(&a.1.last_played),
            SortOrder::ReleaseDate => b.0.release_date.cmp(&a.0.release_date),
        };
        primary.then_with(|| by_title(a, b))
    });
    rows.into_iter().map(|(g, _, _)| g.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GameDraft, Install, LaunchTarget, Rating, Status};
    use time::macros::datetime;

    fn g(title: &str, status: Status, installed: bool, rating: Option<i64>) -> Game {
        let install = installed.then(|| Install {
            launch: LaunchTarget::Exe { path: "C:/x/y.exe".into(), args: vec![], working_dir: None },
            detect_exe: None,
        });
        GameDraft {
            title: title.into(),
            status,
            install,
            rating: rating.map(|r| Rating::new(r).unwrap()),
            genres: vec!["Action".into()],
            ..GameDraft::default()
        }
        .into_game(datetime!(2026-01-01 00:00 UTC))
        .unwrap()
    }

    #[test]
    fn filters_and_sorts() {
        let p = PlayStats::default();
        let data = vec![
            (g("The Witcher 3", Status::Playing, true, Some(5)), PlayStats { minutes: 10.0, ..p }),
            (g("Alan Wake", Status::Backlog, false, None), PlayStats { minutes: 50.0, ..p }),
            (g("Hades", Status::Playing, true, Some(4)), p),
        ];
        let all = filter_and_sort(&data, &GameFilter::default(), SortOrder::Title);
        assert_eq!(all, vec![data[1].0.id, data[2].0.id, data[0].0.id]);

        let pc =
            filter_and_sort(&data, &GameFilter { installed_only: true, ..GameFilter::default() }, SortOrder::Title);
        assert_eq!(pc, vec![data[2].0.id, data[0].0.id]);

        let playing = GameFilter { status: Some(Status::Playing), ..GameFilter::default() };
        assert_eq!(filter_and_sort(&data, &playing, SortOrder::Rating), vec![data[0].0.id, data[2].0.id]);

        let by_time = filter_and_sort(&data, &GameFilter::default(), SortOrder::Playtime);
        assert_eq!(by_time[0], data[1].0.id);

        let q = GameFilter { query: Some("wake".into()), ..GameFilter::default() };
        assert_eq!(filter_and_sort(&data, &q, SortOrder::Title), vec![data[1].0.id]);
        let genre = GameFilter { query: Some("action".into()), ..GameFilter::default() };
        assert_eq!(filter_and_sort(&data, &genre, SortOrder::Title).len(), 3);
    }
}
