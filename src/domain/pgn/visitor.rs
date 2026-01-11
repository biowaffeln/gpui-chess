//! PGN visitor implementation for building game models.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read as IoRead};
use std::ops::ControlFlow;
use std::path::Path;

use pgn_reader::{RawTag, Reader, SanPlus, Skip, Visitor};
use shakmaty::{Chess, Position};

use crate::domain::MoveTree;

use super::{PgnGame, PgnHeaders};

/// State passed through the tags phase
struct TagsState {
    headers: HashMap<String, String>,
}

/// State passed through the movetext phase
struct MovetextState {
    headers: HashMap<String, String>,
    tree: MoveTree,
    position: Chess,
    success: bool,
}

/// Visitor that builds a PgnGame from parsed PGN data.
///
/// Currently handles:
/// - All headers
/// - Mainline moves (no variations yet)
///
/// TODO: Add support for variations (RAV), comments, and NAGs.
#[derive(Default)]
struct GameBuilder;

impl Visitor for GameBuilder {
    type Tags = TagsState;
    type Movetext = MovetextState;
    type Output = Option<PgnGame>;

    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(TagsState {
            headers: HashMap::new(),
        })
    }

    fn tag(
        &mut self,
        tags: &mut Self::Tags,
        name: &[u8],
        value: RawTag<'_>,
    ) -> ControlFlow<Self::Output> {
        if let (Ok(k), Ok(v)) = (
            std::str::from_utf8(name),
            std::str::from_utf8(value.as_bytes()),
        ) {
            tags.headers.insert(k.to_string(), v.to_string());
        }
        ControlFlow::Continue(())
    }

    fn begin_movetext(&mut self, tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> {
        ControlFlow::Continue(MovetextState {
            headers: tags.headers,
            tree: MoveTree::new(),
            position: Chess::default(),
            success: true,
        })
    }

    fn san(&mut self, movetext: &mut Self::Movetext, san: SanPlus) -> ControlFlow<Self::Output> {
        if !movetext.success {
            return ControlFlow::Continue(());
        }

        match san.san.to_move(&movetext.position) {
            Ok(m) => {
                let san_string = san.to_string();
                let new_position = movetext.position.clone().play(m).expect("legal move");
                movetext.tree.add_move(new_position.clone(), san_string);
                movetext.position = new_position;
            }
            Err(_) => {
                movetext.success = false;
            }
        }
        ControlFlow::Continue(())
    }

    fn begin_variation(
        &mut self,
        _movetext: &mut Self::Movetext,
    ) -> ControlFlow<Self::Output, Skip> {
        // Skip variations for now (Phase 1)
        ControlFlow::Continue(Skip(true))
    }

    fn end_game(&mut self, movetext: Self::Movetext) -> Self::Output {
        if movetext.success {
            let headers = PgnHeaders {
                tags: movetext.headers,
            };
            Some(PgnGame::new(headers, movetext.tree))
        } else {
            None
        }
    }
}

/// Parse all games from a PGN string.
///
/// Returns an iterator over successfully parsed games.
/// Games with invalid moves are skipped.
pub fn parse_pgn(pgn: &str) -> impl Iterator<Item = PgnGame> + '_ {
    let mut reader = Reader::new(io::Cursor::new(pgn));
    let mut visitor = GameBuilder;

    std::iter::from_fn(move || loop {
        match reader.read_game(&mut visitor) {
            Ok(Some(game)) => {
                if let Some(g) = game {
                    return Some(g);
                }
                // Game failed to parse, try next
            }
            Ok(None) => return None, // End of input
            Err(_) => return None,   // IO error, stop
        }
    })
}

/// Load and parse games from a PGN file.
///
/// Returns all successfully parsed games.
/// Handles non-UTF-8 content (e.g., Latin-1 encoded player names) by reading as bytes.
pub fn load_pgn_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<PgnGame>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;

    Ok(parse_pgn_bytes(&bytes).collect())
}

/// Parse all games from PGN bytes.
///
/// Returns an iterator over successfully parsed games.
/// Games with invalid moves are skipped.
pub fn parse_pgn_bytes(bytes: &[u8]) -> impl Iterator<Item = PgnGame> + '_ {
    let mut reader = Reader::new(io::Cursor::new(bytes));
    let mut visitor = GameBuilder;

    std::iter::from_fn(move || loop {
        match reader.read_game(&mut visitor) {
            Ok(Some(game)) => {
                if let Some(g) = game {
                    return Some(g);
                }
            }
            Ok(None) => return None,
            Err(_) => return None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_GAME: &str = r#"[Event "Test Game"]
[Site "Internet"]
[Date "2024.01.15"]
[White "Player1"]
[Black "Player2"]
[Result "1-0"]
[WhiteElo "2000"]
[BlackElo "1900"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 1-0
"#;

    const TWO_GAMES: &str = r#"[Event "Game 1"]
[White "Alice"]
[Black "Bob"]
[Result "1-0"]

1. e4 e5 2. Nf3 1-0

[Event "Game 2"]
[White "Carol"]
[Black "Dave"]
[Result "0-1"]

1. d4 d5 2. c4 0-1
"#;

    #[test]
    fn test_parse_simple_game() {
        let games: Vec<_> = parse_pgn(SIMPLE_GAME).collect();
        assert_eq!(games.len(), 1);

        let game = &games[0];

        // Check headers
        assert_eq!(game.headers.event(), Some("Test Game"));
        assert_eq!(game.headers.white(), Some("Player1"));
        assert_eq!(game.headers.black(), Some("Player2"));
        assert_eq!(game.headers.result(), Some("1-0"));
        assert_eq!(game.headers.white_elo(), Some(2000));
        assert_eq!(game.headers.black_elo(), Some(1900));

        // Check moves - main line should have 6 half-moves + root
        let main_line = game.moves.main_line();
        assert_eq!(main_line.len(), 7); // root + 6 moves
    }

    #[test]
    fn test_parse_multiple_games() {
        let games: Vec<_> = parse_pgn(TWO_GAMES).collect();
        assert_eq!(games.len(), 2);

        assert_eq!(games[0].headers.white(), Some("Alice"));
        assert_eq!(games[1].headers.white(), Some("Carol"));
    }

    #[test]
    fn test_move_tree_structure() {
        let games: Vec<_> = parse_pgn(SIMPLE_GAME).collect();
        let game = &games[0];

        // Navigate through moves
        let main_line = game.moves.main_line();

        // Check first move (e4)
        let e4_node = game.moves.get(main_line[1]).unwrap();
        assert_eq!(e4_node.san.as_deref(), Some("e4"));

        // Check second move (e5)
        let e5_node = game.moves.get(main_line[2]).unwrap();
        assert_eq!(e5_node.san.as_deref(), Some("e5"));
    }

    #[test]
    fn test_invalid_move_skips_game() {
        let bad_pgn = r#"[Event "Bad Game"]
[White "X"]
[Black "Y"]
[Result "*"]

1. e4 e5 2. Qxe5 *
"#;
        // Qxe5 is not legal in this position
        let games: Vec<_> = parse_pgn(bad_pgn).collect();
        assert_eq!(games.len(), 0);
    }

    #[test]
    fn test_empty_game() {
        let empty = r#"[Event "Empty"]
[White "A"]
[Black "B"]
[Result "*"]

*
"#;
        let games: Vec<_> = parse_pgn(empty).collect();
        assert_eq!(games.len(), 1);

        // Should just have root node
        let main_line = games[0].moves.main_line();
        assert_eq!(main_line.len(), 1);
    }
}
