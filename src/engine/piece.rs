#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tetromino {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Tetromino {
    pub const ALL: [Tetromino; 7] = [
        Tetromino::I,
        Tetromino::O,
        Tetromino::T,
        Tetromino::S,
        Tetromino::Z,
        Tetromino::J,
        Tetromino::L,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    R0,
    R1,
    R2,
    R3,
}

impl Rotation {
    pub fn cw(self) -> Self {
        match self {
            Rotation::R0 => Rotation::R1,
            Rotation::R1 => Rotation::R2,
            Rotation::R2 => Rotation::R3,
            Rotation::R3 => Rotation::R0,
        }
    }

    pub fn ccw(self) -> Self {
        match self {
            Rotation::R0 => Rotation::R3,
            Rotation::R1 => Rotation::R0,
            Rotation::R2 => Rotation::R1,
            Rotation::R3 => Rotation::R2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Cellules de chaque pièce dans sa boîte englobante (4x4 pour I, 3x3 pour T, S, Z, J, L,
/// avec O dans une 4x4 également). Coordonnées (x, y), y vers le bas.
/// Indexation : SHAPES[pièce][rotation][cellule]
const SHAPES: [[[(i8, i8); 4]; 4]; 7] = [
    // I
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ],
    // O
    [
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
    ],
    // T
    [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // S
    [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
        [(1, 1), (2, 1), (0, 2), (1, 2)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // Z
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(2, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (1, 2), (2, 2)],
        [(1, 0), (0, 1), (1, 1), (0, 2)],
    ],
    // J
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (2, 2)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ],
    // L
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (1, 2), (2, 2)],
        [(0, 1), (1, 1), (2, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    kind: Tetromino,
    rotation: Rotation,
    /// Coin supérieur gauche de la boîte englobante, en coordonnées du plateau.
    position: Position,
}

impl Piece {
    pub fn new(kind: Tetromino, position: Position) -> Self {
        Self {
            kind,
            rotation: Rotation::R0,
            position,
        }
    }

    // --- Lecture ---

    pub fn kind(&self) -> Tetromino {
        self.kind
    }
    pub fn rotation(&self) -> Rotation {
        self.rotation
    }
    pub fn position(&self) -> Position {
        self.position
    }

    /// Les 4 cellules occupées, en coordonnées du plateau.
    pub fn cells(&self) -> [(i32, i32); 4] {
        let shape = SHAPES[self.kind as usize][self.rotation as usize];
        shape.map(|(dx, dy)| (self.position.x + dx as i32, self.position.y + dy as i32))
    }

    // --- Candidats : chaque méthode renvoie une NOUVELLE pièce, `self` n'est pas modifié ---

    #[must_use]
    pub fn moved(&self, dx: i32, dy: i32) -> Piece {
        Piece {
            position: Position {
                x: self.position.x + dx,
                y: self.position.y + dy,
            },
            ..*self
        }
    }

    #[must_use]
    pub fn rotated_cw(&self) -> Piece {
        Piece {
            rotation: self.rotation.cw(),
            ..*self
        }
    }

    #[must_use]
    pub fn rotated_ccw(&self) -> Piece {
        Piece {
            rotation: self.rotation.ccw(),
            ..*self
        }
    }

    #[must_use]
    pub fn moved_left(&self) -> Piece {
        self.moved(-1, 0)
    }

    #[must_use]
    pub fn moved_right(&self) -> Piece {
        self.moved(1, 0)
    }

    #[must_use]
    pub fn moved_down(&self) -> Piece {
        self.moved(0, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: Position = Position { x: 0, y: 0 };

    #[test]
    fn every_shape_has_four_distinct_cells_in_its_box() {
        for kind in Tetromino::ALL {
            let mut piece = Piece::new(kind, ORIGIN);
            for _ in 0..4 {
                let cells = piece.cells();
                for (i, a) in cells.iter().enumerate() {
                    assert!(
                        (0..4).contains(&a.0) && (0..4).contains(&a.1),
                        "{kind:?} {:?} hors boîte",
                        piece.rotation()
                    );
                    for b in &cells[i + 1..] {
                        assert_ne!(
                            a,
                            b,
                            "{kind:?} {:?} a une cellule en double",
                            piece.rotation()
                        );
                    }
                }
                piece = piece.rotated_cw();
            }
        }
    }

    #[test]
    fn four_clockwise_rotations_come_back_to_start() {
        let piece = Piece::new(Tetromino::T, ORIGIN);
        assert_eq!(
            piece.rotated_cw().rotated_cw().rotated_cw().rotated_cw(),
            piece
        );
        assert_eq!(piece.rotated_ccw().rotated_cw(), piece);
    }

    #[test]
    fn cells_are_translated_by_position() {
        let piece = Piece::new(Tetromino::T, Position { x: 3, y: 5 });
        assert_eq!(piece.cells(), [(4, 5), (3, 6), (4, 6), (5, 6)]);
    }

    #[test]
    fn moved_returns_a_new_piece_and_leaves_the_original_untouched() {
        let piece = Piece::new(Tetromino::T, Position { x: 3, y: 5 });
        let candidate = piece.moved(-1, 1);

        assert_eq!(candidate.position(), Position { x: 2, y: 6 });
        assert_eq!(candidate.cells(), [(3, 6), (2, 7), (3, 7), (4, 7)]);
        assert_eq!(piece.position(), Position { x: 3, y: 5 });
    }

    #[test]
    fn rotated_cw_changes_only_the_rotation() {
        let piece = Piece::new(Tetromino::T, ORIGIN);
        let candidate = piece.rotated_cw();

        assert_eq!(candidate.rotation(), Rotation::R1);
        assert_eq!(candidate.kind(), Tetromino::T);
        assert_eq!(candidate.position(), ORIGIN);
        assert_eq!(candidate.cells(), [(1, 0), (1, 1), (2, 1), (1, 2)]);
    }

    #[test]
    fn directional_moves_use_screen_coordinates() {
        let piece = Piece::new(Tetromino::T, Position { x: 3, y: 5 });

        assert_eq!(piece.moved_left().position(), Position { x: 2, y: 5 });
        assert_eq!(piece.moved_right().position(), Position { x: 4, y: 5 });
        assert_eq!(piece.moved_down().position(), Position { x: 3, y: 6 });
    }

    #[test]
    fn ccw_undoes_cw_for_every_rotation() {
        for r in [Rotation::R0, Rotation::R1, Rotation::R2, Rotation::R3] {
            assert_eq!(r.cw().ccw(), r);
            assert_eq!(r.ccw().cw(), r);
        }
    }
}
