use std::time::Duration;

const LINES_PER_LEVEL: u32 = 10;
/// Au-delà de ce niveau, la vitesse ne change plus (et l'intervalle reste strictement positif).
const MAX_SPEED_LEVEL: u32 = 20;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Score {
    points: u32,
    lines: u32,
}

impl Score {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn points(&self) -> u32 {
        self.points
    }
    pub fn lines(&self) -> u32 {
        self.lines
    }

    /// Un niveau de plus toutes les 10 lignes, en commençant au niveau 1.
    pub fn level(&self) -> u32 {
        1 + self.lines / LINES_PER_LEVEL
    }

    /// Les points sont calculés avec le niveau d'AVANT les lignes qu'on vient de supprimer.
    pub fn add_lines(&mut self, cleared: usize) {
        let base = match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };
        self.points += base * self.level();
        self.lines += cleared as u32;
    }

    /// 1 point par case descendue volontairement.
    pub fn add_soft_drop(&mut self, cells: u32) {
        self.points += cells;
    }

    /// 2 points par case descendue d'un coup.
    pub fn add_hard_drop(&mut self, cells: u32) {
        self.points += 2 * cells;
    }
}

/// Temps entre deux descentes automatiques : (0,8 - (niveau-1) × 0,007) ^ (niveau-1) secondes.
/// C'est la formule du guideline officiel : 1 s au niveau 1, environ 0,79 s au niveau 2, etc.
pub fn gravity_interval(level: u32) -> Duration {
    let level = level.clamp(1, MAX_SPEED_LEVEL);
    let n = (level - 1) as i32;
    let seconds = (0.8 - n as f64 * 0.007).powi(n);
    Duration::from_secs_f64(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_score_is_zero_at_level_one() {
        let score = Score::new();
        assert_eq!((score.points(), score.lines(), score.level()), (0, 0, 1));
    }

    #[test]
    fn line_clears_are_worth_more_when_there_are_more_lines() {
        for (cleared, expected) in [(1, 100), (2, 300), (3, 500), (4, 800)] {
            let mut score = Score::new();
            score.add_lines(cleared);
            assert_eq!(score.points(), expected);
            assert_eq!(score.lines(), cleared as u32);
        }
    }

    #[test]
    fn the_level_goes_up_every_ten_lines_and_multiplies_the_points() {
        let mut score = Score::new();
        for _ in 0..3 {
            score.add_lines(4); // 3 × 800 au niveau 1
        }
        assert_eq!(
            (score.points(), score.lines(), score.level()),
            (2400, 12, 2)
        );

        score.add_lines(1); // 100 × niveau 2
        assert_eq!(score.points(), 2600);
    }

    #[test]
    fn soft_drop_gives_one_point_per_cell() {
        let mut score = Score::new();
        score.add_soft_drop(3);
        assert_eq!(score.points(), 3);
    }

    #[test]
    fn hard_drop_gives_two_points_per_cell() {
        let mut score = Score::new();
        score.add_hard_drop(10);
        assert_eq!(score.points(), 20);
    }

    #[test]
    fn gravity_starts_at_one_second_and_gets_faster_every_level() {
        assert_eq!(gravity_interval(1), Duration::from_secs(1));
        for level in 1..MAX_SPEED_LEVEL {
            assert!(gravity_interval(level) > gravity_interval(level + 1));
        }
    }

    #[test]
    fn gravity_never_reaches_zero() {
        assert!(gravity_interval(MAX_SPEED_LEVEL) > Duration::ZERO);
        assert_eq!(gravity_interval(500), gravity_interval(MAX_SPEED_LEVEL));
    }
}
