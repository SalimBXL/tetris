# Architecture

Ce document explique comment le projet est découpé et pourquoi. Pour l'usage, voir le [README](../README.md).

## Principe directeur

> Le moteur de jeu ne sait rien de l'affichage, du clavier ni de l'horloge.

Tout le reste en découle :

```text
 Joueur ──► Interface (terminal / graphique)
                │  Action            ▲ lecture seule
                ▼                    │ (board, piece, score, state...)
            ┌─────────────────────────────┐
            │            Game             │
            │  apply(Action) -> Events    │
            │  tick(dt)      -> Events    │
            └──────┬──────────┬───────────┘
                   │          │
                 Board      Piece, Bag, Score
```

Une interface a trois tâches : traduire des entrées en `Action`, mesurer le temps écoulé pour appeler `tick(dt)`, et dessiner ce que `Game` expose en lecture. Elle ne décide jamais d'une règle.

## Modules

| Module | Responsabilité |
| --- | --- |
| `engine::piece` | `Tetromino`, `Rotation`, `Position`, `Piece`, et la table des formes (`SHAPES`) |
| `engine::board` | Grille des blocs verrouillés, collisions (`can_place`), verrouillage (`lock`), suppression des lignes (`clear_lines`) |
| `engine::bag` | Tirage des pièces par sac de 7, avec un générateur pseudo-aléatoire à graine (SplitMix64) |
| `engine::score` | Points, lignes, niveau, et durée de la gravité selon le niveau |
| `engine::game` | `Game` : orchestre tout le reste, expose `apply`, `tick` et les accesseurs en lecture |
| `seed` | Graine tirée de l'horloge système (seul endroit de la bibliothèque qui lit l'horloge) |
| `bin/terminal`, `bin/gui` | Les deux interfaces |

## Conventions de coordonnées

- `x` va vers la droite, `y` vers le bas, l'origine est en haut à gauche du plateau.
- Les coordonnées d'une **pièce** sont des `i32` : une pièce peut avoir temporairement un `x` négatif (sa boîte déborde du mur alors que ses cellules restent dans le plateau).
- Les index du **plateau** sont des `usize`. La conversion de l'un à l'autre se fait à un seul endroit, `Board::to_index`.
- `Piece::position` est le coin supérieur gauche de la **boîte englobante** de la pièce (3 × 3, ou 4 × 4 pour I et O), pas une de ses cellules. Chaque rotation est dessinée dans la même boîte, ce qui évite tout calcul de rotation : `SHAPES[pièce][rotation]` donne directement les 4 cellules.

## Le plateau ne contient pas la pièce qui tombe

`Board` ne stocke que les blocs déjà fixés. La pièce courante est un objet à part dans `Game`. Les collisions deviennent alors une simple question posée au plateau : « cette pièce peut-elle être placée ici ? » (`Board::can_place`).

## Le schéma « candidat »

Toute modification de la pièce suit le même chemin :

```text
pièce courante ──► moved_left() / rotated_cw() / ...   (renvoie une NOUVELLE pièce)
                         │
                   Board::can_place ?
                  ╱                ╲
                oui                non
                 │                  │
     la courante devient          rien ne change
          le candidat
```

`Piece` est un petit type `Copy` aux champs privés : on ne peut pas la modifier en place, seulement en obtenir une variante. Les méthodes sont marquées `#[must_use]` pour qu'oublier d'utiliser le résultat soit signalé par le compilateur.

## Le temps

`Game::tick(dt: Duration)` reçoit le temps écoulé ; le moteur ne lit jamais l'horloge. Conséquences :

- les tests contrôlent le temps sans jamais attendre ;
- si l'interface rate des images, `tick` rattrape la gravité proprement (le reliquat de temps n'est pas perdu) ;
- la pause gèle tout, sans cas particulier : `tick` ignore simplement le temps hors de l'état `Playing`.

Les interfaces mesurent le temps (`Instant` en terminal, `get_frame_time` en graphique) et le transmettent. Elles bornent les grosses valeurs (fenêtre figée, mise en veille) pour ne pas faire tomber une pièce d'un coup à la reprise.

## Verrouillage différé

L'état de verrouillage est un `Option<Duration>` dans `Game` :

- `None` : la pièce est en l'air ;
- `Some(t)` : la pièce repose sur un appui depuis `t`.

Deux mises à jour distinctes :

- après une action réussie du joueur, un compte à rebours déjà lancé repart de zéro (au plus 15 fois par pièce, pour empêcher de retarder le verrouillage indéfiniment) ;
- à la fin d'un `tick`, le compte à rebours démarre si la pièce vient de se poser, ou s'annule si elle n'est plus posée.

Quand le compte à rebours atteint 0,5 s, `lock_current_and_spawn` fixe la pièce, supprime les lignes, met à jour le score et fait apparaître la suivante. La chute instantanée appelle directement cette même fonction.

## Déterminisme

Une partie est entièrement déterminée par sa graine et la suite des `Action` / `tick` qu'elle reçoit. Le sac de pièces utilise un générateur maison à graine ; `seed::from_clock()` n'est appelé que par les interfaces, jamais par le moteur. Un test vérifie que deux parties lancées avec la même graine restent identiques.

## Événements

`apply` et `tick` renvoient un `Vec<Event>` (`Moved`, `Rotated`, `Locked`, `LinesCleared(n)`, `LevelUp(n)`, `Paused`, `Resumed`, `GameOver`). Les interfaces actuelles ne s'en servent pas, mais c'est le point d'accroche prévu pour des sons ou des effets visuels, sans toucher aux règles.

## Tests

La plupart des tests vivent à côté du code qu'ils vérifient (`#[cfg(test)]`) :

- `piece` : chaque forme a 4 cellules distinctes dans sa boîte, les rotations bouclent, les déplacements renvoient de nouvelles pièces ;
- `board` : collisions avec les murs et les blocs, verrouillage, suppression de lignes (des plateaux sont construits à partir de petits dessins en texte) ;
- `bag` : chaque groupe de 7 contient les 7 pièces, même graine donne même suite ;
- `score` : tarifs, niveaux, la gravité accélère et reste strictement positive ;
- `game` : déplacements, gravité, verrouillage différé, game over, chute instantanée, pause, aperçu de la pièce suivante ;
- `bin/gui` : la logique de répétition des touches, qui ne dépend pas de la fenêtre.

Les tests de `game` accèdent aux champs privés depuis leur module enfant pour préparer des situations précises (un plateau presque plein, une pièce posée au sol), sans exposer de méthode de modification publique.

## Ajouter une fonctionnalité

- **Une nouvelle commande** : ajouter une variante à `Action`, la traiter dans `Game::apply`, puis la brancher dans chaque interface.
- **Un nouveau calcul de score** : `engine::score` seulement.
- **Une nouvelle interface** : un nouveau binaire dans `src/bin/` qui utilise la bibliothèque ; rien à changer dans `engine/`.
- **Un son ou un effet** : lire les `Event` renvoyés par `apply` et `tick`.