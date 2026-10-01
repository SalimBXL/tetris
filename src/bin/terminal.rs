use std::io::{self, Write, stdout};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};

use tetris::engine::board::{BOARD_HEIGHT, BOARD_WIDTH};
use tetris::engine::game::{Action, Game, GameState};
use tetris::engine::piece::{Piece, Position, Tetromino};

/// Durée maximale d'attente d'une touche avant de redessiner (environ 60 images par seconde).
const FRAME: Duration = Duration::from_millis(16);

enum Input {
    Quit,
    Play(Action),
}

fn main() -> io::Result<()> {
    let mut out = stdout();
    terminal::enable_raw_mode()?;
    execute!(out, EnterAlternateScreen, Hide, Clear(ClearType::All))?;

    let result = run(&mut out);

    // On restaure toujours le terminal, même si `run` a échoué.
    execute!(out, Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    result
}

/// Traduit un événement du terminal en entrée de jeu, si c'en est une.
fn read_input(event: event::Event) -> Option<Input> {
    let event::Event::Key(key) = event else {
        return None; // redimensionnement, souris, etc.
    };
    if key.kind == KeyEventKind::Release {
        return None;
    }

    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::Quit),
        KeyCode::Char('q') | KeyCode::Esc => Some(Input::Quit),
        KeyCode::Left | KeyCode::Char('a') => Some(Input::Play(Action::Left)),
        KeyCode::Right | KeyCode::Char('d') => Some(Input::Play(Action::Right)),
        KeyCode::Down | KeyCode::Char('s') => Some(Input::Play(Action::SoftDrop)),
        KeyCode::Up | KeyCode::Char('x') => Some(Input::Play(Action::RotateCw)),
        KeyCode::Char('z') => Some(Input::Play(Action::RotateCcw)),
        KeyCode::Char(' ') => Some(Input::Play(Action::HardDrop)),
        KeyCode::Char('p') => Some(Input::Play(Action::TogglePause)),
        _ => None,
    }
}

