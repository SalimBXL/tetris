# Tetris en Rust

[![CI](https://github.com/SalimBXL/tetris/actions/workflows/ci.yml/badge.svg)](https://github.com/SalimBXL/tetris/actions/workflows/ci.yml)

Un Tetris jouable dans le terminal ou dans une fenêtre, écrit en Rust pour apprendre le langage.

Le projet est construit autour d'un **moteur de jeu indépendant de l'affichage** : toutes les règles vivent dans une bibliothèque entièrement testable, et les deux interfaces (terminal et graphique) se contentent de lire son état et de lui envoyer des actions.

## Fonctionnalités

- Plateau de 10 × 20 et les sept tétrominos, distribués par « sac de 7 » (chaque pièce apparaît une fois par sac, dans un ordre mélangé)
- Déplacements, rotations (avec décalages de secours près des murs), descente douce et chute instantanée
- Gravité qui accélère avec le niveau
- Verrouillage différé : une pièce posée peut encore glisser pendant 0,5 s
- Score, nombre de lignes et niveaux
- Aperçu de la pièce suivante, pause, relance après un game over (interface graphique)
- Parties reproductibles : la même graine donne la même suite de pièces
- Deux interfaces : terminal ([crossterm](https://crates.io/crates/crossterm)) et graphique ([macroquad](https://crates.io/crates/macroquad))

## Prérequis

- [Rust](https://www.rust-lang.org/tools/install) (toolchain stable, installée avec `rustup`)
- Sous Linux, pour la version graphique : les bibliothèques X11 et OpenGL habituelles de votre distribution

La CI compile et teste le projet sous Linux, macOS et Windows.

## Lancer le jeu

```bash
git clone https://github.com/SalimBXL/tetris.git
cd tetris

# Interface graphique
cargo run --release --bin gui

# Interface terminal
cargo run --release --bin terminal
```

## Commandes

| Action                         | Touches                | Interfaces              |
| ------------------------------ | ---------------------- | ----------------------- |
| Déplacer à gauche / à droite   | `←` `→` ou `A` `D`     | graphique, terminal     |
| Descente douce                 | `↓` ou `S`             | graphique, terminal     |
| Rotation horaire               | `↑` ou `X`             | graphique, terminal     |
| Rotation anti-horaire          | `Z`                    | graphique, terminal     |
| Chute instantanée              | `Espace`               | graphique, terminal     |
| Pause / reprise                | `P`                    | graphique, terminal     |
| Rejouer (après un game over)   | `R`                    | graphique               |
| Quitter                        | `Échap`                | graphique, terminal     |
| Quitter                        | `Q` ou `Ctrl+C`        | terminal                |

Dans l'interface graphique, les touches de déplacement et de descente se répètent quand on les maintient.

## Règles implémentées

**Gravité.** La pièce descend d'une ligne à intervalle régulier : environ 1 s au niveau 1, de plus en plus vite ensuite, jusqu'au niveau 20 où la vitesse ne change plus.

**Verrouillage différé.** Quand la pièce repose sur le sol ou sur des blocs, un compte à rebours de 0,5 s démarre. Déplacer ou tourner la pièce le remet à zéro (15 fois au maximum par pièce), et glisser la pièce hors de son appui l'annule. La chute instantanée verrouille tout de suite.

**Rotations.** Si une rotation est bloquée, le jeu essaie la pièce décalée d'une case à gauche, à droite, puis vers le haut. Ce n'est pas le système officiel (SRS), mais c'est suffisant pour une bonne sensation de jeu.

**Score.**

| Lignes supprimées | Points (× niveau) |
| ----------------- | ----------------- |
| 1                 | 100               |
| 2                 | 300               |
| 3                 | 500               |
| 4                 | 800               |

La descente douce rapporte 1 point par case et la chute instantanée 2 points par case. Les points d'une suppression sont calculés avec le niveau d'avant ces lignes. On passe au niveau suivant toutes les 10 lignes.

## Architecture

```text
src/
├── lib.rs              # point d'entrée de la bibliothèque
├── seed.rs             # graine tirée de l'horloge système
├── engine/             # le moteur : aucune dépendance à l'affichage
│   ├── piece.rs        # tétrominos, rotations, table des formes
│   ├── board.rs        # plateau, collisions, lignes
│   ├── bag.rs          # sac de 7 pièces avec générateur à graine
│   ├── score.rs        # points, niveaux, vitesse de gravité
│   └── game.rs         # Game : actions, temps, événements
└── bin/
    ├── terminal.rs     # interface terminal
    └── gui.rs          # interface graphique
```

Le principe : le clavier ne déplace jamais un dessin. Une interface traduit les touches en `Action`, appelle `Game::apply` et `Game::tick(dt)`, puis dessine ce que le moteur expose. Le moteur ne lit jamais l'horloge et ne connaît ni le clavier ni l'écran, ce qui le rend déterministe et facile à tester.

Pour le détail des choix de conception, voir [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Développement

```bash
cargo test                                  # tests unitaires et tests de documentation
cargo fmt --all                             # formatage
cargo clippy --all-targets -- -D warnings   # analyse statique
cargo doc --no-deps --lib --open            # documentation de la bibliothèque
```

La CI GitHub (`.github/workflows/ci.yml`) exécute ces vérifications à chaque `push` sur la branche principale et à chaque *pull request*.

## Pistes d'évolution

- Pièce « hold » (mettre la pièce courante de côté)
- Sons, branchés sur les événements que renvoient déjà `Game::apply` et `Game::tick`
- Menu de départ
- Tables de rotation officielles (SRS) à la place des décalages simplifiés