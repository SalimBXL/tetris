use std::time::Duration;

use macroquad::prelude::*;

use tetris::engine::board::{BOARD_HEIGHT, BOARD_WIDTH};
use tetris::engine::game::{Action, Game, GameState};
use tetris::engine::piece::{Piece, Position, Tetromino};
use tetris::seed;

// --- Mise en page ---

const CELL: f32 = 30.0;
const BOARD_X: f32 = 30.0;
const BOARD_Y: f32 = 30.0;
const BOARD_W: f32 = CELL * BOARD_WIDTH as f32;
const BOARD_H: f32 = CELL * BOARD_HEIGHT as f32;
const PANEL_X: f32 = BOARD_X + BOARD_W + 30.0;

/// Au-delà, on considère que la fenêtre a été figée (déplacée, mise en veille...) et on ne rattrape pas.
const MAX_FRAME_TIME: f32 = 0.1;

fn window_conf() -> Conf {
    Conf {
        window_title: "Tetris".to_owned(),
        window_width: 600,
        window_height: (BOARD_Y * 2.0 + BOARD_H) as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new(seed::from_clock());
    let mut inputs = Inputs::new();

    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        if game.state() == GameState::GameOver && is_key_pressed(KeyCode::R) {
            game = Game::new(seed::from_clock());
        }

        // Temps : c'est ici, et seulement ici, qu'on lit l'horloge.
        let dt = get_frame_time().min(MAX_FRAME_TIME);

        for action in inputs.actions(dt) {
            game.apply(action);
        }
        game.tick(Duration::from_secs_f32(dt));

        draw(&game);
        next_frame().await;
    }
}

// --- Entrées ---

/// Répétition automatique d'une touche maintenue : un déclenchement à l'appui,
/// une pause, puis un déclenchement à intervalle régulier.
struct KeyRepeat {
    /// Durée depuis l'appui, `None` si la touche n'est pas enfoncée.
    held: Option<f32>,
    /// Durée à partir de laquelle le prochain déclenchement a lieu.
    next_at: f32,
}

impl KeyRepeat {
    const DELAY: f32 = 0.17;
    const INTERVAL: f32 = 0.05;

    fn new() -> Self {
        Self {
            held: None,
            next_at: 0.0,
        }
    }

    /// Renvoie `true` si l'action doit être déclenchée à cette image.
    fn update(&mut self, down: bool, dt: f32) -> bool {
        if !down {
            self.held = None;
            return false;
        }
        match self.held {
            None => {
                self.held = Some(0.0);
                self.next_at = Self::DELAY;
                true
            }
            Some(held) => {
                let held = held + dt;
                self.held = Some(held);
                if held >= self.next_at {
                    self.next_at += Self::INTERVAL;
                    true
                } else {
                    false
                }
            }
        }
    }
}

struct Inputs {
    left: KeyRepeat,
    right: KeyRepeat,
    down: KeyRepeat,
}

impl Inputs {
    fn new() -> Self {
        Self {
            left: KeyRepeat::new(),
            right: KeyRepeat::new(),
            down: KeyRepeat::new(),
        }
    }

    /// Les actions du joueur pour cette image.
    fn actions(&mut self, dt: f32) -> Vec<Action> {
        let mut actions = Vec::new();

        if self
            .left
            .update(is_key_down(KeyCode::Left) || is_key_down(KeyCode::A), dt)
        {
            actions.push(Action::Left);
        }
        if self
            .right
            .update(is_key_down(KeyCode::Right) || is_key_down(KeyCode::D), dt)
        {
            actions.push(Action::Right);
        }
        if self
            .down
            .update(is_key_down(KeyCode::Down) || is_key_down(KeyCode::S), dt)
        {
            actions.push(Action::SoftDrop);
        }
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::X) {
            actions.push(Action::RotateCw);
        }
        if is_key_pressed(KeyCode::Z) {
            actions.push(Action::RotateCcw);
        }
        if is_key_pressed(KeyCode::Space) {
            actions.push(Action::HardDrop);
        }
        if is_key_pressed(KeyCode::P) {
            actions.push(Action::TogglePause);
        }
        actions
    }
}

// --- Rendu ---

fn color(kind: Tetromino) -> Color {
    match kind {
        Tetromino::I => Color::from_rgba(0, 220, 230, 255),
        Tetromino::O => Color::from_rgba(240, 220, 0, 255),
        Tetromino::T => Color::from_rgba(170, 70, 220, 255),
        Tetromino::S => Color::from_rgba(60, 200, 70, 255),
        Tetromino::Z => Color::from_rgba(230, 60, 60, 255),
        Tetromino::J => Color::from_rgba(60, 90, 230, 255),
        Tetromino::L => Color::from_rgba(240, 150, 30, 255),
    }
}

