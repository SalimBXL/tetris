//! Moteur de Tetris et utilitaires.
//!
//! Le module [`engine`] contient toutes les règles du jeu, sans aucune
//! dépendance à l'affichage ni à l'horloge. Une interface envoie des
//! actions et le temps écoulé, puis lit l'état du jeu pour le dessiner.
//!
//! # Exemple
//!
//! ```
//! use std::time::Duration;
//! use tetris::engine::game::{Action, Game, GameState};
//!
//! let mut game = Game::new(42);
//! game.apply(Action::Left);
//! game.tick(Duration::from_millis(16));
//! assert_eq!(game.state(), GameState::Playing);
//! ```

pub mod engine;
pub mod seed;
