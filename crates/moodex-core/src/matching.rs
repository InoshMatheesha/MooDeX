// matching.rs - Fuzzy ranking for cover matching and duplicate suggestions; never merges.

use crate::model::{DuplicateCandidate, DuplicateReason, Game, RawgGame, Scored};
use crate::text::{match_key, match_tokens, similarity};
use std::cmp::Ordering;
use std::collections::HashMap;

// Score one candidate title against a query in [0, ~1.1].
fn title_score(query_tokens: &[String], cand: &str) -> f64 {
    let ct = match_tokens(cand);
    if query_tokens.is_empty() || ct.is_empty() {
        return 0.0;
    }
    let q = query_tokens.join(" ");
    let c = ct.join(" ");
    if q == c {
        return 1.1;
    }
    let qset: Vec<&String> = query_tokens.iter().collect();
    let common = ct.iter().filter(|t| qset.contains(t)).count();
    let union = query_tokens.len() + ct.len() - common;
    let jaccard = if union == 0 { 0.0 } else { common as f64 / union as f64 };
    // Coverage of the query by the candidate rewards "cyberpunk 2077" over sequels with extra words.
    let coverage = common as f64 / query_tokens.len() as f64;
    let prefix = if c.starts_with(&q) || q.starts_with(&c) { 0.1 } else { 0.0 };
    0.45 * jaccard + 0.25 * coverage + 0.3 * similarity(&q, &c) + prefix
}

// Rank RAWG results against a query such as an exe stem.
pub fn rank_matches(query: &str, candidates: &[RawgGame]) -> Vec<Scored<RawgGame>> {
    let qt = match_tokens(query);
    let max_added = candidates.iter().map(|c| c.added).max().unwrap_or(0).max(1) as f64;
    let mut scored: Vec<Scored<RawgGame>> = candidates
        .iter()
        .map(|c| {
            // Popularity is only a small tie-breaker.
            let pop = (c.added.max(0) as f64 / max_added) * 0.02;
            Scored { score: title_score(&qt, &c.name) + pop, item: c.clone() }
        })
        .collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    scored
}

// Numeric tokens of a match key, e.g. "witcher 3" -> ["3"].
fn numbers(key: &str) -> Vec<&str> {
    key.split(' ').filter(|t| t.chars().all(|c| c.is_ascii_digit())).collect()
}

// Pairs that look like the same game. Suggestions only (spec 1.3).
pub fn find_duplicate_candidates(games: &[Game]) -> Vec<DuplicateCandidate> {
    let mut out = Vec::new();
    let keys: Vec<(String, &Game)> = games.iter().map(|g| (match_key(&g.title), g)).collect();

    let mut groups: HashMap<&str, Vec<&Game>> = HashMap::new();
    for (k, g) in &keys {
        if !k.is_empty() {
            groups.entry(k.as_str()).or_default().push(g);
        }
    }
    for list in groups.values() {
        for (i, a) in list.iter().enumerate() {
            for b in list.iter().skip(i + 1) {
                out.push(DuplicateCandidate { a: a.id, b: b.id, reason: DuplicateReason::SameTitle });
            }
        }
    }

    // Near matches, bounded to keep this cheap on large libraries.
    if keys.len() <= 3000 {
        for (i, (ka, ga)) in keys.iter().enumerate() {
            for (kb, gb) in keys.iter().skip(i + 1) {
                if ka == kb || ka.is_empty() || kb.is_empty() {
                    continue;
                }
                let len_gap = ka.len().abs_diff(kb.len());
                if len_gap > 3 || ka.len() < 6 {
                    continue;
                }
                // Sequels differ only by a number - never suggest those.
                if numbers(ka) != numbers(kb) {
                    continue;
                }
                let s = similarity(ka, kb);
                if s >= 0.85 {
                    out.push(DuplicateCandidate {
                        a: ga.id,
                        b: gb.id,
                        reason: DuplicateReason::SimilarTitle { score: s },
                    });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GameDraft;
    use time::macros::datetime;

    fn rg(id: i64, name: &str, added: i64) -> RawgGame {
        RawgGame { id, name: name.into(), added, ..RawgGame::default() }
    }

    #[test]
    fn exe_stem_ranks_right_title_first() {
        let c = vec![
            rg(1, "Cyberpunk 2077: Phantom Liberty", 9000),
            rg(2, "Cyber Hook", 100),
            rg(3, "Cyberpunk 2077", 8000),
            rg(4, "Cyberpunk Red", 50),
        ];
        let r = rank_matches("Cyberpunk2077.exe", &c);
        assert_eq!(r[0].item.id, 3);
    }

    #[test]
    fn roman_numerals_match() {
        let c = vec![rg(1, "The Witcher", 10), rg(2, "The Witcher 3: Wild Hunt", 10), rg(3, "The Witcher 2", 10)];
        let r = rank_matches("witcher3", &c);
        assert_eq!(r[0].item.id, 2);
    }

    #[test]
    fn empty_query_scores_zero() {
        let r = rank_matches("", &[rg(1, "Halo", 1)]);
        assert!(r[0].score < 0.05);
    }

    fn g(title: &str) -> Game {
        GameDraft { title: title.into(), ..GameDraft::default() }.into_game(datetime!(2026-01-01 00:00 UTC)).unwrap()
    }

    #[test]
    fn duplicates_are_suggested_not_merged() {
        let games = vec![g("The Witcher 3"), g("Witcher III"), g("Halo"), g("Celeste"), g("Celestee"), g("Witcher 2")];
        let d = find_duplicate_candidates(&games);
        assert!(!d.iter().any(|c| c.a == games[5].id || c.b == games[5].id), "sequel suggested: {d:?}");
        assert!(d.iter().any(|c| c.a == games[0].id && c.b == games[1].id && c.reason == DuplicateReason::SameTitle));
        assert!(d.iter().any(|c| matches!(c.reason, DuplicateReason::SimilarTitle { .. })));
        assert!(!d.iter().any(|c| c.a == games[2].id || c.b == games[2].id));
        // Input untouched: still six distinct games.
        assert_eq!(games.len(), 6);
    }
}
