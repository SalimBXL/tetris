use std::time::Duration;

use super::bag::Bag;
use super::board::Board;
use super::piece::{Piece, Position, Tetromino};
use super::score::{Score, gravity_interval};

/// Coin de la boîte d'une pièce qui apparaît : centrée horizontalement, en haut.
const SPAWN: Position = Position { x: 3, y: 0 };

/// Décalages essayés dans l'ordre quand une rotation est bloquée (wall kicks).
/// (0, 0) = rotation sur place ; les suivants : gauche, droite, haut.
const KICKS: [(i32, i32); 4] = [(0, 0), (-1, 0), (1, 0), (0, -1)];

/// Temps pendant lequel une pièce posée peut encore bouger avant de se fixer.
const LOCK_DELAY: Duration = Duration::from_millis(500);

/// Nombre maximal de remises à zéro du compte à rebours par pièce.
const MAX_LOCK_RESETS: u32 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Left,
    Right,
    SoftDrop,
    HardDrop,
    RotateCw,
    RotateCcw,
    TogglePause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Moved,
    Rotated,
    Locked,
    LinesCleared(usize),
    LevelUp(u32),
    Paused,
    Resumed,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Paused,
    GameOver,
}

#[derive(Debug, Clone)]
pub struct Game {
    board: Board,
    current_piece: Piece,
    bag: Bag,
    /// Pièce annoncée : c'est elle qui apparaîtra au prochain verrouillage.
    next_kind: Tetromino,
    score: Score,
    state: GameState,
    /// Temps écoulé depuis la dernière descente automatique.
    gravity_timer: Duration,
    /// `Some(t)` quand la pièce repose sur quelque chose : temps écoulé depuis qu'elle est posée.
    lock_timer: Option<Duration>,
    /// Nombre de remises à zéro de `lock_timer` pour la pièce courante.
    lock_resets: u32,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut bag = Bag::new(seed);
        let current_piece = Piece::new(bag.draw(), SPAWN);
        let next_kind = bag.draw();
        Self {
            board: Board::new(),
            current_piece,
            bag,
            next_kind,
            score: Score::new(),
            state: GameState::Playing,
            gravity_timer: Duration::ZERO,
            lock_timer: None,
            lock_resets: 0,
        }
    }

    // --- Lecture seule, pour le renderer ---

    pub fn board(&self) -> &Board {
        &self.board
    }
    pub fn current_piece(&self) -> &Piece {
        &self.current_piece
    }
    pub fn score(&self) -> &Score {
        &self.score
    }
    pub fn state(&self) -> GameState {
        self.state
    }

    // --- Points d'entrée qui modifient l'état ---

    pub fn apply(&mut self, action: Action) -> Vec<Event> {
        if self.state != GameState::Playing && action != Action::TogglePause {
            return vec![];
        }

        let event = match action {
            Action::Left => self
                .try_replace_current(self.current_piece.moved_left())
                .then_some(Event::Moved),
            Action::Right => self
                .try_replace_current(self.current_piece.moved_right())
                .then_some(Event::Moved),
            Action::SoftDrop => {
                let moved = self.try_replace_current(self.current_piece.moved_down());
                if moved {
                    self.score.add_soft_drop(1);
                }
                moved.then_some(Event::Moved)
            }
            Action::HardDrop => return self.hard_drop(),
            Action::RotateCw => self
                .try_rotate(self.current_piece.rotated_cw())
                .then_some(Event::Rotated),
            Action::RotateCcw => self
                .try_rotate(self.current_piece.rotated_ccw())
                .then_some(Event::Rotated),
            Action::TogglePause => return self.toggle_pause(),
        };

        if event.is_some() {
            self.refresh_lock_after_move();
        }
        event.into_iter().collect() // Some(e) -> vec![e], None -> vec![]
    }

    /// Fait avancer le temps de `dt`. Le moteur ne lit jamais l'horloge lui-même.
    pub fn tick(&mut self, dt: Duration) -> Vec<Event> {
        let mut events = Vec::new();
        if self.state != GameState::Playing {
            return events;
        }

        self.gravity_timer += dt;
        if let Some(timer) = self.lock_timer.as_mut() {
            *timer += dt;
        }

        // Gravité : la pièce descend tant qu'elle le peut. Une fois posée, la gravité
        // n'a plus d'effet : c'est le délai de verrouillage qui prend le relais.
        let interval = gravity_interval(self.score.level());
        while self.gravity_timer >= interval {
            self.gravity_timer -= interval;
            if self.try_replace_current(self.current_piece.moved_down()) {
                events.push(Event::Moved);
            }
        }

        self.refresh_lock_after_time();
        if self.lock_timer.is_some_and(|t| t >= LOCK_DELAY) {
            events.extend(self.lock_current_and_spawn());
        }
        events
    }

    // --- Interne ---

    /// Descend la pièce jusqu'en bas, la verrouille, et fait apparaître la suivante.
    fn hard_drop(&mut self) -> Vec<Event> {
        let mut cells = 0;
        while self.try_replace_current(self.current_piece.moved_down()) {
            cells += 1;
        }
        self.score.add_hard_drop(cells);
        // La nouvelle pièce repart avec un compte à rebours de gravité neuf.
        self.gravity_timer = Duration::ZERO;

        let mut events = Vec::new();
        if cells > 0 {
            events.push(Event::Moved);
        }
        events.extend(self.lock_current_and_spawn());
        events
    }

    fn toggle_pause(&mut self) -> Vec<Event> {
        match self.state {
            GameState::Playing => {
                self.state = GameState::Paused;
                vec![Event::Paused]
            }
            GameState::Paused => {
                self.state = GameState::Playing;
                vec![Event::Resumed]
            }
            GameState::GameOver => vec![],
        }
    }

    fn lock_current_and_spawn(&mut self) -> Vec<Event> {
        let mut events = vec![Event::Locked];

        self.board.lock(&self.current_piece);
        // La pièce suivante repart d'un état de verrouillage neuf.
        self.lock_timer = None;
        self.lock_resets = 0;

        let cleared = self.board.clear_lines();
        if cleared > 0 {
            let level_before = self.score.level();
            self.score.add_lines(cleared);
            events.push(Event::LinesCleared(cleared));
            if self.score.level() > level_before {
                events.push(Event::LevelUp(self.score.level()));
            }
        }

        let next = Piece::new(self.next_kind, SPAWN);
        if self.board.can_place(&next) {
            self.current_piece = next;
            self.next_kind = self.bag.draw();
        } else {
            self.state = GameState::GameOver;
            events.push(Event::GameOver);
        }
        events
    }

    /// La pièce repose-t-elle sur le sol ou sur des blocs ?
    fn is_grounded(&self) -> bool {
        !self.board.can_place(&self.current_piece.moved_down())
    }

    /// Après un déplacement ou une rotation du joueur qui a réussi.
    fn refresh_lock_after_move(&mut self) {
        if !self.is_grounded() {
            self.lock_timer = None;
            return;
        }
        match self.lock_timer {
            None => self.lock_timer = Some(Duration::ZERO),
            Some(_) if self.lock_resets < MAX_LOCK_RESETS => {
                self.lock_timer = Some(Duration::ZERO);
                self.lock_resets += 1;
            }
            Some(_) => {} // plus de remises à zéro : le compte à rebours continue
        }
    }

    /// À la fin d'un tick : démarre le compte à rebours si la pièce vient de se poser,
    /// l'annule si elle n'est plus posée.
    fn refresh_lock_after_time(&mut self) {
        if !self.is_grounded() {
            self.lock_timer = None;
        } else if self.lock_timer.is_none() {
            self.lock_timer = Some(Duration::ZERO);
        }
    }

    /// Essaie la rotation sur place, puis avec chaque décalage de `KICKS`.
    fn try_rotate(&mut self, rotated: Piece) -> bool {
        KICKS
            .iter()
            .any(|&(dx, dy)| self.try_replace_current(rotated.moved(dx, dy)))
    }

    /// Remplace la pièce courante par `candidate` si elle peut être placée.
    fn try_replace_current(&mut self, candidate: Piece) -> bool {
        if self.board.can_place(&candidate) {
            self.current_piece = candidate;
            true
        } else {
            false
        }
    }

    pub fn next_piece(&self) -> Tetromino {
        self.next_kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bag::Bag;
    use crate::engine::piece::{Rotation, Tetromino};
    use crate::engine::score::gravity_interval;

    /// Intervalle de gravité au niveau 1 (1 seconde).
    fn interval() -> Duration {
        gravity_interval(1)
    }

    /// Prépare la ligne du bas, sauf les colonnes 3 à 5 (la place du T qui va tomber).
    fn leave_room_for_t_on_bottom_row(game: &mut Game) {
        let left_o = Piece::new(Tetromino::O, Position { x: -1, y: 18 }); // colonnes 0-1, lignes 18-19
        let col_2 = Piece::new(Tetromino::I, Position { x: 0, y: 16 }).rotated_cw(); // colonne 2, lignes 16-19
        let right_i = Piece::new(Tetromino::I, Position { x: 6, y: 18 }); // colonnes 6-9, ligne 19
        for piece in [left_o, col_2, right_i] {
            game.board.lock(&piece);
        }
    }

    fn position(game: &Game) -> Position {
        game.current_piece().position()
    }

    /// Partie dont la pièce courante est un T à l'apparition. Les pièces suivantes viennent du sac.
    fn t_game() -> Game {
        let mut game = Game::new(0);
        game.current_piece = Piece::new(Tetromino::T, SPAWN);
        game
    }

    /// Partie où la pièce courante, posée au sol, va se verrouiller au prochain tick,
    /// et où la suivante ne pourra pas apparaître.
    fn about_to_be_over() -> Game {
        let mut game = t_game();
        // Un O verrouillé en haut occupe la case (4, 1), que toute pièce occupe à son apparition.
        game.board
            .lock(&Piece::new(Tetromino::O, Position { x: 3, y: 0 }));
        game.current_piece = Piece::new(Tetromino::T, Position { x: 3, y: 18 });
        game
    }

    /// Fait tourner la gravité jusqu'au prochain verrouillage et renvoie les événements de ce tick.
    fn tick_until_locked(game: &mut Game) -> Vec<Event> {
        for _ in 0..100 {
            let events = game.tick(interval());
            if events.contains(&Event::Locked) {
                return events;
            }
        }
        panic!("la pièce ne s'est jamais verrouillée");
    }

    // --- apply ---

    #[test]
    fn a_new_game_starts_with_a_piece_at_the_spawn_position() {
        let game = Game::new(0);
        assert_eq!(position(&game), SPAWN);
        assert_eq!(game.state(), GameState::Playing);
    }

    #[test]
    fn left_moves_the_piece_until_the_wall() {
        let mut game = t_game();
        for _ in 0..3 {
            assert_eq!(game.apply(Action::Left), vec![Event::Moved]);
        }
        assert_eq!(position(&game).x, 0);

        assert!(game.apply(Action::Left).is_empty());
        assert_eq!(position(&game).x, 0);
    }

    #[test]
    fn right_moves_the_piece_until_the_wall() {
        let mut game = t_game();
        for _ in 0..4 {
            assert_eq!(game.apply(Action::Right), vec![Event::Moved]);
        }
        assert_eq!(position(&game).x, 7);

        assert!(game.apply(Action::Right).is_empty());
        assert_eq!(position(&game).x, 7);
    }

    #[test]
    fn soft_drop_moves_the_piece_down_until_the_floor() {
        let mut game = t_game();
        for _ in 0..18 {
            assert_eq!(game.apply(Action::SoftDrop), vec![Event::Moved]);
        }
        assert_eq!(position(&game).y, 18);

        assert!(game.apply(Action::SoftDrop).is_empty());
        assert_eq!(position(&game).y, 18);
    }

    #[test]
    fn soft_drop_stops_on_locked_blocks() {
        let mut game = t_game();
        // Un I horizontal verrouillé sur la ligne 18, colonnes 0 à 3.
        let blocker = Piece::new(Tetromino::I, Position { x: 0, y: 17 });
        game.board.lock(&blocker);

        for _ in 0..16 {
            assert_eq!(game.apply(Action::SoftDrop), vec![Event::Moved]);
        }
        assert!(game.apply(Action::SoftDrop).is_empty());
        assert_eq!(position(&game).y, 16);
    }

    // --- tick ---

    #[test]
    fn gravity_waits_for_the_full_interval() {
        let mut game = t_game();
        let half = interval() / 2;

        assert!(game.tick(half).is_empty());
        assert_eq!(position(&game).y, 0);

        assert_eq!(game.tick(half), vec![Event::Moved]);
        assert_eq!(position(&game).y, 1);
    }

    #[test]
    fn a_long_tick_applies_gravity_several_times() {
        let mut game = t_game();
        let events = game.tick(interval() * 3);

        assert_eq!(events, vec![Event::Moved; 3]);
        assert_eq!(position(&game).y, 3);
    }

    #[test]
    fn a_piece_that_cannot_fall_is_locked_and_a_new_one_spawns() {
        let mut game = t_game();
        let events = tick_until_locked(&mut game);

        assert_eq!(events, vec![Event::Locked]);
        // Le T est maintenant dans le plateau (case du bas du T).
        assert!(game.board().get(4, 19).is_some());
        // Et une nouvelle pièce est apparue en haut.
        assert_eq!(position(&game), SPAWN);
        assert_eq!(game.state(), GameState::Playing);
    }

    #[test]
    fn completing_a_line_clears_it_and_reports_it() {
        let mut game = t_game();
        leave_room_for_t_on_bottom_row(&mut game);
        let events = tick_until_locked(&mut game);

        assert_eq!(events, vec![Event::Locked, Event::LinesCleared(1)]);
        assert_eq!(game.score().points(), 100);
        assert_eq!(game.score().lines(), 1);
    }

    #[test]
    fn the_game_is_over_when_a_new_piece_cannot_spawn() {
        let mut game = about_to_be_over();

        let events = tick_until_locked(&mut game);

        assert_eq!(events, vec![Event::Locked, Event::GameOver]);
        assert_eq!(game.state(), GameState::GameOver);
    }

    #[test]
    fn nothing_moves_once_the_game_is_over() {
        let mut game = about_to_be_over();
        tick_until_locked(&mut game);
        assert_eq!(game.state(), GameState::GameOver);

        assert!(game.apply(Action::Left).is_empty());
        assert!(game.tick(interval() * 5).is_empty());
    }

    // --- sac de pièces ---

    #[test]
    fn the_same_seed_gives_the_same_game() {
        let mut a = Game::new(7);
        let mut b = Game::new(7);
        for _ in 0..5 {
            tick_until_locked(&mut a);
            tick_until_locked(&mut b);
            assert_eq!(a.current_piece(), b.current_piece());
            assert_eq!(a.board(), b.board());
        }
    }

    #[test]
    fn rotation_in_open_space_happens_in_place() {
        let mut game = t_game();

        assert_eq!(game.apply(Action::RotateCw), vec![Event::Rotated]);
        assert_eq!(game.current_piece().rotation(), Rotation::R1);
        assert_eq!(position(&game), SPAWN);

        assert_eq!(game.apply(Action::RotateCcw), vec![Event::Rotated]);
        assert_eq!(game.current_piece().rotation(), Rotation::R0);
    }

    #[test]
    fn a_rotation_blocked_by_the_wall_is_kicked_away_from_it() {
        let mut game = t_game();
        // T vertical collé au mur gauche : sa boîte dépasse (x = -1), mais ses cellules sont dans le plateau.
        game.current_piece = Piece::new(Tetromino::T, Position { x: -1, y: 5 }).rotated_cw();

        // Tourné à l'horizontale, il sortirait par la gauche : le décalage (1, 0) le sauve.
        assert_eq!(game.apply(Action::RotateCw), vec![Event::Rotated]);
        assert_eq!(game.current_piece().rotation(), Rotation::R2);
        assert_eq!(position(&game), Position { x: 0, y: 5 });
    }

    #[test]
    fn a_rotation_that_fits_nowhere_does_nothing() {
        let mut game = t_game();
        // Deux murs de blocs (colonnes 0-3 et 5-9, lignes 12 à 19) laissent un puits à la colonne 4.
        for col in (0..10).filter(|&c| c != 4) {
            for y in [12, 16] {
                let wall = Piece::new(Tetromino::I, Position { x: col - 2, y }).rotated_cw();
                game.board.lock(&wall);
            }
        }
        // Un I vertical dans le puits ne peut pas se coucher, même avec les 4 décalages.
        game.current_piece = Piece::new(Tetromino::I, Position { x: 2, y: 14 }).rotated_cw();
        let before = *game.current_piece();

        assert!(game.apply(Action::RotateCw).is_empty());
        assert_eq!(*game.current_piece(), before);
    }

    #[test]
    fn soft_drop_scores_one_point_per_cell_but_gravity_does_not() {
        let mut game = t_game();
        for _ in 0..3 {
            game.apply(Action::SoftDrop);
        }
        assert_eq!(game.score().points(), 3);

        game.tick(interval());
        assert_eq!(game.score().points(), 3);
    }

    #[test]
    fn the_tenth_line_raises_the_level() {
        let mut game = t_game();
        for _ in 0..3 {
            game.score.add_lines(3); // 9 lignes au total
        }
        leave_room_for_t_on_bottom_row(&mut game);

        let events = tick_until_locked(&mut game);

        assert_eq!(
            events,
            vec![Event::Locked, Event::LinesCleared(1), Event::LevelUp(2)]
        );
        assert_eq!(game.score().level(), 2);
    }

    #[test]
    fn hard_drop_locks_the_piece_at_the_bottom_and_scores_two_points_per_cell() {
        let mut game = t_game();

        let events = game.apply(Action::HardDrop);

        assert_eq!(events, vec![Event::Moved, Event::Locked]);
        assert!(game.board().get(4, 18).is_some());
        assert!(game.board().get(4, 19).is_some());
        assert_eq!(game.score().points(), 36); // 18 cases × 2
        assert_eq!(position(&game), SPAWN);
    }

    #[test]
    fn hard_drop_on_the_floor_just_locks() {
        let mut game = t_game();
        game.current_piece = Piece::new(Tetromino::T, Position { x: 3, y: 18 });

        assert_eq!(game.apply(Action::HardDrop), vec![Event::Locked]);
        assert_eq!(game.score().points(), 0);
    }

    #[test]
    fn hard_drop_can_clear_a_line() {
        let mut game = t_game();
        leave_room_for_t_on_bottom_row(&mut game);

        let events = game.apply(Action::HardDrop);

        assert_eq!(
            events,
            vec![Event::Moved, Event::Locked, Event::LinesCleared(1)]
        );
        assert_eq!(game.score().points(), 136); // 36 pour la chute + 100 pour la ligne
    }

    #[test]
    fn hard_drop_restarts_the_gravity_timer() {
        let mut game = t_game();
        game.tick(interval() / 2);

        game.apply(Action::HardDrop);

        // Sans remise à zéro, ce demi-intervalle ferait descendre la nouvelle pièce aussitôt.
        assert!(game.tick(interval() / 2).is_empty());
    }

    #[test]
    fn a_grounded_piece_waits_for_the_lock_delay() {
        let mut game = t_game();
        game.current_piece = Piece::new(Tetromino::T, Position { x: 3, y: 18 });
        let half = LOCK_DELAY / 2;

        assert!(game.tick(half).is_empty()); // la pièce est posée : le compte à rebours démarre
        assert!(game.tick(half).is_empty()); // 250 ms
        assert_eq!(game.tick(half), vec![Event::Locked]); // 500 ms
    }

    #[test]
    fn sliding_off_a_ledge_cancels_the_lock() {
        let mut game = t_game();
        // Une plateforme sur la ligne 19, colonnes 0 à 3, et un T posé dessus.
        game.board
            .lock(&Piece::new(Tetromino::I, Position { x: 0, y: 18 }));
        game.current_piece = Piece::new(Tetromino::T, Position { x: 2, y: 17 });

        assert!(game.tick(LOCK_DELAY / 2).is_empty()); // le compte à rebours démarre
        game.apply(Action::Right);
        game.apply(Action::Right); // le T dépasse de la plateforme : il n'est plus posé

        assert!(game.tick(LOCK_DELAY).is_empty()); // pas de verrouillage malgré le temps écoulé
        assert_eq!(game.current_piece().position().y, 17);
    }

    #[test]
    fn moving_a_grounded_piece_restarts_the_lock_timer() {
        let mut game = t_game();
        game.current_piece = Piece::new(Tetromino::T, Position { x: 3, y: 18 });
        let half = LOCK_DELAY / 2;

        game.tick(half); // démarre
        game.tick(half); // 250 ms
        game.apply(Action::Left); // remise à zéro

        assert!(game.tick(half).is_empty()); // 250 ms de nouveau, pas encore 500
        assert_eq!(game.tick(half), vec![Event::Locked]);
    }

    #[test]
    fn the_lock_timer_cannot_be_reset_forever() {
        let mut game = t_game();
        game.current_piece = Piece::new(Tetromino::T, Position { x: 3, y: 18 });
        let half = LOCK_DELAY / 2;
        game.tick(Duration::from_millis(1)); // le compte à rebours démarre

        for i in 0..MAX_LOCK_RESETS {
            game.apply(if i % 2 == 0 {
                Action::Left
            } else {
                Action::Right
            });
        }
        assert!(game.tick(half).is_empty()); // 250 ms

        // Les remises à zéro sont épuisées : ces déplacements n'y changent plus rien.
        for i in 0..5 {
            game.apply(if i % 2 == 0 {
                Action::Left
            } else {
                Action::Right
            });
        }
        assert_eq!(game.tick(half), vec![Event::Locked]);
    }

    #[test]
    fn a_new_game_announces_the_second_piece_of_the_bag() {
        let mut bag = Bag::new(7);
        let game = Game::new(7);

        assert_eq!(game.current_piece().kind(), bag.draw());
        assert_eq!(game.next_piece(), bag.draw());
    }

    #[test]
    fn the_announced_piece_is_the_one_that_spawns() {
        let mut game = Game::new(7);
        let announced = game.next_piece();

        game.apply(Action::HardDrop);

        assert_eq!(game.current_piece().kind(), announced);
    }

    #[test]
    fn the_piece_announced_after_a_spawn_is_the_next_one_in_the_bag() {
        let mut bag = Bag::new(7);
        let mut game = Game::new(7);
        bag.draw();
        bag.draw(); // la courante et l'annoncée du début

        game.apply(Action::HardDrop);

        assert_eq!(game.next_piece(), bag.draw());
    }

    #[test]
    fn pause_freezes_the_game() {
        let mut game = t_game();
        assert_eq!(game.apply(Action::TogglePause), vec![Event::Paused]);
        assert_eq!(game.state(), GameState::Paused);

        assert!(game.apply(Action::Left).is_empty());
        assert!(game.apply(Action::HardDrop).is_empty());
        assert!(game.tick(interval() * 5).is_empty());
        assert_eq!(position(&game), SPAWN);
    }

    #[test]
    fn resuming_continues_where_the_game_left_off() {
        let mut game = t_game();
        game.tick(interval() / 2);
        game.apply(Action::TogglePause);
        game.tick(interval() * 10); // le temps passe pendant la pause : il est ignoré

        assert_eq!(game.apply(Action::TogglePause), vec![Event::Resumed]);
        assert_eq!(game.state(), GameState::Playing);

        assert!(game.tick(interval() / 4).is_empty()); // 0,75 s au total : pas encore
        assert_eq!(game.tick(interval() / 2), vec![Event::Moved]); // 1,25 s : une descente
    }

    #[test]
    fn a_finished_game_cannot_be_paused() {
        let mut game = about_to_be_over();
        tick_until_locked(&mut game);

        assert!(game.apply(Action::TogglePause).is_empty());
        assert_eq!(game.state(), GameState::GameOver);
    }
}
