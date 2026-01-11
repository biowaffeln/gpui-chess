//! PGN library model - manages loaded PGN games.

use crate::domain::pgn::PgnGame;

/// Summary info for a game in the library (for display in table)
#[derive(Clone, Debug)]
pub struct GameSummary {
    pub index: usize,
    pub white: String,
    pub white_elo: Option<u16>,
    pub black: String,
    pub black_elo: Option<u16>,
    pub result: String,
    pub date: String,
    pub event: String,
    pub moves_count: usize,
}

impl GameSummary {
    pub fn from_pgn_game(index: usize, game: &PgnGame) -> Self {
        let moves_count = game.moves.main_line().len().saturating_sub(1); // Exclude root
        Self {
            index,
            white: game.headers.white().unwrap_or("?").to_string(),
            white_elo: game.headers.white_elo(),
            black: game.headers.black().unwrap_or("?").to_string(),
            black_elo: game.headers.black_elo(),
            result: game.headers.result().unwrap_or("*").to_string(),
            date: game.headers.date().unwrap_or("????.??.??").to_string(),
            event: game.headers.event().unwrap_or("?").to_string(),
            moves_count,
        }
    }
}

/// The PGN library model - holds all loaded games
pub struct PgnLibraryModel {
    /// All loaded PGN games
    games: Vec<PgnGame>,
    /// Summaries for table display
    summaries: Vec<GameSummary>,
    /// Currently selected game index (if any)
    selected_index: Option<usize>,
}

impl PgnLibraryModel {
    pub fn new() -> Self {
        Self {
            games: Vec::new(),
            summaries: Vec::new(),
            selected_index: None,
        }
    }

    /// Load games from a PGN file, replacing any existing games
    pub fn load_from_file(&mut self, path: &std::path::Path) -> std::io::Result<usize> {
        let games = crate::domain::pgn::load_pgn_file(path)?;
        let count = games.len();
        
        self.summaries = games
            .iter()
            .enumerate()
            .map(|(i, g)| GameSummary::from_pgn_game(i, g))
            .collect();
        self.games = games;
        self.selected_index = None;
        
        Ok(count)
    }

    /// Load games from a PGN string
    pub fn load_from_string(&mut self, pgn: &str) -> usize {
        let games: Vec<_> = crate::domain::pgn::parse_pgn(pgn).collect();
        let count = games.len();
        
        self.summaries = games
            .iter()
            .enumerate()
            .map(|(i, g)| GameSummary::from_pgn_game(i, g))
            .collect();
        self.games = games;
        self.selected_index = None;
        
        count
    }

    /// Get the number of loaded games
    pub fn games_count(&self) -> usize {
        self.games.len()
    }

    /// Get the summaries for table display
    pub fn summaries(&self) -> &[GameSummary] {
        &self.summaries
    }

    /// Get a specific game by index
    pub fn get_game(&self, index: usize) -> Option<&PgnGame> {
        self.games.get(index)
    }

    /// Get the currently selected game index
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Set the selected game index
    pub fn set_selected(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    /// Clear all loaded games
    pub fn clear(&mut self) {
        self.games.clear();
        self.summaries.clear();
        self.selected_index = None;
    }
}

impl Default for PgnLibraryModel {
    fn default() -> Self {
        Self::new()
    }
}
