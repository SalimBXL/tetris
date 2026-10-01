use super::piece::Tetromino;

/// Petit générateur pseudo-aléatoire déterministe (SplitMix64).
#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Distribue les 7 pièces dans un ordre mélangé, puis recommence avec un nouveau mélange.
#[derive(Debug, Clone)]
pub struct Bag {
    rng: SplitMix64,
    pieces: [Tetromino; 7],
    next_index: usize,
}

impl Bag {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SplitMix64::new(seed),
            pieces: Tetromino::ALL,
            // Sac « vide » au départ : le premier appel à `draw` déclenche un mélange.
            next_index: Tetromino::ALL.len(),
        }
    }

    pub fn draw(&mut self) -> Tetromino {
        if self.next_index == self.pieces.len() {
            self.shuffle();
            self.next_index = 0;
        }
        let piece = self.pieces[self.next_index];
        self.next_index += 1;
        piece
    }

    /// Mélange de Fisher-Yates.
    fn shuffle(&mut self) {
        for i in (1..self.pieces.len()).rev() {
            let j = (self.rng.next_u64() % (i as u64 + 1)) as usize;
            self.pieces.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw_many(seed: u64, n: usize) -> Vec<Tetromino> {
        let mut bag = Bag::new(seed);
        (0..n).map(|_| bag.draw()).collect()
    }

    #[test]
    fn every_group_of_seven_contains_each_piece_once() {
        for chunk in draw_many(42, 70).chunks(7) {
            for kind in Tetromino::ALL {
                assert!(chunk.contains(&kind), "{kind:?} manque dans {chunk:?}");
            }
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_sequence() {
        assert_eq!(draw_many(7, 30), draw_many(7, 30));
    }

    #[test]
    fn different_seeds_give_different_sequences() {
        assert_ne!(draw_many(1, 14), draw_many(2, 14));
    }
}
