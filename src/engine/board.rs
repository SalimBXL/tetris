use super::piece::{Piece, Tetromino};

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 20;

type Row = [Option<Tetromino>; BOARD_WIDTH];

/// Contient uniquement les blocs déjà verrouillés (jamais la pièce courante).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    cells: [Row; BOARD_HEIGHT],
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    pub fn new() -> Self {
        Self {
            cells: [[None; BOARD_WIDTH]; BOARD_HEIGHT],
        }
    }

    /// Lecture d'une case, pour le renderer. `x` et `y` doivent être dans le plateau.
    pub fn get(&self, x: usize, y: usize) -> Option<Tetromino> {
        self.cells[y][x]
    }

    /// Seul endroit où l'on passe de coordonnées signées (pièce) à des index (plateau).
    fn to_index(x: i32, y: i32) -> Option<(usize, usize)> {
        if (0..BOARD_WIDTH as i32).contains(&x) && (0..BOARD_HEIGHT as i32).contains(&y) {
            Some((x as usize, y as usize))
        } else {
            None
        }
    }

    /// La pièce est-elle entièrement dans le plateau, sur des cases libres ?
    pub fn can_place(&self, piece: &Piece) -> bool {
        piece.cells().iter().all(|&(x, y)| {
            Self::to_index(x, y).is_some_and(|(cx, cy)| self.cells[cy][cx].is_none())
        })
    }

    /// Fixe la pièce dans le plateau. À appeler seulement si `can_place` est vrai.
    pub fn lock(&mut self, piece: &Piece) {
        debug_assert!(self.can_place(piece));
        for (x, y) in piece.cells() {
            if let Some((cx, cy)) = Self::to_index(x, y) {
                self.cells[cy][cx] = Some(piece.kind());
            }
        }
    }

    /// Supprime les lignes pleines, fait descendre le reste, renvoie le nombre de lignes supprimées.
    pub fn clear_lines(&mut self) -> usize {
        let mut new_cells = [[None; BOARD_WIDTH]; BOARD_HEIGHT];
        let mut write = BOARD_HEIGHT; // on remplit le nouveau plateau de bas en haut

        for row in self.cells.iter().rev() {
            if row.iter().any(|cell| cell.is_none()) {
                write -= 1;
                new_cells[write] = *row;
            }
        }

        self.cells = new_cells;
        write // les `write` lignes du haut sont vides : c'est le nombre de lignes supprimées
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::piece::Position;

    /// Construit un plateau à partir de lignes de texte alignées en bas.
    /// '.' = vide, tout autre caractère = case occupée.
    fn board_from(rows: &[&str]) -> Board {
        let mut board = Board::new();
        let offset = BOARD_HEIGHT - rows.len();
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), BOARD_WIDTH);
            for (x, c) in row.chars().enumerate() {
                if c != '.' {
                    board.cells[offset + i][x] = Some(Tetromino::T);
                }
            }
        }
        board
    }

    /// Les `n` dernières lignes du plateau, en texte.
    fn bottom_rows(board: &Board, n: usize) -> Vec<String> {
        (BOARD_HEIGHT - n..BOARD_HEIGHT)
            .map(|y| {
                (0..BOARD_WIDTH)
                    .map(|x| if board.get(x, y).is_some() { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    fn t_at(x: i32, y: i32) -> Piece {
        Piece::new(Tetromino::T, Position { x, y })
    }

    #[test]
    fn piece_fits_on_an_empty_board() {
        assert!(Board::new().can_place(&t_at(3, 0)));
    }

    #[test]
    fn piece_cannot_leave_through_the_left_wall() {
        let board = Board::new();
        assert!(board.can_place(&t_at(0, 0)));
        assert!(!board.can_place(&t_at(-1, 0)));
    }

    #[test]
    fn piece_cannot_leave_through_the_right_wall() {
        let board = Board::new();
        assert!(board.can_place(&t_at(7, 0)));
        assert!(!board.can_place(&t_at(8, 0)));
    }

    #[test]
    fn piece_cannot_go_below_the_floor() {
        let board = Board::new();
        assert!(board.can_place(&t_at(3, 18)));
        assert!(!board.can_place(&t_at(3, 19)));
    }

    #[test]
    fn piece_cannot_overlap_locked_blocks() {
        let board = board_from(&["##########"]);
        assert!(board.can_place(&t_at(3, 17)));
        assert!(!board.can_place(&t_at(3, 18)));
    }

    #[test]
    fn lock_writes_the_piece_cells_into_the_board() {
        let mut board = Board::new();
        board.lock(&t_at(3, 18));

        assert_eq!(board.get(4, 18), Some(Tetromino::T));
        assert_eq!(board.get(3, 19), Some(Tetromino::T));
        assert_eq!(board.get(4, 19), Some(Tetromino::T));
        assert_eq!(board.get(5, 19), Some(Tetromino::T));
        assert_eq!(board.get(0, 0), None);
    }

    #[test]
    fn clear_lines_returns_zero_when_nothing_is_full() {
        let mut board = board_from(&["#########."]);
        assert_eq!(board.clear_lines(), 0);
        assert_eq!(bottom_rows(&board, 1), ["#########."]);
    }

    #[test]
    fn clear_lines_removes_full_rows_and_drops_the_rest() {
        let mut board = board_from(&["..#.......", "##########", "#########.", "##########"]);

        assert_eq!(board.clear_lines(), 2);
        assert_eq!(
            bottom_rows(&board, 4),
            ["..........", "..........", "..#.......", "#########."]
        );
    }
}
