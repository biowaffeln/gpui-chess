//! UCI (Universal Chess Interface) protocol types and utilities.
//!
//! This module handles low-level UCI protocol communication with chess engines.
//! It provides types for UCI commands and responses, but does not handle
//! the actual process spawning (that's done in the models layer).

/// UCI commands that can be sent to an engine
#[derive(Debug, Clone)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum UciCommand {
    /// Initialize UCI mode
    Uci,
    /// Check if engine is ready
    IsReady,
    /// Set a new game
    UciNewGame,
    /// Set an engine option
    SetOption { name: String, value: String },
    /// Set position (startpos or FEN, with optional moves)
    Position { fen: Option<String>, moves: Vec<String> },
    /// Start infinite analysis
    GoInfinite,
    /// Start analysis with depth limit
    GoDepth(u32),
    /// Stop analysis
    Stop,
    /// Quit the engine
    Quit,
}

impl UciCommand {
    /// Convert command to UCI protocol string
    pub fn to_uci_string(&self) -> String {
        match self {
            UciCommand::Uci => "uci".to_string(),
            UciCommand::IsReady => "isready".to_string(),
            UciCommand::UciNewGame => "ucinewgame".to_string(),
            UciCommand::SetOption { name, value } => {
                format!("setoption name {} value {}", name, value)
            }
            UciCommand::Position { fen, moves } => {
                let mut cmd = String::from("position ");
                match fen {
                    Some(f) => {
                        cmd.push_str("fen ");
                        cmd.push_str(f);
                    }
                    None => cmd.push_str("startpos"),
                }
                if !moves.is_empty() {
                    cmd.push_str(" moves ");
                    cmd.push_str(&moves.join(" "));
                }
                cmd
            }
            UciCommand::GoInfinite => "go infinite".to_string(),
            UciCommand::GoDepth(d) => format!("go depth {}", d),
            UciCommand::Stop => "stop".to_string(),
            UciCommand::Quit => "quit".to_string(),
        }
    }
}

/// Raw UCI output line types (for Phase 1 - just categorization)
#[derive(Debug, Clone)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum UciOutputKind {
    /// "uciok" - engine is ready for UCI
    UciOk,
    /// "readyok" - engine is ready
    ReadyOk,
    /// "info ..." - analysis information
    Info(String),
    /// "bestmove ..." - best move found
    BestMove(String),
    /// Engine identification
    Id(String),
    /// Option definition
    Option(String),
    /// Unknown/other output
    Other(String),
}

impl UciOutputKind {
    /// Parse a raw UCI output line into a categorized type
    pub fn parse(line: &str) -> Self {
        let line = line.trim();
        
        if line == "uciok" {
            UciOutputKind::UciOk
        } else if line == "readyok" {
            UciOutputKind::ReadyOk
        } else if let Some(rest) = line.strip_prefix("info ") {
            UciOutputKind::Info(rest.to_string())
        } else if let Some(rest) = line.strip_prefix("bestmove ") {
            UciOutputKind::BestMove(rest.to_string())
        } else if let Some(rest) = line.strip_prefix("id ") {
            UciOutputKind::Id(rest.to_string())
        } else if let Some(rest) = line.strip_prefix("option ") {
            UciOutputKind::Option(rest.to_string())
        } else {
            UciOutputKind::Other(line.to_string())
        }
    }
}

/// A timestamped UCI output line (for display in the UI)
#[derive(Debug, Clone)]
pub struct UciOutput {
    /// The raw line from the engine
    pub raw: String,
    /// Parsed/categorized output
    pub kind: UciOutputKind,
}

impl UciOutput {
    pub fn new(line: String) -> Self {
        let kind = UciOutputKind::parse(&line);
        Self { raw: line, kind }
    }
}

/// Engine evaluation score
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Score {
    /// Centipawn score (positive = white advantage)
    Centipawns(i32),
    /// Mate in N moves (positive = white wins, negative = black wins)
    Mate(i32),
}

impl Score {
    /// Format score for display (e.g., "+0.35" or "M3" or "-M2")
    #[allow(dead_code)] // Used in tests, may be used in future
    pub fn display(&self) -> String {
        match self {
            Score::Centipawns(cp) => {
                let pawns = *cp as f64 / 100.0;
                if pawns >= 0.0 {
                    format!("+{:.2}", pawns)
                } else {
                    format!("{:.2}", pawns)
                }
            }
            Score::Mate(moves) => {
                if *moves > 0 {
                    format!("M{}", moves)
                } else {
                    format!("-M{}", moves.abs())
                }
            }
        }
    }

    /// Get a numeric value for comparison/display (centipawns, or large value for mate)
    #[allow(dead_code)] // Used in tests, may be used in future
    pub fn as_centipawns(&self) -> i32 {
        match self {
            Score::Centipawns(cp) => *cp,
            Score::Mate(moves) => {
                if *moves > 0 {
                    10000 - *moves // Mate in fewer moves is better
                } else {
                    -10000 - *moves // Being mated in fewer moves is worse
                }
            }
        }
    }
}

