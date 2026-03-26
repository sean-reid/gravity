/// A single leaderboard entry representing one completed level run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeaderboardEntry {
    pub level_number: u32,
    pub seed: u64,
    pub score: u64,
    pub proper_time: f64,
    pub accuracy: f64,
    pub health_remaining: f64,
    pub timestamp: u64,
}

/// Maximum number of entries stored per seed.
const MAX_ENTRIES_PER_SEED: usize = 10;

/// Local leaderboard storing the top scores per level seed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalLeaderboard {
    pub entries: Vec<LeaderboardEntry>,
}

impl LocalLeaderboard {
    /// Create an empty leaderboard.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Submit an entry. Returns `Some(rank)` (0-indexed) if the entry makes the
    /// top 10 for its seed, or `None` if it did not place.
    pub fn submit(&mut self, entry: LeaderboardEntry) -> Option<usize> {
        let seed = entry.seed;

        // Collect entries for this seed, add the new one, sort descending by score
        let mut seed_entries: Vec<LeaderboardEntry> = self
            .entries
            .iter()
            .filter(|e| e.seed == seed)
            .cloned()
            .collect();

        seed_entries.push(entry.clone());
        seed_entries.sort_by(|a, b| b.score.cmp(&a.score));

        // Find rank of the new entry (first occurrence with matching score+timestamp)
        let rank = seed_entries
            .iter()
            .position(|e| e.score == entry.score && e.timestamp == entry.timestamp);

        // Check if it made the cut
        let rank = match rank {
            Some(r) if r < MAX_ENTRIES_PER_SEED => r,
            _ => return None,
        };

        // Truncate to top N
        seed_entries.truncate(MAX_ENTRIES_PER_SEED);

        // Replace all entries for this seed in the main list
        self.entries.retain(|e| e.seed != seed);
        self.entries.extend(seed_entries);

        Some(rank)
    }

    /// Get the top `count` entries for a given seed, sorted by descending score.
    pub fn get_top(&self, seed: u64, count: usize) -> Vec<&LeaderboardEntry> {
        let mut seed_entries: Vec<&LeaderboardEntry> =
            self.entries.iter().filter(|e| e.seed == seed).collect();
        seed_entries.sort_by(|a, b| b.score.cmp(&a.score));
        seed_entries.truncate(count);
        seed_entries
    }

    /// Total number of stored entries across all seeds.
    pub fn total_entries(&self) -> usize {
        self.entries.len()
    }
}

impl Default for LocalLeaderboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(seed: u64, score: u64, ts: u64) -> LeaderboardEntry {
        LeaderboardEntry {
            level_number: 1,
            seed,
            score,
            proper_time: 60.0,
            accuracy: 0.8,
            health_remaining: 1.0,
            timestamp: ts,
        }
    }

    #[test]
    fn submit_returns_rank() {
        let mut lb = LocalLeaderboard::new();
        let rank = lb.submit(make_entry(42, 1000, 1));
        assert_eq!(rank, Some(0));

        let rank = lb.submit(make_entry(42, 2000, 2));
        assert_eq!(rank, Some(0)); // New top score

        let rank = lb.submit(make_entry(42, 500, 3));
        assert_eq!(rank, Some(2)); // Third place
    }

    #[test]
    fn max_entries_per_seed() {
        let mut lb = LocalLeaderboard::new();
        for i in 0..10 {
            let rank = lb.submit(make_entry(42, (i + 1) * 100, i));
            assert!(rank.is_some());
        }
        // 11th entry with lowest score should not place
        let rank = lb.submit(make_entry(42, 50, 100));
        assert_eq!(rank, None);

        // But a high score should displace the lowest
        let rank = lb.submit(make_entry(42, 5000, 101));
        assert_eq!(rank, Some(0));
        assert_eq!(lb.get_top(42, 20).len(), 10); // Still max 10
    }

    #[test]
    fn get_top_sorted() {
        let mut lb = LocalLeaderboard::new();
        lb.submit(make_entry(42, 300, 1));
        lb.submit(make_entry(42, 100, 2));
        lb.submit(make_entry(42, 200, 3));

        let top = lb.get_top(42, 10);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].score, 300);
        assert_eq!(top[1].score, 200);
        assert_eq!(top[2].score, 100);
    }

    #[test]
    fn separate_seeds() {
        let mut lb = LocalLeaderboard::new();
        lb.submit(make_entry(1, 500, 1));
        lb.submit(make_entry(2, 1000, 2));

        assert_eq!(lb.get_top(1, 10).len(), 1);
        assert_eq!(lb.get_top(2, 10).len(), 1);
        assert_eq!(lb.get_top(3, 10).len(), 0);
    }
}
