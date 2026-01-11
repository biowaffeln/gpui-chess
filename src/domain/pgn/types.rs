//! PGN data types.

use crate::domain::MoveTree;
use std::collections::HashMap;

/// Standard PGN header tags
#[derive(Debug, Clone, Default)]
pub struct PgnHeaders {
    /// All headers as key-value pairs
    pub tags: HashMap<String, String>,
}

impl PgnHeaders {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the Event header
    pub fn event(&self) -> Option<&str> {
        self.tags.get("Event").map(|s| s.as_str())
    }

    /// Get the Site header
    pub fn site(&self) -> Option<&str> {
        self.tags.get("Site").map(|s| s.as_str())
    }

    /// Get the Date header
    pub fn date(&self) -> Option<&str> {
        self.tags.get("Date").map(|s| s.as_str())
    }

    /// Get the Round header
    pub fn round(&self) -> Option<&str> {
        self.tags.get("Round").map(|s| s.as_str())
    }

    /// Get the White player name
    pub fn white(&self) -> Option<&str> {
        self.tags.get("White").map(|s| s.as_str())
    }

    /// Get the Black player name
    pub fn black(&self) -> Option<&str> {
        self.tags.get("Black").map(|s| s.as_str())
    }

    /// Get the Result header (e.g., "1-0", "0-1", "1/2-1/2", "*")
    pub fn result(&self) -> Option<&str> {
        self.tags.get("Result").map(|s| s.as_str())
    }

    /// Get the WhiteElo header
    pub fn white_elo(&self) -> Option<u16> {
        self.tags.get("WhiteElo").and_then(|s| s.parse().ok())
    }

    /// Get the BlackElo header
    pub fn black_elo(&self) -> Option<u16> {
        self.tags.get("BlackElo").and_then(|s| s.parse().ok())
    }

    /// Get the ECO code
    pub fn eco(&self) -> Option<&str> {
        self.tags.get("ECO").map(|s| s.as_str())
    }

    /// Get the FEN for the starting position (if not standard)
    pub fn fen(&self) -> Option<&str> {
        self.tags.get("FEN").map(|s| s.as_str())
    }
}

/// A parsed PGN game with headers and move tree
#[derive(Debug, Clone)]
pub struct PgnGame {
    /// Game headers (Event, White, Black, etc.)
    pub headers: PgnHeaders,
    /// The move tree containing all moves and variations
    pub moves: MoveTree,
}

impl PgnGame {
    pub fn new(headers: PgnHeaders, moves: MoveTree) -> Self {
        Self { headers, moves }
    }
}