/// Parsed UCI info line containing analysis data
#[derive(Debug, Clone)]
pub struct UciInfo {
    /// Search depth
    pub depth: Option<u32>,
    /// Selective search depth
    pub seldepth: Option<u32>,
    /// Multi-PV line number (1-indexed)
    pub multipv: Option<u32>,
    /// Evaluation score
    pub score: Option<Score>,
    /// Nodes searched
    pub nodes: Option<u64>,
    /// Nodes per second
    pub nps: Option<u64>,
    /// Time spent in milliseconds
    pub time: Option<u64>,
    /// Principal variation (best line) as UCI moves
    pub pv: Vec<String>,
    /// Current move being searched
    pub currmove: Option<String>,
    /// Current move number
    pub currmovenumber: Option<u32>,
    /// Hash table usage (per mille)
    pub hashfull: Option<u32>,
}

impl UciInfo {
    /// Parse a UCI info string (the part after "info ")
    pub fn parse(info_str: &str) -> Self {
        let mut info = UciInfo {
            depth: None,
            seldepth: None,
            multipv: None,
            score: None,
            nodes: None,
            nps: None,
            time: None,
            pv: Vec::new(),
            currmove: None,
            currmovenumber: None,
            hashfull: None,
        };

        let tokens: Vec<&str> = info_str.split_whitespace().collect();
        let mut i = 0;

        while i < tokens.len() {
            match tokens[i] {
                "depth" => {
                    if i + 1 < tokens.len() {
                        info.depth = tokens[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "seldepth" => {
                    if i + 1 < tokens.len() {
                        info.seldepth = tokens[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "multipv" => {
                    if i + 1 < tokens.len() {
                        info.multipv = tokens[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "score" => {
                    // score cp <x> or score mate <x>
                    if i + 2 < tokens.len() {
                        match tokens[i + 1] {
                            "cp" => {
                                if let Ok(cp) = tokens[i + 2].parse::<i32>() {
                                    info.score = Some(Score::Centipawns(cp));
                                }
                                i += 3;
                            }
                            "mate" => {
                                if let Ok(m) = tokens[i + 2].parse::<i32>() {
                                    info.score = Some(Score::Mate(m));
                                }
                                i += 3;
                            }
                            _ => i += 1,
                        }
                    } else {
                        i += 1;
                    }
                }
                "nodes" => {
                    if i + 1 < tokens.len() {
                        info.nodes = tokens[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "nps" => {
                    if i + 1 < tokens.len() {
                        info.nps = tokens[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "time" => {
                    if i + 1 < tokens.len() {
                        info.time = tokens[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "hashfull" => {
                    if i + 1 < tokens.len() {
                        info.hashfull = tokens[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "currmove" => {
                    if i + 1 < tokens.len() {
                        info.currmove = Some(tokens[i + 1].to_string());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "currmovenumber" => {
                    if i + 1 < tokens.len() {
                        info.currmovenumber = tokens[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "pv" => {
                    // PV is the rest of the tokens
                    i += 1;
                    while i < tokens.len() {
                        // Stop if we hit another keyword
                        if matches!(
                            tokens[i],
                            "depth" | "seldepth" | "multipv" | "score" | "nodes" 
                            | "nps" | "time" | "hashfull" | "currmove" | "currmovenumber"
                            | "string" | "refutation" | "currline"
                        ) {
                            break;
                        }
                        info.pv.push(tokens[i].to_string());
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }

        info
    }

    /// Check if this info line has meaningful analysis data (depth + score + pv)
    pub fn has_analysis(&self) -> bool {
        self.depth.is_some() && self.score.is_some() && !self.pv.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_startpos() {
        let cmd = UciCommand::Position { fen: None, moves: vec![] };
        assert_eq!(cmd.to_uci_string(), "position startpos");
    }

    #[test]
    fn test_position_with_moves() {
        let cmd = UciCommand::Position {
            fen: None,
            moves: vec!["e2e4".to_string(), "e7e5".to_string()],
        };
        assert_eq!(cmd.to_uci_string(), "position startpos moves e2e4 e7e5");
    }

    #[test]
    fn test_position_fen() {
        let cmd = UciCommand::Position {
            fen: Some("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1".to_string()),
            moves: vec![],
        };
        assert_eq!(
            cmd.to_uci_string(),
            "position fen rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
        );
    }

    #[test]
    fn test_parse_info() {
        let output = UciOutput::new("info depth 20 score cp 35 pv e2e4 e7e5".to_string());
        assert!(matches!(output.kind, UciOutputKind::Info(_)));
    }

    #[test]
    fn test_parse_bestmove() {
        let output = UciOutput::new("bestmove e2e4 ponder e7e5".to_string());
        assert!(matches!(output.kind, UciOutputKind::BestMove(_)));
    }

    // UciInfo parsing tests
    #[test]
    fn test_parse_uci_info_basic() {
        let info = UciInfo::parse("depth 20 score cp 35 nodes 1234567 nps 2500000 pv e2e4 e7e5 g1f3");
        assert_eq!(info.depth, Some(20));
        assert_eq!(info.score, Some(Score::Centipawns(35)));
        assert_eq!(info.nodes, Some(1234567));
        assert_eq!(info.nps, Some(2500000));
        assert_eq!(info.pv, vec!["e2e4", "e7e5", "g1f3"]);
    }

    #[test]
    fn test_parse_uci_info_mate_score() {
        let info = UciInfo::parse("depth 15 score mate 3 pv e2e4 e7e5 d1h5");
        assert_eq!(info.depth, Some(15));
        assert_eq!(info.score, Some(Score::Mate(3)));
        assert_eq!(info.pv, vec!["e2e4", "e7e5", "d1h5"]);
    }

    #[test]
    fn test_parse_uci_info_negative_mate() {
        let info = UciInfo::parse("depth 12 score mate -2 pv g8f6");
        assert_eq!(info.score, Some(Score::Mate(-2)));
    }

    #[test]
    fn test_parse_uci_info_negative_cp() {
        let info = UciInfo::parse("depth 18 score cp -125 pv d7d5");
        assert_eq!(info.score, Some(Score::Centipawns(-125)));
    }

    #[test]
    fn test_parse_uci_info_multipv() {
        let info = UciInfo::parse("depth 20 multipv 2 score cp 15 pv d2d4");
        assert_eq!(info.multipv, Some(2));
        assert_eq!(info.depth, Some(20));
    }

    #[test]
    fn test_parse_uci_info_seldepth() {
        let info = UciInfo::parse("depth 20 seldepth 28 score cp 35 pv e2e4");
        assert_eq!(info.depth, Some(20));
        assert_eq!(info.seldepth, Some(28));
    }

    #[test]
    fn test_parse_uci_info_time() {
        let info = UciInfo::parse("depth 10 time 1500 nodes 500000 pv e2e4");
        assert_eq!(info.time, Some(1500));
        assert_eq!(info.nodes, Some(500000));
    }

    #[test]
    fn test_parse_uci_info_currmove() {
        let info = UciInfo::parse("depth 15 currmove e2e4 currmovenumber 1");
        assert_eq!(info.currmove, Some("e2e4".to_string()));
        assert_eq!(info.currmovenumber, Some(1));
    }

    #[test]
    fn test_parse_uci_info_hashfull() {
        let info = UciInfo::parse("depth 20 hashfull 500 pv e2e4");
        assert_eq!(info.hashfull, Some(500));
    }

    #[test]
    fn test_parse_uci_info_no_pv() {
        // Some info lines don't have a PV (e.g., currmove updates)
        let info = UciInfo::parse("depth 15 currmove g1f3 currmovenumber 5");
        assert!(info.pv.is_empty());
        assert!(!info.has_analysis());
    }

    #[test]
    fn test_has_analysis() {
        let info = UciInfo::parse("depth 20 score cp 35 pv e2e4");
        assert!(info.has_analysis());

        let info_no_score = UciInfo::parse("depth 20 pv e2e4");
        assert!(!info_no_score.has_analysis());

        let info_no_pv = UciInfo::parse("depth 20 score cp 35");
        assert!(!info_no_pv.has_analysis());
    }

    // Score display tests
    #[test]
    fn test_score_display_positive_cp() {
        let score = Score::Centipawns(35);
        assert_eq!(score.display(), "+0.35");
    }

    #[test]
    fn test_score_display_negative_cp() {
        let score = Score::Centipawns(-125);
        assert_eq!(score.display(), "-1.25");
    }

    #[test]
    fn test_score_display_zero() {
        let score = Score::Centipawns(0);
        assert_eq!(score.display(), "+0.00");
    }

    #[test]
    fn test_score_display_mate_white() {
        let score = Score::Mate(3);
        assert_eq!(score.display(), "M3");
    }

    #[test]
    fn test_score_display_mate_black() {
        let score = Score::Mate(-2);
        assert_eq!(score.display(), "-M2");
    }

    #[test]
    fn test_score_as_centipawns() {
        assert_eq!(Score::Centipawns(100).as_centipawns(), 100);
        assert_eq!(Score::Centipawns(-50).as_centipawns(), -50);
        // Mate in 1 is better than mate in 3
        assert!(Score::Mate(1).as_centipawns() > Score::Mate(3).as_centipawns());
        // Being mated in 1 is worse than being mated in 3
        assert!(Score::Mate(-1).as_centipawns() < Score::Mate(-3).as_centipawns());
    }

    #[test]
    fn test_parse_stockfish_real_output() {
        // Real Stockfish output example
        let info = UciInfo::parse(
            "depth 24 seldepth 31 multipv 1 score cp 28 nodes 2847613 nps 2431482 hashfull 457 time 1171 pv e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6 e1g1"
        );
        assert_eq!(info.depth, Some(24));
        assert_eq!(info.seldepth, Some(31));
        assert_eq!(info.multipv, Some(1));
        assert_eq!(info.score, Some(Score::Centipawns(28)));
        assert_eq!(info.nodes, Some(2847613));
        assert_eq!(info.nps, Some(2431482));
        assert_eq!(info.hashfull, Some(457));
        assert_eq!(info.time, Some(1171));
        assert_eq!(info.pv.len(), 9);
        assert_eq!(info.pv[0], "e2e4");
        assert!(info.has_analysis());
    }
}
