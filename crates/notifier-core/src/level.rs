//! Watching the user's level.
//!
//! The level goes up when enough lessons have been finished, which is worth
//! congratulating the user about — but only once.

/// Watches the user's level and notices when it goes up.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelWatcher {
    /// The level the user was last seen at.
    seen: Option<u32>,
}

impl LevelWatcher {
    /// Creates a watcher that has not seen a level yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a watcher that has already seen the given level.
    pub fn seen_at(level: u32) -> Self {
        Self { seen: Some(level) }
    }

    /// The level the user was last seen at.
    pub fn seen(&self) -> Option<u32> {
        self.seen
    }

    /// Feeds in the user's current level.
    ///
    /// Returns true when the level has gone up since the last time, which is
    /// the moment worth congratulating the user on. The first level seen is
    /// only remembered: there is nothing to be congratulated about yet.
    pub fn observe(&mut self, level: u32) -> bool {
        match self.seen {
            Some(seen) if level > seen => {
                self.seen = Some(level);
                true
            }
            Some(seen) => {
                // A level never goes down, but if it somehow does, keep the
                // higher one so the same level is not announced twice.
                self.seen = Some(seen.max(level));
                false
            }
            None => {
                self.seen = Some(level);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_level_is_only_remembered() {
        let mut watcher = LevelWatcher::new();
        assert!(!watcher.observe(5));
        assert_eq!(watcher.seen(), Some(5));
    }

    #[test]
    fn going_up_is_noticed() {
        let mut watcher = LevelWatcher::new();
        watcher.observe(5);
        assert!(watcher.observe(6));
        assert_eq!(watcher.seen(), Some(6));
    }

    #[test]
    fn the_same_level_is_not_announced_twice() {
        let mut watcher = LevelWatcher::new();
        watcher.observe(5);
        assert!(!watcher.observe(5));
        assert!(!watcher.observe(5));
    }

    #[test]
    fn a_level_up_that_happened_while_away_is_still_noticed() {
        // The watcher starts from the level it saw last time, so a level up
        // that happened while the app was not running is still caught.
        let mut watcher = LevelWatcher::seen_at(5);
        assert!(watcher.observe(6));
    }

    #[test]
    fn a_level_up_is_only_announced_once() {
        let mut watcher = LevelWatcher::new();
        watcher.observe(5);
        assert!(watcher.observe(6));
        assert!(!watcher.observe(6));
        assert!(watcher.observe(7));
    }

    #[test]
    fn a_level_that_does_not_go_up_is_ignored() {
        let mut watcher = LevelWatcher::new();
        watcher.observe(5);
        assert!(!watcher.observe(4));
        // The higher level is kept, so 5 is not announced as a level up.
        assert!(!watcher.observe(5));
        assert!(watcher.observe(6));
    }
}