fn run(out: &mut impl Write) -> io::Result<()> {
    let mut game = Game::new(seed());
    let mut last_frame = Instant::now();

    loop {
        render(out, &game)?;

        // Entrées : on attend au plus une image, puis on vide ce qui est déjà arrivé.
        let mut pending = event::poll(FRAME)?;
        while pending {
            match read_input(event::read()?) {
                Some(Input::Quit) => return Ok(()),
                Some(Input::Play(action)) => {
                    game.apply(action);
                }
                None => {}
            }
            pending = event::poll(Duration::ZERO)?;
        }

        // Temps : c'est ici, et seulement ici, qu'on lit l'horloge.
        let now = Instant::now();
        game.tick(now - last_frame);
        last_frame = now;
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn color(kind: Tetromino) -> Color {
    match kind {
        Tetromino::I => Color::Cyan,
        Tetromino::O => Color::Yellow,
        Tetromino::T => Color::Magenta,
        Tetromino::S => Color::Green,
        Tetromino::Z => Color::Red,
        Tetromino::J => Color::Blue,
        Tetromino::L => Color::DarkYellow,
    }
}

fn render(out: &mut impl Write, game: &Game) -> io::Result<()> {
    // 1. On compose une grille à afficher : plateau + pièce courante par-dessus.
    let mut grid: Vec<Vec<Option<Tetromino>>> = (0..BOARD_HEIGHT)
        .map(|y| (0..BOARD_WIDTH).map(|x| game.board().get(x, y)).collect())
        .collect();

    if game.state() != GameState::GameOver {
        let piece = game.current_piece();
        for (x, y) in piece.cells() {
            if (0..BOARD_WIDTH as i32).contains(&x) && (0..BOARD_HEIGHT as i32).contains(&y) {
                grid[y as usize][x as usize] = Some(piece.kind());
            }
        }
    }

    // 2. On dessine. En mode « raw », une fin de ligne s'écrit "\r\n".
    let score = game.score();
    queue!(out, MoveTo(0, 0))?;
    queue!(
        out,
        Print(format!("┌{}┐", "──".repeat(BOARD_WIDTH))),
        Print("\r\n")
    )?;

    let next_kind = game.next_piece();
    let preview = Piece::new(next_kind, Position { x: 0, y: 0 }).cells();

    let status = match game.state() {
        GameState::Playing => "",
        GameState::Paused => "PAUSE",
        GameState::GameOver => "GAME OVER",
    };

    for (y, row) in grid.iter().enumerate() {
        queue!(out, Print("│"))?;
        for cell in row {
            match cell {
                Some(kind) => {
                    queue!(
                        out,
                        SetForegroundColor(color(*kind)),
                        Print("██"),
                        ResetColor
                    )?;
                }
                None => {
                    queue!(out, Print(" ."))?;
                }
            }
        }
        queue!(out, Print("│"))?;

        let side = match y {
            1 => format!("  Score  {}", score.points()),
            2 => format!("  Lignes {}", score.lines()),
            3 => format!("  Niveau {}", score.level()),
            5 => "  Suivante".to_string(),
            9 if !status.is_empty() => format!("  {status}"),
            _ => String::new(),
        };
        queue!(out, Print(side))?;

        // Aperçu de la pièce suivante, sur deux lignes.
        if (6..=7).contains(&y) {
            queue!(out, Print("  "))?;
            for col in 0..4 {
                if preview.contains(&(col, y as i32 - 6)) {
                    queue!(
                        out,
                        SetForegroundColor(color(next_kind)),
                        Print("██"),
                        ResetColor
                    )?;
                } else {
                    queue!(out, Print("  "))?;
                }
            }
        }
        queue!(out, Clear(ClearType::UntilNewLine), Print("\r\n"))?;
    }

    queue!(
        out,
        Print(format!("└{}┘", "──".repeat(BOARD_WIDTH))),
        Print("\r\n")
    )?;
    queue!(
        out,
        Print(
            "←/→ ou a/d : déplacer   ↓/s : descendre   espace : chute   ↑/x : tourner   z : tourner (inverse)   p : pause"
        ),
        Clear(ClearType::UntilNewLine),
        Print("\r\n"),
        Print("q ou Échap : quitter"),
        Clear(ClearType::UntilNewLine),
    )?;

    // 3. Tout part d'un coup : un seul `flush` par image évite le scintillement.
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> event::Event {
        event::Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn arrows_and_letters_map_to_actions() {
        let none = KeyModifiers::NONE;
        assert!(matches!(
            read_input(press(KeyCode::Left, none)),
            Some(Input::Play(Action::Left))
        ));
        assert!(matches!(
            read_input(press(KeyCode::Char('d'), none)),
            Some(Input::Play(Action::Right))
        ));
        assert!(matches!(
            read_input(press(KeyCode::Up, none)),
            Some(Input::Play(Action::RotateCw))
        ));
        assert!(matches!(
            read_input(press(KeyCode::Char('z'), none)),
            Some(Input::Play(Action::RotateCcw))
        ));
    }

    #[test]
    fn quit_keys_quit() {
        assert!(matches!(
            read_input(press(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Input::Quit)
        ));
        assert!(matches!(
            read_input(press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Input::Quit)
        ));
    }

    #[test]
    fn key_release_and_other_keys_are_ignored() {
        let release = event::Event::Key(KeyEvent::new_with_kind(
            KeyCode::Left,
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        assert!(read_input(release).is_none());
        assert!(read_input(press(KeyCode::Char('w'), KeyModifiers::NONE)).is_none());
    }

    #[test]
    fn space_is_a_hard_drop() {
        assert!(matches!(
            read_input(press(KeyCode::Char(' '), KeyModifiers::NONE)),
            Some(Input::Play(Action::HardDrop))
        ));
    }

    #[test]
    fn p_toggles_the_pause() {
        assert!(matches!(
            read_input(press(KeyCode::Char('p'), KeyModifiers::NONE)),
            Some(Input::Play(Action::TogglePause))
        ));
    }
}