fn draw(game: &Game) {
    clear_background(Color::from_rgba(18, 18, 24, 255));
    draw_board(game);
    draw_panel(game);
    draw_overlay(game);
}

fn draw_block(px: f32, py: f32, kind: Tetromino) {
    draw_rectangle(px + 1.0, py + 1.0, CELL - 2.0, CELL - 2.0, color(kind));
}

fn draw_board(game: &Game) {
    draw_rectangle(
        BOARD_X,
        BOARD_Y,
        BOARD_W,
        BOARD_H,
        Color::from_rgba(28, 28, 38, 255),
    );

    let grid = Color::from_rgba(45, 45, 60, 255);
    for x in 1..BOARD_WIDTH {
        let px = BOARD_X + x as f32 * CELL;
        draw_line(px, BOARD_Y, px, BOARD_Y + BOARD_H, 1.0, grid);
    }
    for y in 1..BOARD_HEIGHT {
        let py = BOARD_Y + y as f32 * CELL;
        draw_line(BOARD_X, py, BOARD_X + BOARD_W, py, 1.0, grid);
    }

    // Blocs verrouillés.
    for y in 0..BOARD_HEIGHT {
        for x in 0..BOARD_WIDTH {
            if let Some(kind) = game.board().get(x, y) {
                draw_block(BOARD_X + x as f32 * CELL, BOARD_Y + y as f32 * CELL, kind);
            }
        }
    }

    // Pièce courante, par-dessus.
    if game.state() != GameState::GameOver {
        let piece = game.current_piece();
        for (x, y) in piece.cells() {
            draw_block(
                BOARD_X + x as f32 * CELL,
                BOARD_Y + y as f32 * CELL,
                piece.kind(),
            );
        }
    }

    draw_rectangle_lines(BOARD_X, BOARD_Y, BOARD_W, BOARD_H, 2.0, LIGHTGRAY);
}

fn draw_panel(game: &Game) {
    let score = game.score();
    draw_text(
        format!("Score  {}", score.points()),
        PANEL_X,
        60.0,
        28.0,
        WHITE,
    );
    draw_text(
        format!("Lignes {}", score.lines()),
        PANEL_X,
        95.0,
        28.0,
        WHITE,
    );
    draw_text(
        format!("Niveau {}", score.level()),
        PANEL_X,
        130.0,
        28.0,
        WHITE,
    );

    draw_text("Suivante", PANEL_X, 190.0, 28.0, WHITE);
    let next = Piece::new(game.next_piece(), Position { x: 0, y: 0 });
    for (x, y) in next.cells() {
        draw_block(
            PANEL_X + x as f32 * CELL,
            210.0 + y as f32 * CELL,
            next.kind(),
        );
    }

    let help = [
        "Gauche/Droite : deplacer",
        "Bas : descendre",
        "Espace : chute",
        "Haut / X : tourner",
        "Z : tourner (inverse)",
        "P : pause",
        "Echap : quitter",
    ];
    for (i, line) in help.iter().enumerate() {
        draw_text(line, PANEL_X, 420.0 + i as f32 * 24.0, 20.0, GRAY);
    }
}

fn draw_centered(text: &str, y: f32, size: u16, color: Color) {
    let dims = measure_text(text, None, size, 1.0);
    draw_text(
        text,
        BOARD_X + (BOARD_W - dims.width) / 2.0,
        y,
        size as f32,
        color,
    );
}

fn draw_overlay(game: &Game) {
    let (message, hint) = match game.state() {
        GameState::Playing => return,
        GameState::Paused => ("PAUSE", "P : reprendre"),
        GameState::GameOver => ("GAME OVER", "R : rejouer"),
    };

    draw_rectangle(
        BOARD_X,
        BOARD_Y,
        BOARD_W,
        BOARD_H,
        Color::new(0.0, 0.0, 0.0, 0.6),
    );
    draw_centered(message, BOARD_Y + BOARD_H / 2.0, 40, WHITE);
    draw_centered(hint, BOARD_Y + BOARD_H / 2.0 + 35.0, 24, LIGHTGRAY);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_fires_on_press_then_waits_for_the_delay() {
        let mut key = KeyRepeat::new();
        assert!(key.update(true, 0.016)); // l'appui déclenche tout de suite
        assert!(!key.update(true, 0.1)); // 0,1 s : pas encore
        assert!(key.update(true, 0.1)); // 0,2 s : la répétition démarre
        assert!(!key.update(true, 0.01)); // 0,21 s : trop tôt pour la suivante
        assert!(key.update(true, 0.02)); // 0,23 s : la suivante
    }

    #[test]
    fn releasing_the_key_resets_the_repeat() {
        let mut key = KeyRepeat::new();
        key.update(true, 0.0);
        key.update(true, 0.3);

        assert!(!key.update(false, 0.016));
        assert!(key.update(true, 0.016)); // un nouvel appui déclenche aussitôt
    }
}
